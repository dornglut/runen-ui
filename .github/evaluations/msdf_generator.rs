use std::{error::Error, path::PathBuf, time::Instant};

use bymsdfgen_core::{
    Bitmap, DistanceMapping, EdgeSegment, GeneratorConfig, MsdfGeneratorConfig, Projection, Range,
    SdfTransformation, Shape, Vector2, coloring::edge_coloring_simple, generate_msdf, generate_sdf,
};
use runenui_core::{FontFamily, LogicalLength, Typography};
use runenui_text::{
    FontSourcePolicy, ShapedTextResource, TextArtifact, TextConstraints, TextLanguage,
    TextLayoutState, TextLine, TextParagraphStyle, TextRequest, TextSystem,
};
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, NormalizedCoord, Size},
    outline::{DrawSettings, OutlinePen},
};

const CANTARELL_PATH: &str = "crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf";
const ARABIC_PATH: &str =
    "crates/runenui_text/tests/fixtures/RunenUIFixtureArabic-Regular.ttf";
const DEVANAGARI_PATH: &str =
    "crates/runenui_text/tests/fixtures/RunenUIFixtureDevanagari-Regular.ttf";
const TIERS: [f64; 4] = [16.0, 24.0, 32.0, 48.0];
const FIELD_RANGE_PX: f64 = 4.0;
const REPEATS: usize = 3;

#[derive(Clone, Copy)]
struct CorpusCase {
    label: &'static str,
    family: &'static str,
    language: &'static str,
    text: &'static str,
    font_path: &'static str,
}

const CORPUS: [CorpusCase; 3] = [
    CorpusCase {
        label: "latin",
        family: "Cantarell",
        language: "en",
        text: "AVO8",
        font_path: CANTARELL_PATH,
    },
    CorpusCase {
        label: "arabic",
        family: "RunenUI Fixture Arabic",
        language: "ar",
        text: "سلام",
        font_path: ARABIC_PATH,
    },
    CorpusCase {
        label: "devanagari",
        family: "RunenUI Fixture Devanagari",
        language: "hi-Deva",
        text: "क्षि",
        font_path: DEVANAGARI_PATH,
    },
];

#[derive(Default)]
struct ShapePen {
    shape: Shape,
    position: Vector2,
    start: Vector2,
    contour_open: bool,
}

impl ShapePen {
    fn point(x: f32, y: f32) -> Vector2 {
        Vector2::new(f64::from(x), f64::from(y))
    }

    fn current_contour(&mut self) -> &mut bymsdfgen_core::Contour {
        self.shape
            .contours
            .last_mut()
            .expect("Skrifa emitted an edge before move_to")
    }

    fn finish(mut self) -> Shape {
        if self.contour_open {
            self.close();
        }
        self.shape
            .contours
            .retain(|contour| !contour.segments.is_empty());
        self.shape
    }
}

impl OutlinePen for ShapePen {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.contour_open {
            self.close();
        }
        self.shape.contours.push(bymsdfgen_core::Contour::new());
        self.position = Self::point(x, y);
        self.start = self.position;
        self.contour_open = true;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let endpoint = Self::point(x, y);
        if endpoint != self.position {
            let start = self.position;
            self.current_contour()
                .add_edge(EdgeSegment::line(start, endpoint));
            self.position = endpoint;
        }
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let control = Self::point(cx0, cy0);
        let endpoint = Self::point(x, y);
        let start = self.position;
        if control != start || endpoint != start {
            self.current_contour()
                .add_edge(EdgeSegment::quadratic(start, control, endpoint));
        }
        self.position = endpoint;
    }

    fn curve_to(
        &mut self,
        cx0: f32,
        cy0: f32,
        cx1: f32,
        cy1: f32,
        x: f32,
        y: f32,
    ) {
        let control0 = Self::point(cx0, cy0);
        let control1 = Self::point(cx1, cy1);
        let endpoint = Self::point(x, y);
        let start = self.position;
        if control0 != start || control1 != start || endpoint != start {
            self.current_contour().add_edge(EdgeSegment::cubic(
                start, control0, control1, endpoint,
            ));
        }
        self.position = endpoint;
    }

    fn close(&mut self) {
        if self.contour_open && self.position != self.start {
            let position = self.position;
            let start = self.start;
            self.current_contour()
                .add_edge(EdgeSegment::line(position, start));
            self.position = start;
        }
        self.contour_open = false;
    }
}

