#![allow(refining_impl_trait)]

use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, FocusBoundaryPolicy, FocusScope, FocusScopePolicy,
    LayoutDimension, LayoutStyle, LogicalLength, LogicalPoint, LogicalSize, NoHostProtocol,
    PresentationFocusPolicy, SemanticCommand, StyleEnvironment, SurfacePresentation,
    SurfacePresentationAnchor, SurfacePresentationPlacement, SurfacePresentationSide, UiApp, View,
    Widget, WidgetActivation, button, column, container,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, SurfaceBuildContext};

fn fixed(width: u16, height: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::length(LogicalLength::from(width)))
        .with_height(LayoutDimension::length(LogicalLength::from(height)))
}

fn surface_context<'a>(environment: &'a StyleEnvironment) -> SurfaceBuildContext<'a> {
    SurfaceBuildContext::new(
        environment,
        LayoutConstraints::tight(LogicalSize::new(
            LogicalLength::from(120_u8),
            LogicalLength::from(80_u8),
        )),
    )
}

fn settle<App: UiApp>(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );
}

fn id(
    publication: &runenui_runtime::SurfacePublication,
    authored: &str,
) -> MountedNodeId {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|value| value.as_str() == authored)
        })
        .unwrap_or_else(|| unreachable!("fixture authored node is published"))
        .id()
        .clone()
}

#[derive(Clone, Debug)]
struct FocusOwner;

impl<Action> Widget<Action> for FocusOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
}

impl<Action> ChildBearingWidget<Action> for FocusOwner {}

fn presentation_at<Action>(
    child: Element<Action>,
    authored: &'static str,
) -> Element<Action>
where
    Action: 'static,
{
    column(vec![child])
        .id(authored)
        .key(authored)
        .surface_presentation(
            SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
            )
            .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                LogicalPoint::new(85.0, 30.0)
                    .unwrap_or_else(|_| unreachable!("fixture presentation point is finite")),
            ))
            .with_focus_policy(PresentationFocusPolicy::EnterAndRestore),
        )
        .into_element()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnerRestoreAction {
    Open,
    CloseAndReplace,
}

#[derive(Clone, Debug)]
struct OwnerRestoreState {
    open: bool,
    replacement: bool,
}

struct OwnerRestoreApp;

impl UiApp for OwnerRestoreApp {
    type State = OwnerRestoreState;
    type Action = OwnerRestoreAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let trigger_key = if state.replacement {
            "restore-trigger-replacement"
        } else {
            "restore-trigger-original"
        };
        let trigger = button("trigger")
            .on_activate(|| OwnerRestoreAction::Open)
            .id("restore-trigger")
            .key(trigger_key)
            .with_layout(fixed(20, 20))
            .into_element();
        let mut owned = vec![trigger];
        if state.open {
            let preferred = button("inside")
                .on_activate(|| OwnerRestoreAction::CloseAndReplace)
                .id("restore-inside")
                .key("restore-inside")
                .presentation_focus_preferred(true)
                .with_layout(fixed(20, 20))
                .into_element();
            owned.push(presentation_at(preferred, "restore-presentation"));
        }
        container(FocusOwner, owned)
            .id("restore-owner")
            .key("restore-owner")
            .into_element()
            .focusable(true)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            OwnerRestoreAction::Open => state.open = true,
            OwnerRestoreAction::CloseAndReplace => {
                state.open = false;
                state.replacement = true;
            }
        }
    }
}

#[test]
fn stale_exact_restoration_generation_never_retargets_by_authored_id_and_uses_owner() {
    let environment = StyleEnvironment::default();
    let context = surface_context(&environment);
    let mut runtime = AppRuntime::<OwnerRestoreApp>::mount(OwnerRestoreState {
        open: false,
        replacement: false,
    });
    let first = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("baseline restoration surface publishes"));
    let owner = id(&first, "restore-owner");
    let original_trigger = id(&first, "restore-trigger");
    runtime
        .submit_command(
            original_trigger.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("trigger focus request is accepted"));
    settle(&mut runtime);

    runtime
        .submit_action(OwnerRestoreAction::Open)
        .unwrap_or_else(|_| unreachable!("open action is accepted"));
    settle(&mut runtime);
    let open = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("presentation surface publishes"));
    settle(&mut runtime);
    let inside = id(&open, "restore-inside");
    assert_eq!(runtime.focus().focused_node(), Some(&inside));

    runtime
        .submit_action(OwnerRestoreAction::CloseAndReplace)
        .unwrap_or_else(|_| unreachable!("close and replacement action is accepted"));
    settle(&mut runtime);
    let closed = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("closed surface republishes"));
    let replacement = id(&closed, "restore-trigger");
    assert_ne!(replacement, original_trigger);
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&owner),
        "stale saved generation must fall back to the exact live owner"
    );
    assert_ne!(runtime.focus().focused_node(), Some(&replacement));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScopeRestoreAction {
    Open,
    CloseAndRemoveOwner,
}

