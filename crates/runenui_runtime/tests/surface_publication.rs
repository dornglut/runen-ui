#![allow(refining_impl_trait)]

use runenui_core::{
    Color, EdgeInsets, Element, LogicalLength, NoHostProtocol, Radius, StyleEnvironment,
    StyleTokens, UiApp, View, button, children, color_token, column, radius_token, row,
    spacing_token, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, LogicalPoint, LogicalSize, MountedNodeId, PumpBudget,
    SurfaceBuildContext, SurfacePhase, SurfacePublication, render_debug_surface_frame,
};

fn publish<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    context: &SurfaceBuildContext<'_>,
) -> SurfacePublication {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("surface publication test is admitted"))
}

#[derive(Debug)]
enum Action {
    Press,
}

#[derive(Clone, Copy, Debug)]
enum Structure {
    Ab,
    Ba,
    A,
    Abc,
    RenamedA,
    NewRoot,
    NestedInitial,
    NestedChanged,
}

#[derive(Clone, Copy, Debug)]
enum StructureAction {
    Set(Structure),
}

struct StructuralApp;

fn structural_leaf(name: &'static str, authored: &'static str) -> Element<StructureAction> {
    text(name).id(authored).key(name).into_element()
}

impl UiApp for StructuralApp {
    type State = Structure;
    type Action = StructureAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        match state {
            Structure::Ab => column(vec![structural_leaf("a", "a"), structural_leaf("b", "b")])
                .id("root")
                .key("root")
                .into_element(),
            Structure::Ba => column(vec![structural_leaf("b", "b"), structural_leaf("a", "a")])
                .id("root")
                .key("root")
                .into_element(),
            Structure::A => column(vec![structural_leaf("a", "a")])
                .id("root")
                .key("root")
                .into_element(),
            Structure::Abc => column(vec![
                structural_leaf("a", "a"),
                structural_leaf("b", "b"),
                structural_leaf("c", "c"),
            ])
            .id("root")
            .key("root")
            .into_element(),
            Structure::RenamedA => column(vec![
                structural_leaf("a", "renamed-a"),
                structural_leaf("b", "b"),
            ])
            .id("root")
            .key("root")
            .into_element(),
            Structure::NewRoot => column(vec![structural_leaf("c", "c")])
                .id("new-root")
                .key("new-root")
                .into_element(),
            Structure::NestedInitial => column(vec![
                column(vec![structural_leaf("a", "a"), structural_leaf("b", "b")])
                    .id("left")
                    .key("left")
                    .into_element(),
                column(vec![structural_leaf("c", "c")])
                    .id("right")
                    .key("right")
                    .into_element(),
            ])
            .id("root")
            .key("root")
            .into_element(),
            Structure::NestedChanged => column(vec![
                column(vec![structural_leaf("c", "c")])
                    .id("right")
                    .key("right")
                    .into_element(),
                column(vec![structural_leaf("b", "b")])
                    .id("left")
                    .key("left")
                    .into_element(),
            ])
            .id("root")
            .key("root")
            .into_element(),
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        let StructureAction::Set(next) = action;
        *state = next;
    }
}

fn assert_structural_alignment(
    runtime: &mut AppRuntime<StructuralApp>,
    publication: &runenui_runtime::SurfacePublication,
    expected_authored: &[&str],
) {
    let index = runtime.index();
    assert_eq!(index.nodes().len(), expected_authored.len());
    assert_eq!(publication.frame().nodes().len(), expected_authored.len());
    assert_eq!(
        publication.style_report().nodes().len(),
        expected_authored.len()
    );
    assert_eq!(
        publication.layout_report().nodes().len(),
        expected_authored.len()
    );
    for (position, expected) in expected_authored.iter().enumerate() {
        let indexed = &index.nodes()[position];
        let framed = &publication.frame().nodes()[position];
        let styled = &publication.style_report().nodes()[position];
        let laid_out = &publication.layout_report().nodes()[position];
        assert_eq!(indexed.id(), framed.id());
        assert_eq!(indexed.id(), styled.id());
        assert_eq!(indexed.id(), laid_out.id());
        assert_eq!(indexed.parent(), framed.parent());
        assert_eq!(indexed.parent(), styled.parent());
        assert_eq!(indexed.parent(), laid_out.parent());
        assert_eq!(indexed.authored_id(), framed.authored_id());
        assert_eq!(indexed.authored_id(), styled.authored_id());
        assert_eq!(indexed.authored_id(), laid_out.authored_id());
        assert_eq!(
            indexed.authored_id().map(runenui_core::ElementId::as_str),
            Some(*expected)
        );
    }
}