struct FieldEvidence {
    width: usize,
    height: usize,
    hash: u64,
    elapsed_micros: u128,
    mean_abs_error: f64,
    max_abs_error: f64,
    sign_mismatches: usize,
    quantized_mean_abs_error: f64,
    quantized_max_abs_error: f64,
    quantized_sign_mismatches: usize,
}

fn typography(family: &str) -> Result<Typography, Box<dyn Error>> {
    Ok(Typography::new(
        FontFamily::named(family)?,
        LogicalLength::new(20.0)?,
    ))
}

fn repository_root() -> PathBuf {
    std::env::var_os("RUNENUI_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn shape_case(case: CorpusCase) -> Result<(TextArtifact, Vec<u8>), Box<dyn Error>> {
    let font_bytes = std::fs::read(repository_root().join(case.font_path))?;
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    if system.register_font_bytes(font_bytes.clone())? == 0 {
        return Err(format!("{} fixture registered no fonts", case.label).into());
    }
    let request = TextRequest::new(
        case.text,
        typography(case.family)?,
        TextConstraints::unbounded(),
    )
    .with_paragraph_style(
        TextParagraphStyle::default().with_language(TextLanguage::new(case.language)?),
    );
    let mut state = TextLayoutState::new();
    Ok((
        system.layout_text(&mut state, &request)?.into_artifact(),
        font_bytes,
    ))
}

fn to_shape(resource: &ShapedTextResource, glyph_id: u32) -> Result<Shape, Box<dyn Error>> {
    let binding = resource.font();
    if binding.faux_bold() || binding.faux_skew().is_some() {
        return Err("controlled evaluation unexpectedly requires synthetic font transforms".into());
    }

    let font = FontRef::from_index(binding.bytes(), binding.face_index())?;
    let coords = binding
        .normalized_coords()
        .iter()
        .copied()
        .map(NormalizedCoord::from_bits)
        .collect::<Vec<_>>();
    let location = LocationRef::new(&coords);
    let outline = font
        .outline_glyphs()
        .get(GlyphId::new(glyph_id))
        .ok_or_else(|| format!("glyph {glyph_id} has no supported outline"))?;
    let mut pen = ShapePen::default();
    outline.draw(
        DrawSettings::unhinted(Size::new(1.0), location),
        &mut pen,
    )?;
    let mut shape = pen.finish();
    if shape.edge_count() == 0 {
        return Err(format!("glyph {glyph_id} produced an empty outline").into());
    }
    if !shape.validate() {
        return Err(format!("glyph {glyph_id} produced an invalid closed outline").into());
    }
    shape.normalize();
    shape.orient_contours();
    edge_coloring_simple(&mut shape, 3.0, 0);
    Ok(shape)
}

fn median3(a: f32, b: f32, c: f32) -> f64 {
    f64::from(a.max(b.min(c)).min(b.max(c)))
}

fn quantize(value: f32) -> f64 {
    f64::from((value.clamp(0.0, 1.0) * 255.0).round()) / 255.0
}

fn hash_field(field: &Bitmap<f32, 3>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in field.data() {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn evaluate_shape(shape: &Shape, tier: f64) -> Result<FieldEvidence, Box<dyn Error>> {
    let range = FIELD_RANGE_PX / tier;
    let bounds = shape.get_bounds(range);
    let width = ((bounds.r - bounds.l) * tier).ceil().max(1.0) as usize;
    let height = ((bounds.t - bounds.b) * tier).ceil().max(1.0) as usize;
    let transform = SdfTransformation::new(
        Projection::new(
            Vector2::splat(tier),
            Vector2::new(-bounds.l, -bounds.b),
        ),
        DistanceMapping::from_range(Range::symmetric(range)),
    );

    let mut sdf = Bitmap::<f32, 1>::new(width, height);
    generate_sdf(&mut sdf, shape, &transform, &GeneratorConfig::default());

    let mut first = Bitmap::<f32, 3>::new(width, height);
    let start = Instant::now();
    for repeat in 0..REPEATS {
        if repeat == 0 {
            generate_msdf(
                &mut first,
                shape,
                &transform,
                &MsdfGeneratorConfig::default(),
            );
        } else {
            let mut repeated = Bitmap::<f32, 3>::new(width, height);
            generate_msdf(
                &mut repeated,
                shape,
                &transform,
                &MsdfGeneratorConfig::default(),
            );
            if hash_field(&repeated) != hash_field(&first) {
                return Err("same outline/configuration produced a nondeterministic MSDF hash".into());
            }
        }
    }
    let elapsed_micros = start.elapsed().as_micros() / REPEATS as u128;
    let hash = hash_field(&first);

    let mut abs_error = 0.0;
    let mut max_abs_error = 0.0_f64;
    let mut sign_mismatches = 0;
    let mut quantized_abs_error = 0.0;
    let mut quantized_max_abs_error = 0.0_f64;
    let mut quantized_sign_mismatches = 0;

    for index in 0..width * height {
        let sdf_value = f64::from(sdf.data()[index]);
        if !sdf_value.is_finite() {
            return Err("SDF contained a non-finite value".into());
        }
        let base = index * 3;
        let r = first.data()[base];
        let g = first.data()[base + 1];
        let b = first.data()[base + 2];
        if !r.is_finite() || !g.is_finite() || !b.is_finite() {
            return Err("MSDF contained a non-finite value".into());
        }

        let reconstructed = median3(r, g, b);
        let error = (reconstructed - sdf_value).abs();
        abs_error += error;
        max_abs_error = max_abs_error.max(error);
        sign_mismatches += usize::from((reconstructed >= 0.5) != (sdf_value >= 0.5));

        let quantized = median3(
            quantize(r) as f32,
            quantize(g) as f32,
            quantize(b) as f32,
        );
        let quantized_error = (quantized - sdf_value).abs();
        quantized_abs_error += quantized_error;
        quantized_max_abs_error = quantized_max_abs_error.max(quantized_error);
        quantized_sign_mismatches +=
            usize::from((quantized >= 0.5) != (sdf_value >= 0.5));
    }

    let pixels = (width * height) as f64;
    Ok(FieldEvidence {
        width,
        height,
        hash,
        elapsed_micros,
        mean_abs_error: abs_error / pixels,
        max_abs_error,
        sign_mismatches,
        quantized_mean_abs_error: quantized_abs_error / pixels,
        quantized_max_abs_error,
        quantized_sign_mismatches,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("M8B MSDF generator evaluation");
    println!("candidate=bymsdfgen-core@0.1.1 parallel=false edge_coloring=simple seed=0");
    println!("field_range_px={FIELD_RANGE_PX} repeats={REPEATS}");

    let mut glyph_count = 0usize;
    let mut total_edges = 0usize;
    for case in CORPUS {
        let (artifact, font_bytes) = shape_case(case)?;
        for run in artifact.lines().iter().flat_map(TextLine::runs) {
            let resource = run.shaped_resource();
            if resource.font().bytes() != font_bytes.as_slice() {
                return Err(format!("{} shaped with an unexpected font source", case.label).into());
            }
            for glyph in resource.glyphs() {
                let shape = to_shape(resource, glyph.id())?;
                glyph_count += 1;
                total_edges += shape.edge_count();
                for tier in TIERS {
                    let evidence = evaluate_shape(&shape, tier)?;
                    let pixels = evidence.width * evidence.height;
                    println!(
                        "case={} glyph={} tier={} extent={}x{} edges={} hash={:016x} avg_us={} rgb_f32_bytes={} rgb8_bytes={} rgba8_wgpu_bytes={} mean_abs_error={:.8} max_abs_error={:.8} sign_mismatch={}/{} quantized_mean_abs_error={:.8} quantized_max_abs_error={:.8} quantized_sign_mismatch={}/{}",
                        case.label,
                        glyph.id(),
                        tier as u32,
                        evidence.width,
                        evidence.height,
                        shape.edge_count(),
                        evidence.hash,
                        evidence.elapsed_micros,
                        pixels * 3 * std::mem::size_of::<f32>(),
                        pixels * 3,
                        pixels * 4,
                        evidence.mean_abs_error,
                        evidence.max_abs_error,
                        evidence.sign_mismatches,
                        pixels,
                        evidence.quantized_mean_abs_error,
                        evidence.quantized_max_abs_error,
                        evidence.quantized_sign_mismatches,
                        pixels,
                    );
                }
            }
        }
    }

    if glyph_count == 0 {
        return Err("controlled corpus produced no outline glyphs".into());
    }
    println!(
        "summary glyphs={glyph_count} total_edges={total_edges} tiers={}",
        TIERS.len()
    );
    Ok(())
}
