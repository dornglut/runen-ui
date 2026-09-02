use std::{error::Error, path::PathBuf, time::Instant};

use bymsdfgen_core::{
    Bitmap, DistanceMapping, EdgeSegment as BySegment, GeneratorConfig, MsdfGeneratorConfig,
    Projection, Range, SdfTransformation, Shape as ByShape, Vector2 as ByPoint,
    coloring::edge_coloring_simple as by_edge_coloring_simple,
    generate_msdf as by_generate_msdf, generate_sdf as by_generate_sdf,
};
use fdsm::{
    bezier::{Point as FdPoint, Segment as FdSegment, scanline::FillRule},
    correct_error::{ErrorCorrectionConfig as FdErrorCorrectionConfig, correct_error_msdf},
    generate::{generate_msdf as fd_generate_msdf, generate_sdf as fd_generate_sdf},
    render::{correct_sign_msdf, correct_sign_sdf},
    shape::{Contour as FdContour, Shape as FdShape},
};
use image::{ImageBuffer, Luma, Rgb};
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
const FDSM_DISTANCE_RANGE_PX: f64 = FIELD_RANGE_PX * 2.0;
const REPEATS: usize = 3;
const EDGE_ANGLE_THRESHOLD_RADIANS: f64 = 3.0;
const EDGE_COLORING_SEED: u64 = 0;

#[derive(Clone, Copy)]
struct CorpusCase {
    label: &'static str,
    family: &'static str,
    language: &'static str,
    text: &'static str,
    font_path: &'static str,
}