fn warm_and_change(
    initial: Structure,
    next: Structure,
) -> (
    AppRuntime<StructuralApp>,
    runenui_runtime::SurfacePublication,
) {
    let mut runtime = AppRuntime::<StructuralApp>::mount(initial);
    let environment = StyleEnvironment::default();
    let _ = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded()),
    );
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    runtime
        .submit_action(StructureAction::Set(next))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded()),
    );
    (runtime, publication)
}

#[test]
fn warmed_cache_keyed_reorder_rebuilds_every_aligned_product() {
    let (mut runtime, publication) = warm_and_change(Structure::Ab, Structure::Ba);
    assert_structural_alignment(&mut runtime, &publication, &["root", "b", "a"]);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            runenui_runtime::SurfacePhase::Tree,
            runenui_runtime::SurfacePhase::Style,
            runenui_runtime::SurfacePhase::Layout,
            runenui_runtime::SurfacePhase::HitTesting,
            runenui_runtime::SurfacePhase::Paint,
            runenui_runtime::SurfacePhase::Semantics,
            runenui_runtime::SurfacePhase::Diagnostics,
        ]
    );
}

#[test]
fn warmed_cache_removal_and_insertion_leave_no_stale_products() {
    let (mut removed_runtime, removed) = warm_and_change(Structure::Ab, Structure::A);
    assert_structural_alignment(&mut removed_runtime, &removed, &["root", "a"]);
    assert!(
        removed
            .frame()
            .nodes()
            .iter()
            .all(|node| node.authored_id().is_none_or(|id| id.as_str() != "b"))
    );

    let (mut inserted_runtime, inserted) = warm_and_change(Structure::Ab, Structure::Abc);
    assert_structural_alignment(&mut inserted_runtime, &inserted, &["root", "a", "b", "c"]);
}

#[test]
fn warmed_cache_authored_id_and_root_replacement_are_current_everywhere() {
    let (mut renamed_runtime, renamed) = warm_and_change(Structure::Ab, Structure::RenamedA);
    assert_structural_alignment(&mut renamed_runtime, &renamed, &["root", "renamed-a", "b"]);
    let (mut replaced_runtime, replaced) = warm_and_change(Structure::Ab, Structure::NewRoot);
    assert_structural_alignment(&mut replaced_runtime, &replaced, &["new-root", "c"]);
}

#[test]
fn warmed_nested_reorder_and_removal_follow_current_mounted_preorder() {
    let (mut runtime, publication) =
        warm_and_change(Structure::NestedInitial, Structure::NestedChanged);
    assert_structural_alignment(
        &mut runtime,
        &publication,
        &["root", "right", "c", "left", "b"],
    );
}
struct App;
impl UiApp for App {
    type State = ();
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root((): &()) -> Element<Action> {
        row(children![
            text("Title").id("title").key("title"),
            button("Press")
                .id("press")
                .key("press")
                .on_activate(|| Action::Press)
        ])
        .key("root")
        .gap(4_u16)
        .padding(EdgeInsets::all(LogicalLength::from(2_u16)))
        .into_element()
    }
    fn update((): &mut (), _: Action) {}
}