#[derive(Clone, Debug)]
struct ScopeRestoreState {
    open: bool,
    owner_present: bool,
}

struct ScopeRestoreApp;

impl UiApp for ScopeRestoreApp {
    type State = ScopeRestoreState;
    type Action = ScopeRestoreAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let fallback = button("scope fallback")
            .on_activate(|| ScopeRestoreAction::Open)
            .id("scope-fallback")
            .key("scope-fallback")
            .with_layout(fixed(20, 20))
            .into_element();
        let mut scope_children = vec![fallback];

        if state.owner_present {
            let trigger = button("scope trigger")
                .on_activate(|| ScopeRestoreAction::Open)
                .id("scope-trigger")
                .key("scope-trigger")
                .with_layout(fixed(20, 20))
                .into_element();
            let mut owner_children = vec![trigger];
            if state.open {
                let inside = button("scope inside")
                    .on_activate(|| ScopeRestoreAction::CloseAndRemoveOwner)
                    .id("scope-inside")
                    .key("scope-inside")
                    .presentation_focus_preferred(true)
                    .with_layout(fixed(20, 20))
                    .into_element();
                owner_children.push(presentation_at(inside, "scope-presentation"));
            }
            scope_children.push(
                container(FocusOwner, owner_children)
                    .id("scope-owner")
                    .key("scope-owner")
                    .into_element(),
            );
        }

        column(scope_children)
            .id("restore-scope")
            .key("restore-scope")
            .into_element()
            .focus_scope(FocusScope::new())
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            ScopeRestoreAction::Open => state.open = true,
            ScopeRestoreAction::CloseAndRemoveOwner => {
                state.open = false;
                state.owner_present = false;
            }
        }
    }
}

#[test]
fn restoration_uses_existing_focus_scope_fallback_when_exact_target_and_owner_are_gone() {
    let environment = StyleEnvironment::default();
    let context = surface_context(&environment);
    let mut runtime = AppRuntime::<ScopeRestoreApp>::mount(ScopeRestoreState {
        open: false,
        owner_present: true,
    });
    let first = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("scope baseline publishes"));
    let trigger = id(&first, "scope-trigger");
    runtime
        .submit_command(
            trigger,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("scope trigger focus is accepted"));
    settle(&mut runtime);

    runtime
        .submit_action(ScopeRestoreAction::Open)
        .unwrap_or_else(|_| unreachable!("scope presentation opens"));
    settle(&mut runtime);
    let open = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("scope presentation publishes"));
    settle(&mut runtime);
    let inside = id(&open, "scope-inside");
    assert_eq!(runtime.focus().focused_node(), Some(&inside));

    runtime
        .submit_action(ScopeRestoreAction::CloseAndRemoveOwner)
        .unwrap_or_else(|_| unreachable!("owner removal is accepted"));
    settle(&mut runtime);
    let closed = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("scope fallback surface publishes"));
    let fallback = id(&closed, "scope-fallback");
    assert_eq!(runtime.focus().focused_node(), Some(&fallback));
    assert_eq!(
        runtime.focus().reason(),
        Some(runenui_runtime::FocusReason::PresentationRestoration)
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrapAction {
    Open,
}

struct TrapApp;

impl UiApp for TrapApp {
    type State = bool;
    type Action = TrapAction;
    type HostProtocol = NoHostProtocol;

    fn root(open: &Self::State) -> impl View<Self::Action> {
        let outside = button("trap outside")
            .on_activate(|| TrapAction::Open)
            .id("trap-outside")
            .key("trap-outside")
            .with_layout(fixed(20, 20))
            .into_element();
        let mut children = vec![outside];
        if *open {
            let first = button("trap first")
                .on_activate(|| TrapAction::Open)
                .id("trap-first")
                .key("trap-first")
                .with_layout(fixed(20, 20))
                .into_element();
            let second = button("trap second")
                .on_activate(|| TrapAction::Open)
                .id("trap-second")
                .key("trap-second")
                .with_layout(fixed(20, 20))
                .into_element();
            children.push(
                column(vec![first, second])
                    .id("trap-presentation")
                    .key("trap-presentation")
                    .into_element()
                    .focus_scope(FocusScope::new().with_policy(FocusScopePolicy::new(
                        FocusBoundaryPolicy::Trap,
                        FocusBoundaryPolicy::Trap,
                    )))
                    .surface_presentation(
                        SurfacePresentation::new(
                            SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
                        )
                        .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                            LogicalPoint::new(80.0, 30.0)
                                .unwrap_or_else(|_| unreachable!("trap point is finite")),
                        ))
                        .with_focus_policy(PresentationFocusPolicy::EnterAndRestore),
                    )
                    .into_element(),
            );
        }
        column(children)
    }

    fn update(open: &mut Self::State, _: Self::Action) {
        *open = true;
    }
}