const CORPUS: [CorpusCase; 4] = [
    CorpusCase {
        label: "latin-acute",
        family: "Cantarell",
        language: "en",
        text: "AV",
        font_path: CANTARELL_PATH,
    },
    CorpusCase {
        label: "latin-curves",
        family: "Cantarell",
        language: "en",
        text: "O8",
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f32, y: f32) -> Self {
        Self {
            x: f64::from(x),
            y: f64::from(y),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Segment {
    Line(Point, Point),
    Quadratic(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

#[derive(Clone, Debug, Default)]
struct Contour {
    segments: Vec<Segment>,
}

#[derive(Clone, Debug, Default)]
struct OutlineGeometry {
    contours: Vec<Contour>,
    position: Option<Point>,
    start: Option<Point>,
}

impl OutlineGeometry {
    fn current_contour(&mut self) -> &mut Contour {
        self.contours
            .last_mut()
            .expect("Skrifa emitted an edge before move_to")
    }

    fn finish(mut self) -> Self {
        if self.position.is_some() {
            self.close();
        }
        self.contours.retain(|contour| !contour.segments.is_empty());
        self
    }

    fn segment_count(&self) -> usize {
        self.contours
            .iter()
            .map(|contour| contour.segments.len())
            .sum()
    }
}

impl OutlinePen for OutlineGeometry {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.position.is_some() {
            self.close();
        }
        let point = Point::new(x, y);
        self.contours.push(Contour::default());
        self.position = Some(point);
        self.start = Some(point);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let endpoint = Point::new(x, y);
        let start = self.position.expect("line_to before move_to");
        if endpoint != start {
            self.current_contour()
                .segments
                .push(Segment::Line(start, endpoint));
        }
        self.position = Some(endpoint);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let control = Point::new(cx0, cy0);
        let endpoint = Point::new(x, y);
        let start = self.position.expect("quad_to before move_to");
        if control != start || endpoint != start {
            self.current_contour()
                .segments
                .push(Segment::Quadratic(start, control, endpoint));
        }
        self.position = Some(endpoint);
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
        let control0 = Point::new(cx0, cy0);
        let control1 = Point::new(cx1, cy1);
        let endpoint = Point::new(x, y);
        let start = self.position.expect("curve_to before move_to");
        if control0 != start || control1 != start || endpoint != start {
            self.current_contour()
                .segments
                .push(Segment::Cubic(start, control0, control1, endpoint));
        }
        self.position = Some(endpoint);
    }

    fn close(&mut self) {
        if let (Some(position), Some(start)) = (self.position, self.start) {
            if position != start {
                self.current_contour()
                    .segments
                    .push(Segment::Line(position, start));
            }
        }
        self.position = None;
        self.start = None;
    }
}

struct Domain {
    shape: ByShape,
    width: usize,
    height: usize,
    bounds: bymsdfgen_core::Bounds,
    transform: SdfTransformation,
    reference_sdf: Vec<f32>,
}

struct FieldEvidence {
    hash: u64,
    elapsed_micros: u128,
    mean_abs_error: f64,
    max_abs_error: f64,
    sign_mismatches: usize,
    quantized_mean_abs_error: f64,
    quantized_max_abs_error: f64,
    quantized_boundary_mean_abs_error: f64,
    quantized_boundary_max_abs_error: f64,
    boundary_pixels: usize,
    quantized_sign_mismatches: usize,
}

struct CrossSdfEvidence {
    mean_abs_error: f64,
    max_abs_error: f64,
    sign_mismatches: usize,
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

fn outline_geometry(
    resource: &ShapedTextResource,
    glyph_id: u32,
) -> Result<OutlineGeometry, Box<dyn Error>> {
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
    let mut geometry = OutlineGeometry::default();
    outline.draw(
        DrawSettings::unhinted(Size::new(1.0), location),
        &mut geometry,
    )?;
    let geometry = geometry.finish();
    if geometry.segment_count() == 0 {
        return Err(format!("glyph {glyph_id} produced an empty outline").into());
    }
    Ok(geometry)
}

fn by_point(point: Point) -> ByPoint {
    ByPoint::new(point.x, point.y)
}

fn by_shape(geometry: &OutlineGeometry) -> Result<ByShape, Box<dyn Error>> {
    let mut shape = ByShape::new();
    for contour in &geometry.contours {
        let target = shape.add_contour_mut();
        for segment in &contour.segments {
            match *segment {
                Segment::Line(start, end) => {
                    target.add_edge(BySegment::line(by_point(start), by_point(end)));
                }
                Segment::Quadratic(start, control, end) => {
                    target.add_edge(BySegment::quadratic(
                        by_point(start),
                        by_point(control),
                        by_point(end),
                    ));
                }
                Segment::Cubic(start, control0, control1, end) => {
                    target.add_edge(BySegment::cubic(
                        by_point(start),
                        by_point(control0),
                        by_point(control1),
                        by_point(end),
                    ));
                }
            }
        }
    }
    if !shape.validate() {
        return Err("Skrifa outline converted into an invalid bymsdfgen shape".into());
    }
    shape.normalize();
    shape.orient_contours();
    by_edge_coloring_simple(
        &mut shape,
        EDGE_ANGLE_THRESHOLD_RADIANS,
        EDGE_COLORING_SEED,
    );
    Ok(shape)
}

fn domain(geometry: &OutlineGeometry, tier: f64) -> Result<Domain, Box<dyn Error>> {
    let shape = by_shape(geometry)?;
    let range = FIELD_RANGE_PX / tier;
    let bounds = shape.get_bounds(range);
    let width = ((bounds.r - bounds.l) * tier).ceil().max(1.0) as usize;
    let height = ((bounds.t - bounds.b) * tier).ceil().max(1.0) as usize;
    let transform = SdfTransformation::new(
        Projection::new(
            ByPoint::splat(tier),
            ByPoint::new(-bounds.l, -bounds.b),
        ),
        DistanceMapping::from_range(Range::symmetric(range)),
    );
    let mut sdf = Bitmap::<f32, 1>::new(width, height);
    by_generate_sdf(&mut sdf, &shape, &transform, &GeneratorConfig::default());
    let reference_sdf = sdf.data().to_vec();
    Ok(Domain {
        shape,
        width,
        height,
        bounds,
        transform,
        reference_sdf,
    })
}

fn fd_point(point: Point, bounds: bymsdfgen_core::Bounds, tier: f64) -> FdPoint {
    FdPoint::new(
        (point.x - bounds.l) * tier,
        (point.y - bounds.b) * tier,
    )
}

fn fd_shape(
    geometry: &OutlineGeometry,
    bounds: bymsdfgen_core::Bounds,
    tier: f64,
) -> FdShape<FdContour> {
    let contours = geometry
        .contours
        .iter()
        .map(|contour| FdContour {
            segments: contour
                .segments
                .iter()
                .map(|segment| match *segment {
                    Segment::Line(start, end) => {
                        FdSegment::line(fd_point(start, bounds, tier), fd_point(end, bounds, tier))
                    }
                    Segment::Quadratic(start, control, end) => FdSegment::quad(
                        fd_point(start, bounds, tier),
                        fd_point(control, bounds, tier),
                        fd_point(end, bounds, tier),
                    ),
                    Segment::Cubic(start, control0, control1, end) => FdSegment::cubic(
                        fd_point(start, bounds, tier),
                        fd_point(control0, bounds, tier),
                        fd_point(control1, bounds, tier),
                        fd_point(end, bounds, tier),
                    ),
                })
                .collect(),
        })
        .collect();
    FdShape { contours }
}

fn median3(a: f32, b: f32, c: f32) -> f64 {
    f64::from(a.max(b.min(c)).min(b.max(c)))
}

fn quantize(value: f32) -> f64 {
    f64::from((value.clamp(0.0, 1.0) * 255.0).round()) / 255.0
}

fn hash_f32(values: &[f32]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn field_evidence(
    reference_sdf: &[f32],
    field: &[f32],
    elapsed_micros: u128,
) -> Result<FieldEvidence, Box<dyn Error>> {
    if field.len() != reference_sdf.len() * 3 {
        return Err("candidate field dimensions do not match the shared reference".into());
    }

    let mut abs_error = 0.0;
    let mut max_abs_error = 0.0_f64;
    let mut sign_mismatches = 0usize;
    let mut quantized_abs_error = 0.0;
    let mut quantized_max_abs_error = 0.0_f64;
    let mut quantized_boundary_abs_error = 0.0;
    let mut quantized_boundary_max_abs_error = 0.0_f64;
    let mut boundary_pixels = 0usize;
    let mut quantized_sign_mismatches = 0usize;

    for (index, reference) in reference_sdf.iter().enumerate() {
        let sdf_value = f64::from(*reference);
        if !sdf_value.is_finite() {
            return Err("reference SDF contained a non-finite value".into());
        }
        let base = index * 3;
        let r = field[base];
        let g = field[base + 1];
        let b = field[base + 2];
        if !r.is_finite() || !g.is_finite() || !b.is_finite() {
            return Err("candidate MSDF contained a non-finite value".into());
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
        let clamped_sdf = sdf_value.clamp(0.0, 1.0);
        let quantized_error = (quantized - clamped_sdf).abs();
        quantized_abs_error += quantized_error;
        quantized_max_abs_error = quantized_max_abs_error.max(quantized_error);
        if (0.0..=1.0).contains(&sdf_value) {
            boundary_pixels += 1;
            quantized_boundary_abs_error += quantized_error;
            quantized_boundary_max_abs_error =
                quantized_boundary_max_abs_error.max(quantized_error);
        }
        quantized_sign_mismatches +=
            usize::from((quantized >= 0.5) != (sdf_value >= 0.5));
    }

    let pixels = reference_sdf.len() as f64;
    Ok(FieldEvidence {
        hash: hash_f32(field),
        elapsed_micros,
        mean_abs_error: abs_error / pixels,
        max_abs_error,
        sign_mismatches,
        quantized_mean_abs_error: quantized_abs_error / pixels,
        quantized_max_abs_error,
        quantized_boundary_mean_abs_error: if boundary_pixels == 0 {
            0.0
        } else {
            quantized_boundary_abs_error / boundary_pixels as f64
        },
        quantized_boundary_max_abs_error,
        boundary_pixels,
        quantized_sign_mismatches,
    })
}

fn cross_sdf_evidence(
    reference: &[f32],
    candidate: &[f32],
) -> Result<CrossSdfEvidence, Box<dyn Error>> {
    if reference.len() != candidate.len() {
        return Err("candidate SDF dimensions do not match the shared reference".into());
    }
    let mut abs_error = 0.0;
    let mut max_abs_error = 0.0_f64;
    let mut sign_mismatches = 0usize;
    for (reference_value, candidate_value) in reference.iter().zip(candidate) {
        let a = f64::from(*reference_value);
        let b = f64::from(*candidate_value);
        if !a.is_finite() || !b.is_finite() {
            return Err("cross-candidate SDF comparison contained a non-finite value".into());
        }
        let error = (a.clamp(0.0, 1.0) - b.clamp(0.0, 1.0)).abs();
        abs_error += error;
        max_abs_error = max_abs_error.max(error);
        sign_mismatches += usize::from((a >= 0.5) != (b >= 0.5));
    }
    Ok(CrossSdfEvidence {
        mean_abs_error: abs_error / reference.len() as f64,
        max_abs_error,
        sign_mismatches,
    })
}

fn evaluate_bymsdfgen(domain: &Domain) -> Result<FieldEvidence, Box<dyn Error>> {
    let mut fields = (0..REPEATS)
        .map(|_| Bitmap::<f32, 3>::new(domain.width, domain.height))
        .collect::<Vec<_>>();
    let start = Instant::now();
    for field in &mut fields {
        by_generate_msdf(
            field,
            &domain.shape,
            &domain.transform,
            &MsdfGeneratorConfig::default(),
        );
    }
    let elapsed_micros = start.elapsed().as_micros() / REPEATS as u128;
    let first_hash = hash_f32(fields[0].data());
    if fields
        .iter()
        .skip(1)
        .any(|field| hash_f32(field.data()) != first_hash)
    {
        return Err("bymsdfgen produced nondeterministic output for one outline/configuration".into());
    }
    field_evidence(&domain.reference_sdf, fields[0].data(), elapsed_micros)
}

type FdSdf = ImageBuffer<Luma<f32>, Vec<f32>>;
type FdMsdf = ImageBuffer<Rgb<f32>, Vec<f32>>;

fn evaluate_fdsm(
    geometry: &OutlineGeometry,
    domain: &Domain,
    tier: f64,
) -> Result<(FieldEvidence, CrossSdfEvidence), Box<dyn Error>> {
    let shape = fd_shape(geometry, domain.bounds, tier);
    let prepared_shape = shape.prepare();
    let colored = FdShape::edge_coloring_simple(
        shape.clone(),
        EDGE_ANGLE_THRESHOLD_RADIANS.sin(),
        EDGE_COLORING_SEED,
    );
    let prepared_colored = colored.prepare();

    let mut sdf = FdSdf::new(domain.width as u32, domain.height as u32);
    fd_generate_sdf(&prepared_shape, FDSM_DISTANCE_RANGE_PX, &mut sdf);
    correct_sign_sdf(&mut sdf, &prepared_shape, FillRule::Nonzero);
    let cross = cross_sdf_evidence(&domain.reference_sdf, sdf.as_raw())?;

    let correction = FdErrorCorrectionConfig::default();
    let mut fields = (0..REPEATS)
        .map(|_| FdMsdf::new(domain.width as u32, domain.height as u32))
        .collect::<Vec<_>>();
    let start = Instant::now();
    for field in &mut fields {
        fd_generate_msdf(&prepared_colored, FDSM_DISTANCE_RANGE_PX, field);
        correct_error_msdf(
            field,
            &colored,
            &prepared_colored,
            FDSM_DISTANCE_RANGE_PX,
            &correction,
        );
        correct_sign_msdf(field, &prepared_colored, FillRule::Nonzero);
    }
    let elapsed_micros = start.elapsed().as_micros() / REPEATS as u128;
    let first_hash = hash_f32(fields[0].as_raw());
    if fields
        .iter()
        .skip(1)
        .any(|field| hash_f32(field.as_raw()) != first_hash)
    {
        return Err("fdsm produced nondeterministic output for one outline/configuration".into());
    }
    let evidence = field_evidence(sdf.as_raw(), fields[0].as_raw(), elapsed_micros)?;
    Ok((evidence, cross))
}

fn print_evidence(
    candidate: &str,
    case: CorpusCase,
    glyph_id: u32,
    tier: f64,
    segments: usize,
    width: usize,
    height: usize,
    evidence: &FieldEvidence,
    cross: Option<&CrossSdfEvidence>,
) {
    let pixels = width * height;
    print!(
        "candidate={candidate} case={} glyph={} tier={} extent={}x{} segments={} hash={:016x} avg_us={} rgb_f32_bytes={} rgb8_bytes={} rgba8_wgpu_bytes={} mean_abs_error={:.8} max_abs_error={:.8} sign_mismatch={}/{} quantized_mean_abs_error={:.8} quantized_max_abs_error={:.8} quantized_boundary_mean_abs_error={:.8} quantized_boundary_max_abs_error={:.8} boundary_pixels={} quantized_sign_mismatch={}/{}",
        case.label,
        glyph_id,
        tier as u32,
        width,
        height,
        segments,
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
        evidence.quantized_boundary_mean_abs_error,
        evidence.quantized_boundary_max_abs_error,
        evidence.boundary_pixels,
        evidence.quantized_sign_mismatches,
        pixels,
    );
    if let Some(cross) = cross {
        print!(
            " shared_sdf_mean_abs_error={:.8} shared_sdf_max_abs_error={:.8} shared_sdf_sign_mismatch={}/{}",
            cross.mean_abs_error,
            cross.max_abs_error,
            cross.sign_mismatches,
            pixels,
        );
    }
    println!();
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("M8B shared-corpus MSDF generator evaluation");
    println!(
        "candidates=bymsdfgen-core@0.1.1,fdsm@0.8.0 tiers=16,24,32,48 field_range_px={FIELD_RANGE_PX} repeats={REPEATS} edge_angle_rad={EDGE_ANGLE_THRESHOLD_RADIANS} seed={EDGE_COLORING_SEED}"
    );
    println!(
        "timing=field_generation_with_candidate_default_error/sign_correction; outline extraction/coloring/preparation excluded"
    );

    let mut glyph_count = 0usize;
    let mut segment_count = 0usize;

    for case in CORPUS {
        let (artifact, font_bytes) = shape_case(case)?;
        for run in artifact.lines().iter().flat_map(TextLine::runs) {
            let resource = run.shaped_resource();
            if resource.font().bytes() != font_bytes.as_slice() {
                return Err(format!("{} shaped with an unexpected font source", case.label).into());
            }
            for glyph in resource.glyphs() {
                let geometry = outline_geometry(resource, glyph.id())?;
                glyph_count += 1;
                segment_count += geometry.segment_count();

                for tier in TIERS {
                    let domain = domain(&geometry, tier)?;
                    let by = evaluate_bymsdfgen(&domain)?;
                    print_evidence(
                        "bymsdfgen-core@0.1.1",
                        case,
                        glyph.id(),
                        tier,
                        geometry.segment_count(),
                        domain.width,
                        domain.height,
                        &by,
                        None,
                    );

                    let (fd, cross) = evaluate_fdsm(&geometry, &domain, tier)?;
                    print_evidence(
                        "fdsm@0.8.0",
                        case,
                        glyph.id(),
                        tier,
                        geometry.segment_count(),
                        domain.width,
                        domain.height,
                        &fd,
                        Some(&cross),
                    );
                }
            }
        }
    }

    if glyph_count == 0 {
        return Err("controlled corpus produced no outline glyphs".into());
    }

    println!(
        "summary glyphs={glyph_count} source_segments={segment_count} tiers={} candidate_runs={} shared_corpus=latin-acute,latin-curves,arabic,devanagari",
        TIERS.len(),
        glyph_count * TIERS.len() * 2,
    );
    Ok(())
}