#[test]
fn mounted_surface_products_align_and_hit_testing_targets_mounted_ids() {
    let mut runtime = AppRuntime::<App>::mount(());
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
    let publication = publish(&mut runtime, &context);
    assert_eq!(publication.frame().nodes().len(), 3);
    assert_eq!(publication.style_report().nodes().len(), 3);
    assert_eq!(publication.layout_report().nodes().len(), 3);
    for ((frame, style), layout) in publication
        .frame()
        .nodes()
        .iter()
        .zip(publication.style_report().nodes())
        .zip(publication.layout_report().nodes())
    {
        assert_eq!(frame.id(), style.id());
        assert_eq!(frame.id(), layout.id());
        assert!(frame.bounds().width().is_finite());
    }
    let debug = render_debug_surface_frame(publication.frame());
    assert!(debug.contains("authored=press"));
    let press = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "press"))
        .unwrap_or_else(|| unreachable!("the press button is published"));
    let bounds = press.bounds();
    let point = LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published button bounds are finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(point),
        Some(press.id())
    );
}

#[test]
fn finite_saturating_geometry_remains_valid() {
    let size = runenui_runtime::LogicalSize::new(
        LogicalLength::new(f32::MAX).unwrap_or_else(|_| unreachable!()),
        LogicalLength::from(1_u16),
    );
    assert!(size.width().is_finite());
}

#[derive(Clone, Copy, Debug)]
enum CommonFields {
    ForegroundBlack,
    ForegroundWhite,
    BackgroundBlack,
    BackgroundWhite,
    Radius4,
    Radius20,
    Padding4,
    Padding20,
    SpacingSmall,
    SpacingLarge,
    ColorPrimary,
    ColorSecondary,
    Gap4,
    Gap20,
    CombinedInitial,
    CombinedChanged,
    Equivalent,
}

#[derive(Clone, Copy, Debug)]
struct SetCommonFields(CommonFields);

struct CommonFieldsApp;

impl UiApp for CommonFieldsApp {
    type State = CommonFields;
    type Action = SetCommonFields;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let root = match state {
            CommonFields::ForegroundBlack => text("X").foreground(Color::BLACK).into_element(),
            CommonFields::ForegroundWhite => text("X").foreground(Color::WHITE).into_element(),
            CommonFields::BackgroundBlack => text("X").background(Color::BLACK).into_element(),
            CommonFields::BackgroundWhite => text("X").background(Color::WHITE).into_element(),
            CommonFields::Radius4 => text("X")
                .radius(Radius::all(LogicalLength::from(4_u16)))
                .into_element(),
            CommonFields::Radius20 => text("X")
                .radius(Radius::all(LogicalLength::from(20_u16)))
                .into_element(),
            CommonFields::Padding4 => text("X")
                .padding(EdgeInsets::all(LogicalLength::from(4_u16)))
                .into_element(),
            CommonFields::Padding20 => text("X")
                .padding(EdgeInsets::all(LogicalLength::from(20_u16)))
                .into_element(),
            CommonFields::SpacingSmall => text("X")
                .padding(spacing_token!("spacing.small"))
                .into_element(),
            CommonFields::SpacingLarge => text("X")
                .padding(spacing_token!("spacing.large"))
                .into_element(),
            CommonFields::ColorPrimary => text("X")
                .foreground(color_token!("color.primary"))
                .into_element(),
            CommonFields::ColorSecondary => text("X")
                .foreground(color_token!("color.secondary"))
                .into_element(),
            CommonFields::Gap4 => row(children![text("A").key("a"), text("B").key("b")])
                .gap(4_u16)
                .into_element(),
            CommonFields::Gap20 => row(children![text("A").key("a"), text("B").key("b")])
                .gap(20_u16)
                .into_element(),
            CommonFields::CombinedInitial => text("X")
                .padding(EdgeInsets::all(LogicalLength::from(4_u16)))
                .foreground(Color::BLACK)
                .into_element(),
            CommonFields::CombinedChanged => text("X")
                .padding(EdgeInsets::all(LogicalLength::from(20_u16)))
                .foreground(Color::WHITE)
                .into_element(),
            CommonFields::Equivalent => text("X")
                .padding(EdgeInsets::all(LogicalLength::from(4_u16)))
                .foreground(Color::BLACK)
                .background(Color::WHITE)
                .radius(radius_token!("radius.control"))
                .into_element(),
        };
        root.id("root").key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        *state = action.0;
    }
}