#[test]
fn modal_style_focus_trapping_reuses_ordinary_focus_scope_authority() {
    let environment = StyleEnvironment::default();
    let context = surface_context(&environment);
    let mut runtime = AppRuntime::<TrapApp>::mount(false);
    runtime
        .submit_action(TrapAction::Open)
        .unwrap_or_else(|_| unreachable!("trap presentation open is accepted"));
    settle(&mut runtime);
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("trap presentation publishes"));
    settle(&mut runtime);
    let second = id(&publication, "trap-second");
    runtime
        .submit_command(
            second.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("trap second focus request is accepted"));
    settle(&mut runtime);
    runtime
        .submit_command(
            second.clone(),
            SemanticCommand::FocusNext,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("trap traversal command is accepted"));
    settle(&mut runtime);
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&second),
        "presentation trapping must be exactly the existing FocusScope trap behavior"
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NestedAction {
    OpenOuter,
    OpenInner,
    CloseInner,
}

#[derive(Clone, Debug)]
struct NestedState {
    outer: bool,
    inner: bool,
}

struct NestedRestoreApp;

impl UiApp for NestedRestoreApp {
    type State = NestedState;
    type Action = NestedAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let trigger = button("outer trigger")
            .on_activate(|| NestedAction::OpenOuter)
            .id("nested-trigger")
            .key("nested-trigger")
            .with_layout(fixed(20, 20))
            .into_element();
        let mut root_children = vec![trigger];

        if state.outer {
            let outer_focus = button("outer focus")
                .on_activate(|| NestedAction::OpenInner)
                .id("outer-focus")
                .key("outer-focus")
                .presentation_focus_preferred(true)
                .with_layout(fixed(20, 20))
                .into_element();
            let mut outer_children = vec![outer_focus];
            if state.inner {
                let inner_focus = button("inner focus")
                    .on_activate(|| NestedAction::CloseInner)
                    .id("inner-focus")
                    .key("inner-focus")
                    .presentation_focus_preferred(true)
                    .with_layout(fixed(20, 20))
                    .into_element();
                outer_children.push(
                    column(vec![inner_focus])
                        .id("inner-presentation")
                        .key("inner-presentation")
                        .surface_presentation(
                            SurfacePresentation::new(
                                SurfacePresentationPlacement::new(
                                    SurfacePresentationSide::Center,
                                ),
                            )
                            .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                                LogicalPoint::new(95.0, 40.0)
                                    .unwrap_or_else(|_| unreachable!("inner point is finite")),
                            ))
                            .with_focus_policy(PresentationFocusPolicy::EnterAndRestore),
                        )
                        .into_element(),
                );
            }
            root_children.push(
                column(outer_children)
                    .id("outer-presentation")
                    .key("outer-presentation")
                    .surface_presentation(
                        SurfacePresentation::new(
                            SurfacePresentationPlacement::new(SurfacePresentationSide::Center),
                        )
                        .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                            LogicalPoint::new(75.0, 30.0)
                                .unwrap_or_else(|_| unreachable!("outer point is finite")),
                        ))
                        .with_focus_policy(PresentationFocusPolicy::EnterAndRestore),
                    )
                    .into_element(),
            );
        }
        column(root_children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            NestedAction::OpenOuter => state.outer = true,
            NestedAction::OpenInner => state.inner = true,
            NestedAction::CloseInner => state.inner = false,
        }
    }
}

#[test]
fn nested_presentation_close_restores_within_surviving_outer_presentation() {
    let environment = StyleEnvironment::default();
    let context = surface_context(&environment);
    let mut runtime = AppRuntime::<NestedRestoreApp>::mount(NestedState {
        outer: false,
        inner: false,
    });
    let baseline = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("nested baseline publishes"));
    let trigger = id(&baseline, "nested-trigger");
    runtime
        .submit_command(
            trigger,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("nested trigger focus request is accepted"));
    settle(&mut runtime);

    runtime
        .submit_action(NestedAction::OpenOuter)
        .unwrap_or_else(|_| unreachable!("outer open is accepted"));
    settle(&mut runtime);
    let outer = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("outer presentation publishes"));
    settle(&mut runtime);
    let outer_focus = id(&outer, "outer-focus");
    assert_eq!(runtime.focus().focused_node(), Some(&outer_focus));

    runtime
        .submit_action(NestedAction::OpenInner)
        .unwrap_or_else(|_| unreachable!("inner open is accepted"));
    settle(&mut runtime);
    let inner = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("inner presentation publishes"));
    settle(&mut runtime);
    let inner_focus = id(&inner, "inner-focus");
    assert_eq!(runtime.focus().focused_node(), Some(&inner_focus));

    runtime
        .submit_action(NestedAction::CloseInner)
        .unwrap_or_else(|_| unreachable!("inner close is accepted"));
    settle(&mut runtime);
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&outer_focus),
        "closing inner presentation must restore into the surviving outer presentation"
    );
}