type MountedIdentity = MountedNodeId;

fn mounted_identities(runtime: &mut AppRuntime<CommonFieldsApp>) -> Vec<MountedIdentity> {
    runtime
        .index()
        .nodes()
        .iter()
        .map(|node| node.id().clone())
        .collect()
}

fn assert_mounted_identities(
    runtime: &mut AppRuntime<CommonFieldsApp>,
    expected: &[MountedIdentity],
) {
    assert_eq!(mounted_identities(runtime), expected);
}

fn warm_common_fields(
    initial: CommonFields,
    changed: CommonFields,
    tokens: &StyleTokens,
    constraints: LayoutConstraints,
) -> (
    AppRuntime<CommonFieldsApp>,
    runenui_runtime::SurfacePublication,
    runenui_runtime::SurfacePublication,
    Vec<MountedIdentity>,
) {
    let mut runtime = AppRuntime::<CommonFieldsApp>::mount(initial);
    let environment = StyleEnvironment::from_tokens(tokens.clone());
    let context = SurfaceBuildContext::new(&environment, constraints);
    let before = publish(&mut runtime, &context);
    let identities = mounted_identities(&mut runtime);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    runtime
        .submit_action(SetCommonFields(changed))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let after = publish(&mut runtime, &context);
    (runtime, before, after, identities)
}

fn assert_common_phases(runtime: &AppRuntime<CommonFieldsApp>, expected: &[SurfacePhase]) {
    assert_eq!(runtime.last_surface_phase_report().executed(), expected);
    assert!(
        !runtime
            .last_surface_phase_report()
            .contains(SurfacePhase::Tree)
    );
}

fn root_style(publication: &runenui_runtime::SurfacePublication) -> runenui_core::ComputedStyle {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .computed_style()
}

#[test]
fn warmed_literal_foreground_change_reads_current_mounted_style() {
    let tokens = StyleTokens::new();
    let (mut runtime, _, after, identities) = warm_common_fields(
        CommonFields::ForegroundBlack,
        CommonFields::ForegroundWhite,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert_common_phases(&runtime, &[SurfacePhase::Style, SurfacePhase::Paint]);
    assert_eq!(root_style(&after).foreground(), Some(Color::WHITE));
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn warmed_literal_background_change_reads_current_mounted_style() {
    let tokens = StyleTokens::new();
    let (mut runtime, _, after, identities) = warm_common_fields(
        CommonFields::BackgroundBlack,
        CommonFields::BackgroundWhite,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert_common_phases(&runtime, &[SurfacePhase::Style, SurfacePhase::Paint]);
    assert_eq!(root_style(&after).background(), Some(Color::WHITE));
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn warmed_literal_radius_change_reads_current_mounted_style() {
    let tokens = StyleTokens::new();
    let (mut runtime, _, after, identities) = warm_common_fields(
        CommonFields::Radius4,
        CommonFields::Radius20,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert_common_phases(&runtime, &[SurfacePhase::Style, SurfacePhase::Paint]);
    assert_eq!(
        root_style(&after).radius(),
        Some(Radius::all(LogicalLength::from(20_u16)))
    );
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn warmed_literal_padding_change_updates_geometry_from_current_mounted_style() {
    let tokens = StyleTokens::new();
    let constraints = LayoutConstraints::loose(LogicalSize::new(
        LogicalLength::from(100_u16),
        LogicalLength::from(100_u16),
    ));
    let (mut runtime, before, after, identities) = warm_common_fields(
        CommonFields::Padding4,
        CommonFields::Padding20,
        &tokens,
        constraints,
    );
    assert_common_phases(
        &runtime,
        &[
            SurfacePhase::Style,
            SurfacePhase::Layout,
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ],
    );
    assert_eq!(
        root_style(&after).padding(),
        Some(EdgeInsets::all(LogicalLength::from(20_u16)))
    );
    let before_width = before
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds()
        .width();
    let after_width = after
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds()
        .width();
    assert!((after_width - before_width - 32.0).abs() <= f32::EPSILON);
    let content = after
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!())
        .content_constraints();
    assert_eq!(
        content
            .horizontal()
            .max()
            .as_finite()
            .map(LogicalLength::get),
        Some(60.0)
    );
    assert_eq!(
        content.vertical().max().as_finite().map(LogicalLength::get),
        Some(60.0)
    );
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn warmed_spacing_token_reference_change_uses_unchanged_current_token_set() {
    let mut tokens = StyleTokens::new();
    tokens
        .define_spacing(
            spacing_token!("spacing.small"),
            EdgeInsets::all(LogicalLength::from(4_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    tokens
        .define_spacing(
            spacing_token!("spacing.large"),
            EdgeInsets::all(LogicalLength::from(20_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    let revision = tokens.revision();
    let (mut runtime, _, after, identities) = warm_common_fields(
        CommonFields::SpacingSmall,
        CommonFields::SpacingLarge,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert_eq!(tokens.revision(), revision);
    assert_common_phases(
        &runtime,
        &[
            SurfacePhase::Style,
            SurfacePhase::Layout,
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ],
    );
    assert_eq!(
        root_style(&after).padding(),
        Some(EdgeInsets::all(LogicalLength::from(20_u16)))
    );
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn warmed_color_token_reference_change_uses_unchanged_current_token_set() {
    let mut tokens = StyleTokens::new();
    tokens
        .define_color(color_token!("color.primary"), Color::BLACK)
        .unwrap_or_else(|_| unreachable!());
    tokens
        .define_color(color_token!("color.secondary"), Color::WHITE)
        .unwrap_or_else(|_| unreachable!());
    let revision = tokens.revision();
    let (mut runtime, _, after, identities) = warm_common_fields(
        CommonFields::ColorPrimary,
        CommonFields::ColorSecondary,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert_eq!(tokens.revision(), revision);
    assert_common_phases(&runtime, &[SurfacePhase::Style, SurfacePhase::Paint]);
    assert_eq!(root_style(&after).foreground(), Some(Color::WHITE));
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn warmed_container_gap_change_reads_current_mounted_layout() {
    let tokens = StyleTokens::new();
    let (mut runtime, before, after, identities) = warm_common_fields(
        CommonFields::Gap4,
        CommonFields::Gap20,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert_common_phases(
        &runtime,
        &[
            SurfacePhase::Layout,
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ],
    );
    let before_second_x = before.frame().nodes()[2].bounds().x();
    let after_second_x = after.frame().nodes()[2].bounds().x();
    assert!((after_second_x - before_second_x - 16.0).abs() <= f32::EPSILON);
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn warmed_combined_padding_and_color_change_executes_canonical_phases() {
    let tokens = StyleTokens::new();
    let (mut runtime, before, after, identities) = warm_common_fields(
        CommonFields::CombinedInitial,
        CommonFields::CombinedChanged,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert_common_phases(
        &runtime,
        &[
            SurfacePhase::Style,
            SurfacePhase::Layout,
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ],
    );
    assert_eq!(root_style(&after).foreground(), Some(Color::WHITE));
    assert_eq!(
        root_style(&after).padding(),
        Some(EdgeInsets::all(LogicalLength::from(20_u16)))
    );
    assert!(
        after
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!())
            .bounds()
            .width()
            > before
                .frame()
                .root()
                .unwrap_or_else(|| unreachable!())
                .bounds()
                .width()
    );
    assert_mounted_identities(&mut runtime, &identities);
}

#[test]
fn equivalent_common_authored_fields_execute_no_publication_phase() {
    let mut tokens = StyleTokens::new();
    tokens
        .define_radius(
            radius_token!("radius.control"),
            Radius::all(LogicalLength::from(4_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    let (mut runtime, before, after, identities) = warm_common_fields(
        CommonFields::Equivalent,
        CommonFields::Equivalent,
        &tokens,
        LayoutConstraints::unbounded(),
    );
    assert!(before.renderer_products_eq(&after));
    assert_eq!(
        before.hit_test_scene().regions(),
        after.hit_test_scene().regions()
    );
    assert_eq!(
        before.hit_test_scene().mounted_targets(),
        after.hit_test_scene().mounted_targets()
    );
    assert_ne!(before.input_context(), after.input_context());
    assert!(runtime.last_surface_phase_report().executed().is_empty());
    assert_mounted_identities(&mut runtime, &identities);
}

struct TokenApp;

impl UiApp for TokenApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        text("X")
            .padding(spacing_token!("surface.padding"))
            .foreground(color_token!("surface.foreground"))
            .key("root")
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn token_padding(publication: &runenui_runtime::SurfacePublication) -> Option<EdgeInsets> {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .computed_style()
        .padding()
}

#[test]
fn different_token_sets_with_the_same_revision_never_alias() {
    let mut runtime = AppRuntime::<TokenApp>::mount(());
    let mut first = StyleTokens::new();
    first
        .define_spacing(
            spacing_token!("surface.padding"),
            EdgeInsets::all(LogicalLength::from(5_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    let mut second = StyleTokens::new();
    second
        .define_spacing(
            spacing_token!("surface.padding"),
            EdgeInsets::all(LogicalLength::from(20_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(first.revision(), second.revision());
    let first = StyleEnvironment::from_tokens(first);
    let second = StyleEnvironment::from_tokens(second);
    let _ = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&first, LayoutConstraints::unbounded()),
    );
    let second_publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&second, LayoutConstraints::unbounded()),
    );
    assert_eq!(
        token_padding(&second_publication),
        Some(EdgeInsets::all(LogicalLength::from(20_u16)))
    );
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            runenui_runtime::SurfacePhase::Style,
            runenui_runtime::SurfacePhase::Layout,
            runenui_runtime::SurfacePhase::HitTesting,
            runenui_runtime::SurfacePhase::Paint,
            runenui_runtime::SurfacePhase::Semantics,
        ]
    );
}

#[test]
fn divergent_clones_follow_exact_current_token_content() {
    let mut left = StyleTokens::new();
    left.define_color(color_token!("base"), Color::BLACK)
        .unwrap_or_else(|_| unreachable!());
    let mut right = left.clone();
    left.define_spacing(
        spacing_token!("surface.padding"),
        EdgeInsets::all(LogicalLength::from(3_u16)),
    )
    .unwrap_or_else(|_| unreachable!());
    right
        .define_spacing(
            spacing_token!("surface.padding"),
            EdgeInsets::all(LogicalLength::from(11_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(left.revision(), right.revision());
    let left = StyleEnvironment::from_tokens(left);
    let right = StyleEnvironment::from_tokens(right);
    let mut runtime = AppRuntime::<TokenApp>::mount(());
    let _ = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&left, LayoutConstraints::unbounded()),
    );
    let current = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&right, LayoutConstraints::unbounded()),
    );
    assert_eq!(
        token_padding(&current),
        Some(EdgeInsets::all(LogicalLength::from(11_u16)))
    );
}

#[test]
fn equal_token_content_can_reuse_the_warmed_publication() {
    let mut first = StyleTokens::new();
    let mut second = StyleTokens::new();
    for tokens in [&mut first, &mut second] {
        tokens
            .define_spacing(
                spacing_token!("surface.padding"),
                EdgeInsets::all(LogicalLength::from(7_u16)),
            )
            .unwrap_or_else(|_| unreachable!());
    }
    let first = StyleEnvironment::from_tokens(first);
    let second = StyleEnvironment::from_tokens(second);
    let mut runtime = AppRuntime::<TokenApp>::mount(());
    let first_publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&first, LayoutConstraints::unbounded()),
    );
    let second_publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&second, LayoutConstraints::unbounded()),
    );
    assert!(first_publication.renderer_products_eq(&second_publication));
    assert_eq!(
        first_publication.hit_test_scene().regions(),
        second_publication.hit_test_scene().regions()
    );
    assert_eq!(
        first_publication.hit_test_scene().mounted_targets(),
        second_publication.hit_test_scene().mounted_targets()
    );
    assert_ne!(
        first_publication.input_context(),
        second_publication.input_context()
    );
    assert!(runtime.last_surface_phase_report().executed().is_empty());
}

#[test]
fn color_only_token_change_executes_style_and_paint_without_layout() {
    let mut first = StyleTokens::new();
    first
        .define_color(color_token!("surface.foreground"), Color::BLACK)
        .unwrap_or_else(|_| unreachable!());
    let mut second = StyleTokens::new();
    second
        .define_color(color_token!("surface.foreground"), Color::WHITE)
        .unwrap_or_else(|_| unreachable!());
    let first = StyleEnvironment::from_tokens(first);
    let second = StyleEnvironment::from_tokens(second);
    let mut runtime = AppRuntime::<TokenApp>::mount(());
    let before = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&first, LayoutConstraints::unbounded()),
    );
    let before_bounds = before
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds();
    let after = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&second, LayoutConstraints::unbounded()),
    );
    assert_eq!(
        before_bounds,
        after
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!())
            .bounds()
    );
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            runenui_runtime::SurfacePhase::Style,
            runenui_runtime::SurfacePhase::Paint,
        ]
    );
    assert_eq!(
        after
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!())
            .computed_style()
            .foreground(),
        Some(Color::WHITE)
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn saturated_token_revision_still_uses_content_for_cache_compatibility() {
    let mut first = StyleTokens::new();
    first.__seed_revision_for_test(u64::MAX);
    first
        .define_spacing(
            spacing_token!("surface.padding"),
            EdgeInsets::all(LogicalLength::from(4_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    let mut second = StyleTokens::new();
    second.__seed_revision_for_test(u64::MAX);
    second
        .define_spacing(
            spacing_token!("surface.padding"),
            EdgeInsets::all(LogicalLength::from(14_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(first.revision(), second.revision());
    let first = StyleEnvironment::from_tokens(first);
    let second = StyleEnvironment::from_tokens(second);
    let mut runtime = AppRuntime::<TokenApp>::mount(());
    let _ = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&first, LayoutConstraints::unbounded()),
    );
    let current = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&second, LayoutConstraints::unbounded()),
    );
    assert_eq!(
        token_padding(&current),
        Some(EdgeInsets::all(LogicalLength::from(14_u16)))
    );
}

#[test]
fn style_token_revision_invalidates_resolved_padding_and_layout() {
    let mut runtime = AppRuntime::<TokenApp>::mount(());
    let mut tokens = StyleTokens::new();
    let first_environment = StyleEnvironment::from_tokens(tokens.clone());
    let first = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&first_environment, LayoutConstraints::unbounded()),
    );
    let first_width = first
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds()
        .width();
    tokens
        .define_spacing(
            spacing_token!("surface.padding"),
            EdgeInsets::all(LogicalLength::from(5_u16)),
        )
        .unwrap_or_else(|_| unreachable!());
    let second_environment = StyleEnvironment::from_tokens(tokens);
    let second = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&second_environment, LayoutConstraints::unbounded()),
    );
    let second_width = second
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!())
        .bounds()
        .width();
    assert!((second_width - first_width - 10.0).abs() <= f32::EPSILON);
    assert!(
        runtime
            .last_surface_phase_report()
            .contains(runenui_runtime::SurfacePhase::Style)
    );
    assert!(
        runtime
            .last_surface_phase_report()
            .contains(runenui_runtime::SurfacePhase::Layout)
    );
}
