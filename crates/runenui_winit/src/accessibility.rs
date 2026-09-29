//! Reusable AccessKit projection at the native adapter boundary.
//!
//! This module deliberately owns all AccessKit identities, caches, and native
//! callback plumbing. `RunenUI` semantic publication and action ingress remain the
//! only semantic/runtime authorities.

#![cfg_attr(test, allow(clippy::ignored_unit_patterns, clippy::unwrap_used))]

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, RwLock},
};

use accesskit::{
    Action, ActionData, ActionRequest, ActivationHandler, AutoComplete as AccessAutoComplete,
    CustomAction, HasPopup, Invalid as AccessInvalid, Node, NodeId,
    Orientation as AccessOrientation, Rect, Role, TextPosition as AccessTextPosition,
    TextSelection as AccessTextSelection, Toggled, Tree, TreeId, TreeUpdate,
};
use runenui_core::{
    SemanticAction, SemanticAutocomplete, SemanticCheckedState, SemanticEditableMode,
    SemanticInvalidState, SemanticNodeId, SemanticOrientation, SemanticPopupKind,
    SemanticPressedState, SemanticRelationshipKind, SemanticRole, SemanticSelectionMode,
    SemanticText, SemanticValue, SurfaceId, TextAffinity, TextPosition, TextSensitivity,
};
use runenui_runtime::{SemanticNode, SemanticPublication, SemanticSnapshot, SemanticUpdateResult};

pub const OPEN_MENU_CUSTOM_ACTION_ID: i32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterDiagnostic {
    UnsupportedInertState(SemanticNodeId),
    UnsupportedCheckedState(SemanticNodeId),
    UnsupportedPressedState(SemanticNodeId),
    UnsupportedInvalidState(SemanticNodeId),
    UnsupportedOrientation(SemanticNodeId),
    UnsupportedPopupKind(SemanticNodeId),
    UnsupportedAutocomplete(SemanticNodeId),
    UnsupportedValueType(SemanticNodeId),
    UnsupportedTextShape(SemanticNodeId),
    UnsupportedRole(SemanticNodeId),
    MissingRelationshipTarget {
        source: SemanticNodeId,
        target: SemanticNodeId,
    },
    UnsupportedRelationship(SemanticNodeId),
    MultipleErrorMessages(SemanticNodeId),
    UnrepresentableCollectionMetadata(SemanticNodeId),
    UnsupportedSemanticAction {
        target: SemanticNodeId,
        action: SemanticAction,
    },
    WrongTreeId,
    UnknownNodeId,
    RetiredNodeId,
    NodeIdSpaceExhausted,
    WrongCustomActionId(i32),
    CustomActionDataMissing,
    UnexpectedActionData(Action),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateMode {
    InitialFull,
    Delta,
    FullResync,
    Unchanged,
}

#[derive(Debug)]
pub struct AccessibilityUpdate {
    pub mode: UpdateMode,
    pub tree_update: TreeUpdate,
    pub diagnostics: Vec<AdapterDiagnostic>,
}

#[derive(Clone, Debug)]
pub enum AccessibilityEvent {
    InitialTreeRequested,
    ActionRequested(ActionRequest),
    AccessibilityDeactivated,
}

impl From<accesskit_winit::Event> for AccessibilityEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        match event.window_event {
            accesskit_winit::WindowEvent::InitialTreeRequested => Self::InitialTreeRequested,
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                Self::ActionRequested(request)
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                Self::AccessibilityDeactivated
            }
        }
    }
}

struct ActivationSnapshot {
    latest: Arc<RwLock<Option<TreeUpdate>>>,
}

impl ActivationHandler for ActivationSnapshot {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.latest.read().ok().and_then(|tree| tree.clone())
    }
}

pub struct SemanticAdapter {
    projection: SurfaceProjection,
    latest_tree: Arc<RwLock<Option<TreeUpdate>>>,
}

impl Default for SemanticAdapter {
    fn default() -> Self {
        Self {
            projection: SurfaceProjection::new(),
            latest_tree: Arc::new(RwLock::new(None)),
        }
    }
}

impl SemanticAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn activation_handler(&self) -> impl ActivationHandler + Send + 'static {
        ActivationSnapshot {
            latest: Arc::clone(&self.latest_tree),
        }
    }

    pub fn update(&mut self, publication: &SemanticPublication) -> AccessibilityUpdate {
        let (mode, tree_update, diagnostics) = self.projection.update(publication);
        if self.projection.current_snapshot.is_some() {
            let full_tree = self.projection.full_tree_update();
            if let Ok(mut latest) = self.latest_tree.write() {
                *latest = Some(full_tree);
            }
        }
        AccessibilityUpdate {
            mode,
            tree_update,
            diagnostics,
        }
    }

    /// Translates one native `AccessKit` action into the exact neutral semantic request.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterDiagnostic`] when the request targets the wrong tree, an unknown or
    /// retired adapter node, carries invalid action data, or requests an unsupported semantic
    /// action.
    pub fn action_request(
        &self,
        request: &ActionRequest,
    ) -> Result<runenui_core::SemanticActionRequest, AdapterDiagnostic> {
        if request.target_tree != self.projection.tree_id {
            return Err(AdapterDiagnostic::WrongTreeId);
        }
        self.projection.action_request(request)
    }

    #[cfg(test)]
    fn active_id(&self, surface: &SurfaceId, semantic: &SemanticNodeId) -> Option<NodeId> {
        if self.projection.current_surface.as_ref() != Some(surface) {
            return None;
        }
        self.projection.semantic_to_accesskit.get(semantic).copied()
    }
}

struct SurfaceProjection {
    tree_id: TreeId,
    current_surface: Option<SurfaceId>,
    current_revision: Option<runenui_runtime::SemanticRevision>,
    current_snapshot: Option<SemanticSnapshot>,
    semantic_to_accesskit: HashMap<SemanticNodeId, NodeId>,
    accesskit_to_semantic: HashMap<NodeId, SemanticNodeId>,
    editable_text_runs: HashMap<SemanticNodeId, NodeId>,
    retired_semantic: HashSet<SemanticNodeId>,
    retired_accesskit: HashSet<NodeId>,
    current_nodes: BTreeMap<NodeId, Node>,
    synthetic_root: Option<NodeId>,
    next_node_id: Option<u64>,
}

impl SurfaceProjection {
    fn new() -> Self {
        Self {
            tree_id: TreeId::ROOT,
            current_surface: None,
            current_revision: None,
            current_snapshot: None,
            semantic_to_accesskit: HashMap::new(),
            accesskit_to_semantic: HashMap::new(),
            editable_text_runs: HashMap::new(),
            retired_semantic: HashSet::new(),
            retired_accesskit: HashSet::new(),
            current_nodes: BTreeMap::new(),
            synthetic_root: None,
            next_node_id: Some(1),
        }
    }

    fn update(
        &mut self,
        publication: &SemanticPublication,
    ) -> (UpdateMode, TreeUpdate, Vec<AdapterDiagnostic>) {
        let snapshot = publication.snapshot();
        let previous_snapshot = self.current_snapshot.clone();
        let result = self
            .current_surface
            .as_ref()
            .zip(self.current_revision)
            .map_or(
                SemanticUpdateResult::FullResync(snapshot),
                |(surface, revision)| publication.update_from(surface, revision),
            );
        match result {
            SemanticUpdateResult::Delta(delta) => {
                if snapshot
                    .nodes()
                    .iter()
                    .any(|node| node.editable().is_some())
                    || previous_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot
                            .nodes()
                            .iter()
                            .any(|node| node.editable().is_some())
                    })
                {
                    if !self.has_node_id_capacity(self.full_resync_allocation_count(snapshot)) {
                        return self.node_id_exhausted_update();
                    }
                    let (tree_update, diagnostics) = self.full_resync(snapshot);
                    return (UpdateMode::FullResync, tree_update, diagnostics);
                }
                if !self.has_node_id_capacity(self.delta_allocation_count(snapshot, delta)) {
                    return self.node_id_exhausted_update();
                }
                let (tree_update, diagnostics) =
                    self.apply_delta(snapshot, previous_snapshot.as_ref(), delta);
                (UpdateMode::Delta, tree_update, diagnostics)
            }
            SemanticUpdateResult::Unchanged => {
                let tree_update = TreeUpdate {
                    nodes: Vec::new(),
                    tree: None,
                    tree_id: self.tree_id,
                    focus: self.focus_id(snapshot),
                };
                (UpdateMode::Unchanged, tree_update, Vec::new())
            }
            SemanticUpdateResult::FullResync(snapshot) => {
                if !self.has_node_id_capacity(self.full_resync_allocation_count(snapshot)) {
                    return self.node_id_exhausted_update();
                }
                let (tree_update, diagnostics) = self.full_resync(snapshot);
                let mode = if previous_snapshot.is_some() {
                    UpdateMode::FullResync
                } else {
                    UpdateMode::InitialFull
                };
                (mode, tree_update, diagnostics)
            }
        }
    }

    fn semantic_node_requires_id(&self, semantic: &SemanticNodeId) -> bool {
        self.retired_semantic.contains(semantic)
            || !self.semantic_to_accesskit.contains_key(semantic)
    }

    fn full_resync_allocation_count(&self, snapshot: &SemanticSnapshot) -> Option<usize> {
        let semantic_ids = snapshot
            .nodes()
            .iter()
            .filter(|node| self.semantic_node_requires_id(node.id()))
            .count();
        let editable_text_runs = snapshot
            .nodes()
            .iter()
            .filter(|node| {
                supports_editable_text_run(node) && !self.editable_text_runs.contains_key(node.id())
            })
            .count();
        let surface_changed = self
            .current_surface
            .as_ref()
            .is_some_and(|surface| surface != snapshot.surface_id());
        let synthetic_root = usize::from(
            snapshot.roots().len() != 1 && (surface_changed || self.synthetic_root.is_none()),
        );
        semantic_ids
            .checked_add(editable_text_runs)?
            .checked_add(synthetic_root)
    }

    fn delta_allocation_count(
        &self,
        snapshot: &SemanticSnapshot,
        delta: &runenui_runtime::SemanticUpdate,
    ) -> Option<usize> {
        let semantic_ids = delta
            .added()
            .iter()
            .filter(|node| self.semantic_node_requires_id(node.id()))
            .count();
        let synthetic_root =
            usize::from(snapshot.roots().len() != 1 && self.synthetic_root.is_none());
        semantic_ids.checked_add(synthetic_root)
    }

    fn has_node_id_capacity(&self, required: Option<usize>) -> bool {
        let Some(required) = required else {
            return false;
        };
        let Ok(required) = u64::try_from(required) else {
            return false;
        };
        if required == 0 {
            return true;
        }
        let Some(next_node_id) = self.next_node_id else {
            return false;
        };
        next_node_id.checked_add(required - 1).is_some()
    }

    fn node_id_exhausted_update(&self) -> (UpdateMode, TreeUpdate, Vec<AdapterDiagnostic>) {
        let focus = self
            .current_snapshot
            .as_ref()
            .map_or(NodeId(0), |snapshot| {
                self.focus_id_without_mutation(snapshot)
            });
        (
            UpdateMode::Unchanged,
            TreeUpdate {
                nodes: Vec::new(),
                tree: None,
                tree_id: self.tree_id,
                focus,
            },
            vec![AdapterDiagnostic::NodeIdSpaceExhausted],
        )
    }

    fn allocate_node_id(&mut self) -> NodeId {
        let value = self
            .next_node_id
            .unwrap_or_else(|| unreachable!("AccessKit node ID capacity is preflighted"));
        self.next_node_id = value.checked_add(1);
        NodeId(value)
    }

    fn ensure_node_id(&mut self, semantic: &SemanticNodeId) -> NodeId {
        if !self.retired_semantic.contains(semantic)
            && let Some(id) = self.semantic_to_accesskit.get(semantic).copied()
        {
            return id;
        }
        let id = self.allocate_node_id();
        self.semantic_to_accesskit.insert(semantic.clone(), id);
        self.accesskit_to_semantic.insert(id, semantic.clone());
        id
    }

    fn retire_missing(&mut self, snapshot: &SemanticSnapshot) {
        let live: HashSet<_> = snapshot
            .nodes()
            .iter()
            .map(|node| node.id().clone())
            .collect();
        let retired: Vec<_> = self
            .semantic_to_accesskit
            .keys()
            .filter(|id| !live.contains(*id))
            .cloned()
            .collect();
        for semantic in retired {
            if let Some(text_run) = self.editable_text_runs.remove(&semantic) {
                self.retired_accesskit.insert(text_run);
            }
            if let Some(accesskit) = self.semantic_to_accesskit.remove(&semantic) {
                self.accesskit_to_semantic.remove(&accesskit);
                self.retired_accesskit.insert(accesskit);
                self.retired_semantic.insert(semantic);
            }
        }
        let text_runs_to_retire = self
            .editable_text_runs
            .keys()
            .filter(|semantic| {
                snapshot
                    .node(semantic)
                    .is_none_or(|node| !supports_editable_text_run(node))
            })
            .cloned()
            .collect::<Vec<_>>();
        for semantic in text_runs_to_retire {
            if let Some(text_run) = self.editable_text_runs.remove(&semantic) {
                self.retired_accesskit.insert(text_run);
            }
        }
    }

    fn retire_synthetic_root(&mut self) {
        if let Some(root) = self.synthetic_root.take() {
            self.retired_accesskit.insert(root);
        }
    }

    fn root_id(&mut self, snapshot: &SemanticSnapshot) -> NodeId {
        if snapshot.roots().len() == 1 {
            self.semantic_to_accesskit
                .get(&snapshot.roots()[0])
                .copied()
                .unwrap_or_else(|| unreachable!("all snapshot roots receive adapter IDs first"))
        } else if let Some(root) = self.synthetic_root {
            root
        } else {
            let root = self.allocate_node_id();
            self.synthetic_root = Some(root);
            root
        }
    }

    fn full_resync(&mut self, snapshot: &SemanticSnapshot) -> (TreeUpdate, Vec<AdapterDiagnostic>) {
        let surface_changed = self
            .current_surface
            .as_ref()
            .is_some_and(|surface| surface != snapshot.surface_id());
        if surface_changed || snapshot.roots().len() == 1 {
            self.retire_synthetic_root();
        }
        self.retire_missing(snapshot);
        for node in snapshot.nodes() {
            self.ensure_node_id(node.id());
            if supports_editable_text_run(node) && !self.editable_text_runs.contains_key(node.id())
            {
                let text_run = self.allocate_node_id();
                self.editable_text_runs.insert(node.id().clone(), text_run);
            }
        }
        let root = self.root_id(snapshot);
        let (nodes, diagnostics) = self.project_all_nodes(snapshot, root);
        self.current_nodes = nodes.iter().cloned().collect();
        self.current_surface = Some(snapshot.surface_id().clone());
        self.current_revision = Some(snapshot.revision());
        self.current_snapshot = Some(snapshot.clone());
        (
            TreeUpdate {
                nodes,
                tree: Some(Tree::new(root)),
                tree_id: self.tree_id,
                focus: self.focus_id(snapshot),
            },
            diagnostics,
        )
    }

    fn apply_delta(
        &mut self,
        snapshot: &SemanticSnapshot,
        previous_snapshot: Option<&SemanticSnapshot>,
        delta: &runenui_runtime::SemanticUpdate,
    ) -> (TreeUpdate, Vec<AdapterDiagnostic>) {
        for node in delta.added() {
            self.ensure_node_id(node.id());
        }
        let removed: HashSet<_> = delta.removed().iter().cloned().collect();
        for semantic in &removed {
            if let Some(accesskit) = self.semantic_to_accesskit.remove(semantic) {
                self.accesskit_to_semantic.remove(&accesskit);
                self.retired_accesskit.insert(accesskit);
                self.retired_semantic.insert(semantic.clone());
            }
        }
        if snapshot.roots().len() == 1 {
            self.retire_synthetic_root();
        }
        let root = self.root_id(snapshot);
        let current_collection_sizes = collection_sizes(snapshot);
        let previous_collection_sizes = previous_snapshot.map(collection_sizes).unwrap_or_default();
        let mut changed = Vec::new();
        for node in delta.added().iter().chain(delta.changed()) {
            if !changed.contains(node.id()) {
                changed.push(node.id().clone());
            }
        }
        if let Some(previous) = previous_snapshot {
            for removed_id in delta.removed() {
                if let Some(parent) = previous.node(removed_id).and_then(SemanticNode::parent)
                    && !changed.contains(parent)
                {
                    changed.push(parent.clone());
                }
            }
        }
        for collection in previous_collection_sizes
            .keys()
            .chain(current_collection_sizes.keys())
        {
            if previous_collection_sizes.get(collection) != current_collection_sizes.get(collection)
                && snapshot.node(collection).is_some()
                && !changed.contains(collection)
            {
                changed.push(collection.clone());
            }
        }
        if delta.roots().is_some() {
            for root_id in snapshot.roots() {
                if !changed.contains(root_id) {
                    changed.push(root_id.clone());
                }
            }
            if snapshot.roots().len() != 1 {
                changed.clear();
            }
        }
        let mut diagnostics = Vec::new();
        let mut projected = Vec::new();
        if snapshot.roots().len() != 1 && delta.roots().is_some() {
            let (all, all_diagnostics) = self.project_all_nodes(snapshot, root);
            projected = all;
            diagnostics.extend(all_diagnostics);
        } else {
            for id in &changed {
                if let Some(node) = snapshot.node(id) {
                    let accesskit_id = self.semantic_to_accesskit[id];
                    let (node, node_diagnostics) =
                        self.project_node(snapshot, node, &current_collection_sizes);
                    diagnostics.extend(node_diagnostics);
                    projected.push((accesskit_id, node));
                }
            }
        }
        self.current_nodes = self
            .project_all_nodes(snapshot, root)
            .0
            .into_iter()
            .collect();
        self.current_surface = Some(snapshot.surface_id().clone());
        self.current_revision = Some(snapshot.revision());
        self.current_snapshot = Some(snapshot.clone());
        (
            TreeUpdate {
                nodes: projected,
                tree: delta.roots().map(|_| Tree::new(root)),
                tree_id: self.tree_id,
                focus: self.focus_id(snapshot),
            },
            diagnostics,
        )
    }

    fn project_all_nodes(
        &self,
        snapshot: &SemanticSnapshot,
        root: NodeId,
    ) -> (Vec<(NodeId, Node)>, Vec<AdapterDiagnostic>) {
        let mut result = Vec::with_capacity(snapshot.nodes().len() + 1);
        let mut diagnostics = Vec::new();
        if snapshot.roots().len() != 1 {
            let mut synthetic = Node::new(Role::GenericContainer);
            synthetic.set_children(
                snapshot
                    .roots()
                    .iter()
                    .filter_map(|id| self.semantic_to_accesskit.get(id).copied())
                    .collect::<Vec<_>>(),
            );
            result.push((root, synthetic));
        }
        let collection_sizes = collection_sizes(snapshot);
        for semantic in snapshot.nodes() {
            let accesskit_id = self.semantic_to_accesskit[semantic.id()];
            let (node, node_diagnostics) = self.project_node(snapshot, semantic, &collection_sizes);
            diagnostics.extend(node_diagnostics);
            result.push((accesskit_id, node));
            if let Some(text_run_id) = self.editable_text_runs.get(semantic.id()).copied()
                && let Some(text_run) = project_editable_text_run(semantic)
            {
                result.push((text_run_id, text_run));
            }
        }
        (result, diagnostics)
    }

    #[allow(clippy::too_many_lines)]
    fn project_node(
        &self,
        snapshot: &SemanticSnapshot,
        semantic: &SemanticNode,
        collection_sizes: &HashMap<SemanticNodeId, Result<usize, ()>>,
    ) -> (Node, Vec<AdapterDiagnostic>) {
        let mut diagnostics = Vec::new();
        let role = if let Some(editable) = semantic.editable() {
            map_editable_role(semantic, editable.sensitivity(), &mut diagnostics)
        } else {
            map_role(semantic.role(), semantic.id(), &mut diagnostics)
        };
        let mut node = Node::new(role);
        if semantic.state().disabled() {
            node.set_disabled();
        }
        if semantic.state().inert() {
            diagnostics.push(AdapterDiagnostic::UnsupportedInertState(
                semantic.id().clone(),
            ));
        }
        if semantic.state().read_only() {
            node.set_read_only();
        }
        if let Some(checked) = semantic.state().checked()
            && let Some(toggled) = map_checked_state(checked, semantic.id(), &mut diagnostics)
        {
            node.set_toggled(toggled);
        }
        if let Some(pressed) = semantic.state().pressed()
            && let Some(toggled) = map_pressed_state(pressed, semantic.id(), &mut diagnostics)
        {
            node.set_toggled(toggled);
        }
        if let Some(selected) = semantic.state().selected() {
            node.set_selected(selected);
        }
        if let Some(expanded) = semantic.state().expanded() {
            node.set_expanded(expanded);
        }
        if semantic.state().required() == Some(true) {
            node.set_required();
        }
        if let Some(invalid) = semantic.state().invalid()
            && let Some(invalid) = map_invalid_state(invalid, semantic.id(), &mut diagnostics)
        {
            node.set_invalid(invalid);
        }
        if semantic.state().modal() == Some(true) {
            node.set_modal();
        }
        if let Some(name) = semantic.name() {
            let is_duplicate_text = matches!(role, Role::Label)
                && semantic
                    .text()
                    .and_then(SemanticText::as_plain)
                    .is_some_and(|text| text == name);
            if !is_duplicate_text {
                node.set_label(name);
            }
        }
        if let Some(description) = semantic.description() {
            node.set_description(description);
        }
        if let Some(value) = semantic.value() {
            match value {
                SemanticValue::Text(value) => node.set_value(value.as_str()),
                SemanticValue::Boolean(_) | SemanticValue::Integer(_) => {
                    diagnostics.push(AdapterDiagnostic::UnsupportedValueType(
                        semantic.id().clone(),
                    ));
                }
                #[allow(unreachable_patterns)]
                _ => diagnostics.push(AdapterDiagnostic::UnsupportedValueType(
                    semantic.id().clone(),
                )),
            }
        }
        if let Some(editable) = semantic.editable() {
            if let Some(value) = editable.value() {
                node.set_value(value);
            }
            if let (Some(text_run), Some(offsets)) = (
                self.editable_text_runs.get(semantic.id()).copied(),
                editable.caret_offsets(),
            ) && let (Ok(anchor), Ok(focus)) = (
                offsets.binary_search(&editable.selection().anchor().byte_offset()),
                offsets.binary_search(&editable.selection().active().byte_offset()),
            ) {
                node.set_text_selection(AccessTextSelection {
                    anchor: AccessTextPosition {
                        node: text_run,
                        character_index: anchor,
                    },
                    focus: AccessTextPosition {
                        node: text_run,
                        character_index: focus,
                    },
                });
            }
        }
        if let Some(text) = semantic.text() {
            match text.as_plain() {
                Some(text) if matches!(role, Role::Label) => node.set_value(text),
                Some(_) | None => diagnostics.push(AdapterDiagnostic::UnsupportedTextShape(
                    semantic.id().clone(),
                )),
            }
        }
        if let Some(range) = semantic.range() {
            if let Some(value) = range.minimum() {
                node.set_min_numeric_value(value.get());
            }
            if let Some(value) = range.maximum() {
                node.set_max_numeric_value(value.get());
            }
            if let Some(value) = range.current() {
                node.set_numeric_value(value.get());
            }
            if let Some(value) = range.small_step() {
                node.set_numeric_value_step(value.get());
            }
            if let Some(value) = range.large_step() {
                node.set_numeric_value_jump(value.get());
            }
            if let Some(value_text) = range.value_text() {
                node.set_value(value_text);
            }
        }
        if let Some(orientation) = semantic.orientation()
            && let Some(orientation) = map_orientation(orientation, semantic.id(), &mut diagnostics)
        {
            node.set_orientation(orientation);
        }
        if let Some(popup) = semantic.popup()
            && let Some(popup) = map_popup(popup, semantic.id(), &mut diagnostics)
        {
            node.set_has_popup(popup);
        }
        if semantic.selection_mode() == Some(SemanticSelectionMode::Multiple) {
            node.set_multiselectable();
        }
        if let Some(position) = semantic.collection_position() {
            match usize::try_from(position.index()) {
                Ok(index) => node.set_position_in_set(index),
                Err(_) => diagnostics.push(AdapterDiagnostic::UnrepresentableCollectionMetadata(
                    semantic.id().clone(),
                )),
            }
        }
        match collection_sizes.get(semantic.id()).copied() {
            Some(Ok(size)) => node.set_size_of_set(size),
            Some(Err(())) => diagnostics.push(
                AdapterDiagnostic::UnrepresentableCollectionMetadata(semantic.id().clone()),
            ),
            None => {}
        }
        if semantic
            .collection_position()
            .and_then(runenui_core::SemanticCollectionPosition::known_size)
            .is_some()
            && collection_parent(snapshot, semantic).is_none()
        {
            diagnostics.push(AdapterDiagnostic::UnrepresentableCollectionMetadata(
                semantic.id().clone(),
            ));
        }
        if let Some(level) = semantic.hierarchy_level() {
            let zero_based = level
                .get()
                .checked_sub(1)
                .unwrap_or_else(|| unreachable!("semantic hierarchy level is positive"));
            node.set_level(
                usize::try_from(zero_based)
                    .unwrap_or_else(|_| unreachable!("u32 hierarchy level fits target usize")),
            );
        }
        if let Some(placeholder) = semantic.placeholder() {
            node.set_placeholder(placeholder);
        }
        if let Some(autocomplete) = semantic.autocomplete()
            && let Some(autocomplete) =
                map_autocomplete(autocomplete, semantic.id(), &mut diagnostics)
        {
            node.set_auto_complete(autocomplete);
        }

        let mut controls = Vec::new();
        let mut described_by = Vec::new();
        let mut labelled_by = Vec::new();
        let mut error_messages = Vec::new();
        let mut active_descendants = Vec::new();
        for relationship in semantic.relationships() {
            let Some(target) = self
                .semantic_to_accesskit
                .get(relationship.target())
                .copied()
            else {
                diagnostics.push(AdapterDiagnostic::MissingRelationshipTarget {
                    source: semantic.id().clone(),
                    target: relationship.target().clone(),
                });
                continue;
            };
            match relationship.kind() {
                SemanticRelationshipKind::LabelledBy => labelled_by.push(target),
                SemanticRelationshipKind::DescribedBy => described_by.push(target),
                SemanticRelationshipKind::Controls => controls.push(target),
                SemanticRelationshipKind::ErrorMessage => error_messages.push(target),
                SemanticRelationshipKind::ActiveDescendant => active_descendants.push(target),
                #[allow(unreachable_patterns)]
                _ => diagnostics.push(AdapterDiagnostic::UnsupportedRelationship(
                    semantic.id().clone(),
                )),
            }
        }
        if !labelled_by.is_empty() {
            node.set_labelled_by(labelled_by);
        }
        if !described_by.is_empty() {
            node.set_described_by(described_by);
        }
        if !controls.is_empty() {
            node.set_controls(controls);
        }
        match error_messages.as_slice() {
            [] => {}
            [target] => node.set_error_message(*target),
            _ => diagnostics.push(AdapterDiagnostic::MultipleErrorMessages(
                semantic.id().clone(),
            )),
        }
        match active_descendants.as_slice() {
            [] => {}
            [target] => node.set_active_descendant(*target),
            _ => diagnostics.push(AdapterDiagnostic::UnsupportedRelationship(
                semantic.id().clone(),
            )),
        }
        for action in semantic.supported_actions() {
            match action {
                SemanticAction::Activate => node.add_action(Action::Click),
                SemanticAction::RequestFocus => node.add_action(Action::Focus),
                SemanticAction::OpenContextMenu => node.add_action(Action::ShowContextMenu),
                SemanticAction::OpenMenu => node.add_action(Action::CustomAction),
                SemanticAction::SetSelection
                    if self.editable_text_runs.contains_key(semantic.id()) =>
                {
                    node.add_action(Action::SetTextSelection);
                }
                SemanticAction::SetSelection => {}
                SemanticAction::ReplaceSelection => node.add_action(Action::ReplaceSelectedText),
                #[allow(unreachable_patterns)]
                _ => diagnostics.push(AdapterDiagnostic::UnsupportedSemanticAction {
                    target: semantic.id().clone(),
                    action: action.clone(),
                }),
            }
        }
        if semantic
            .supported_actions()
            .contains(&SemanticAction::OpenMenu)
        {
            node.set_custom_actions([CustomAction {
                id: OPEN_MENU_CUSTOM_ACTION_ID,
                description: "Open menu".into(),
            }]);
        }
        let mut children = semantic
            .children()
            .iter()
            .filter_map(|id| self.semantic_to_accesskit.get(id).copied())
            .collect::<Vec<_>>();
        if let Some(text_run) = self.editable_text_runs.get(semantic.id()).copied() {
            children.insert(0, text_run);
        }
        node.set_children(children);
        let bounds = semantic.bounds();
        node.set_bounds(Rect {
            x0: f64::from(bounds.x()),
            y0: f64::from(bounds.y()),
            x1: f64::from(bounds.x() + bounds.width()),
            y1: f64::from(bounds.y() + bounds.height()),
        });
        (node, diagnostics)
    }

    fn focus_id(&self, snapshot: &SemanticSnapshot) -> NodeId {
        snapshot
            .focused()
            .and_then(|id| self.semantic_to_accesskit.get(id).copied())
            .unwrap_or_else(|| self.root_id_for_snapshot(snapshot))
    }

    fn root_id_for_snapshot(&self, snapshot: &SemanticSnapshot) -> NodeId {
        if snapshot.roots().len() == 1 {
            self.semantic_to_accesskit[&snapshot.roots()[0]]
        } else {
            self.synthetic_root.unwrap_or(NodeId(0))
        }
    }

    fn full_tree_update(&self) -> TreeUpdate {
        let snapshot = self
            .current_snapshot
            .as_ref()
            .unwrap_or_else(|| unreachable!("projection update stores snapshot"));
        let root = self.root_id_for_snapshot(snapshot);
        TreeUpdate {
            nodes: self
                .current_nodes
                .iter()
                .map(|(id, node)| (*id, node.clone()))
                .collect(),
            tree: Some(Tree::new(root)),
            tree_id: self.tree_id,
            focus: self.focus_id_without_mutation(snapshot),
        }
    }

    fn focus_id_without_mutation(&self, snapshot: &SemanticSnapshot) -> NodeId {
        snapshot
            .focused()
            .and_then(|id| self.semantic_to_accesskit.get(id).copied())
            .unwrap_or_else(|| self.root_id_for_snapshot(snapshot))
    }

    #[allow(clippy::too_many_lines)]
    fn action_request(
        &self,
        request: &ActionRequest,
    ) -> Result<runenui_core::SemanticActionRequest, AdapterDiagnostic> {
        let semantic = self
            .accesskit_to_semantic
            .get(&request.target_node)
            .ok_or_else(|| {
                if self.retired_accesskit.contains(&request.target_node) {
                    AdapterDiagnostic::RetiredNodeId
                } else {
                    AdapterDiagnostic::UnknownNodeId
                }
            })?;
        let snapshot = self
            .current_snapshot
            .as_ref()
            .ok_or(AdapterDiagnostic::RetiredNodeId)?;
        let node = snapshot
            .node(semantic)
            .ok_or(AdapterDiagnostic::RetiredNodeId)?;
        if request.action == Action::SetTextSelection {
            let Some(ActionData::SetTextSelection(selection)) = request.data.as_ref() else {
                return Err(AdapterDiagnostic::UnexpectedActionData(request.action));
            };
            if !node
                .supported_actions()
                .contains(&SemanticAction::SetSelection)
            {
                return Err(AdapterDiagnostic::UnsupportedSemanticAction {
                    target: semantic.clone(),
                    action: SemanticAction::SetSelection,
                });
            }
            let editable = node
                .editable()
                .ok_or(AdapterDiagnostic::UnexpectedActionData(request.action))?;
            let source = editable
                .value()
                .ok_or(AdapterDiagnostic::UnexpectedActionData(request.action))?;
            let offsets = editable
                .caret_offsets()
                .ok_or(AdapterDiagnostic::UnexpectedActionData(request.action))?;
            let text_run = self
                .editable_text_runs
                .get(semantic)
                .copied()
                .ok_or(AdapterDiagnostic::UnknownNodeId)?;
            if selection.anchor.node != text_run || selection.focus.node != text_run {
                return Err(AdapterDiagnostic::UnknownNodeId);
            }
            let anchor_offset = offsets
                .get(selection.anchor.character_index)
                .copied()
                .ok_or(AdapterDiagnostic::UnexpectedActionData(request.action))?;
            let focus_offset = offsets
                .get(selection.focus.character_index)
                .copied()
                .ok_or(AdapterDiagnostic::UnexpectedActionData(request.action))?;
            let position = |offset| {
                TextPosition::new(
                    editable.snapshot(),
                    source,
                    offset,
                    if !source.is_empty() && offset == source.len() {
                        TextAffinity::Upstream
                    } else {
                        TextAffinity::Downstream
                    },
                )
                .map_err(|_| AdapterDiagnostic::UnexpectedActionData(request.action))
            };
            let selection =
                runenui_core::TextSelection::new(position(anchor_offset)?, position(focus_offset)?)
                    .map_err(|_| AdapterDiagnostic::UnexpectedActionData(request.action))?;
            return Ok(runenui_core::SemanticActionRequest::set_selection(
                snapshot.surface_id().clone(),
                semantic.clone(),
                selection,
            ));
        }
        if request.action == Action::ReplaceSelectedText {
            let Some(ActionData::Value(value)) = request.data.as_ref() else {
                return Err(AdapterDiagnostic::UnexpectedActionData(request.action));
            };
            if !node
                .supported_actions()
                .contains(&SemanticAction::ReplaceSelection)
            {
                return Err(AdapterDiagnostic::UnsupportedSemanticAction {
                    target: semantic.clone(),
                    action: SemanticAction::ReplaceSelection,
                });
            }
            return Ok(runenui_core::SemanticActionRequest::replace_selection(
                snapshot.surface_id().clone(),
                semantic.clone(),
                value.as_ref(),
            ));
        }
        let action = match request.action {
            Action::Click => SemanticAction::Activate,
            Action::Focus => SemanticAction::RequestFocus,
            Action::ShowContextMenu => SemanticAction::OpenContextMenu,
            Action::CustomAction => match request.data {
                Some(ActionData::CustomAction(id)) if id == OPEN_MENU_CUSTOM_ACTION_ID => {
                    SemanticAction::OpenMenu
                }
                Some(ActionData::CustomAction(id)) => {
                    return Err(AdapterDiagnostic::WrongCustomActionId(id));
                }
                None | Some(_) => return Err(AdapterDiagnostic::CustomActionDataMissing),
            },
            unsupported => return Err(AdapterDiagnostic::UnexpectedActionData(unsupported)),
        };
        if request.action != Action::CustomAction && request.data.is_some() {
            return Err(AdapterDiagnostic::UnexpectedActionData(request.action));
        }
        if !node.supported_actions().contains(&action) {
            return Err(AdapterDiagnostic::UnsupportedSemanticAction {
                target: semantic.clone(),
                action,
            });
        }
        Ok(runenui_core::SemanticActionRequest::new(
            snapshot.surface_id().clone(),
            semantic.clone(),
            action,
        ))
    }
}

fn map_role(
    role: SemanticRole,
    id: &SemanticNodeId,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Role {
    match role {
        SemanticRole::Generic => Role::GenericContainer,
        SemanticRole::Group => Role::Group,
        SemanticRole::Text => Role::Label,
        SemanticRole::Button => Role::Button,
        SemanticRole::EditableText => Role::TextInput,
        SemanticRole::Checkbox => Role::CheckBox,
        SemanticRole::RadioButton => Role::RadioButton,
        SemanticRole::RadioGroup => Role::RadioGroup,
        SemanticRole::Switch => Role::Switch,
        SemanticRole::Link => Role::Link,
        SemanticRole::Image => Role::Image,
        SemanticRole::ComboBox => Role::ComboBox,
        SemanticRole::Slider => Role::Slider,
        SemanticRole::Progress => Role::ProgressIndicator,
        SemanticRole::SpinButton => Role::SpinButton,
        SemanticRole::ListBox => Role::ListBox,
        SemanticRole::Option => Role::ListBoxOption,
        SemanticRole::TabList => Role::TabList,
        SemanticRole::Tab => Role::Tab,
        SemanticRole::TabPanel => Role::TabPanel,
        SemanticRole::Toolbar => Role::Toolbar,
        SemanticRole::Menu => Role::Menu,
        SemanticRole::MenuBar => Role::MenuBar,
        SemanticRole::MenuItem => Role::MenuItem,
        SemanticRole::MenuItemCheckbox => Role::MenuItemCheckBox,
        SemanticRole::MenuItemRadio => Role::MenuItemRadio,
        SemanticRole::Dialog => Role::Dialog,
        SemanticRole::Tooltip => Role::Tooltip,
        SemanticRole::Separator | SemanticRole::Splitter => Role::Splitter,
        SemanticRole::Tree => Role::Tree,
        SemanticRole::TreeItem => Role::TreeItem,
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedRole(id.clone()));
            Role::Unknown
        }
    }
}

fn map_checked_state(
    checked: SemanticCheckedState,
    id: &SemanticNodeId,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Option<Toggled> {
    match checked {
        SemanticCheckedState::Unchecked => Some(Toggled::False),
        SemanticCheckedState::Checked => Some(Toggled::True),
        SemanticCheckedState::Mixed => Some(Toggled::Mixed),
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedCheckedState(id.clone()));
            None
        }
    }
}

fn map_editable_role(
    semantic: &SemanticNode,
    sensitivity: TextSensitivity,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Role {
    match (semantic.role(), sensitivity) {
        (SemanticRole::EditableText, TextSensitivity::Public) => {
            if semantic.editable_mode() == Some(SemanticEditableMode::Multiline) {
                Role::MultilineTextInput
            } else {
                Role::TextInput
            }
        }
        (SemanticRole::EditableText, TextSensitivity::Secret) => Role::PasswordInput,
        (SemanticRole::ComboBox, TextSensitivity::Public) => Role::EditableComboBox,
        (SemanticRole::SpinButton, TextSensitivity::Public) => Role::SpinButton,
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedRole(semantic.id().clone()));
            Role::Unknown
        }
    }
}

fn map_pressed_state(
    pressed: SemanticPressedState,
    id: &SemanticNodeId,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Option<Toggled> {
    match pressed {
        SemanticPressedState::Unpressed => Some(Toggled::False),
        SemanticPressedState::Pressed => Some(Toggled::True),
        SemanticPressedState::Mixed => Some(Toggled::Mixed),
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedPressedState(id.clone()));
            None
        }
    }
}

fn map_invalid_state(
    invalid: SemanticInvalidState,
    id: &SemanticNodeId,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Option<AccessInvalid> {
    match invalid {
        SemanticInvalidState::Invalid => Some(AccessInvalid::True),
        SemanticInvalidState::Grammar => Some(AccessInvalid::Grammar),
        SemanticInvalidState::Spelling => Some(AccessInvalid::Spelling),
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedInvalidState(id.clone()));
            None
        }
    }
}

fn map_orientation(
    orientation: SemanticOrientation,
    id: &SemanticNodeId,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Option<AccessOrientation> {
    match orientation {
        SemanticOrientation::Horizontal => Some(AccessOrientation::Horizontal),
        SemanticOrientation::Vertical => Some(AccessOrientation::Vertical),
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedOrientation(id.clone()));
            None
        }
    }
}

fn map_popup(
    popup: SemanticPopupKind,
    id: &SemanticNodeId,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Option<HasPopup> {
    match popup {
        SemanticPopupKind::Menu => Some(HasPopup::Menu),
        SemanticPopupKind::ListBox => Some(HasPopup::Listbox),
        SemanticPopupKind::Dialog => Some(HasPopup::Dialog),
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedPopupKind(id.clone()));
            None
        }
    }
}

fn map_autocomplete(
    autocomplete: SemanticAutocomplete,
    id: &SemanticNodeId,
    diagnostics: &mut Vec<AdapterDiagnostic>,
) -> Option<AccessAutoComplete> {
    match autocomplete {
        SemanticAutocomplete::None => None,
        SemanticAutocomplete::Inline => Some(AccessAutoComplete::Inline),
        SemanticAutocomplete::List => Some(AccessAutoComplete::List),
        SemanticAutocomplete::Both => Some(AccessAutoComplete::Both),
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(AdapterDiagnostic::UnsupportedAutocomplete(id.clone()));
            None
        }
    }
}

fn collection_parent<'a>(
    snapshot: &'a SemanticSnapshot,
    semantic: &SemanticNode,
) -> Option<&'a SemanticNode> {
    let mut parent = semantic.parent().cloned();
    for _ in 0..snapshot.nodes().len() {
        let current = snapshot.node(parent.as_ref()?)?;
        let is_collection = match semantic.role() {
            SemanticRole::Option => current.role() == SemanticRole::ListBox,
            SemanticRole::TreeItem => {
                matches!(current.role(), SemanticRole::Tree | SemanticRole::TreeItem)
            }
            _ => false,
        };
        if is_collection {
            return Some(current);
        }
        parent = current.parent().cloned();
    }
    None
}

fn collection_sizes(snapshot: &SemanticSnapshot) -> HashMap<SemanticNodeId, Result<usize, ()>> {
    let mut sizes = HashMap::new();
    for item in snapshot.nodes() {
        let Some(position) = item.collection_position() else {
            continue;
        };
        let Some(known_size) = position.known_size() else {
            continue;
        };
        let Some(parent) = collection_parent(snapshot, item) else {
            continue;
        };
        let size = usize::try_from(known_size).map_err(|_| ());
        match sizes.get_mut(parent.id()) {
            Some(existing) if *existing != size => *existing = Err(()),
            Some(_) => {}
            None => {
                sizes.insert(parent.id().clone(), size);
            }
        }
    }
    sizes
}

fn project_editable_text_run(semantic: &SemanticNode) -> Option<Node> {
    let editable = semantic.editable()?;
    let value = editable.value()?;
    let offsets = editable.caret_offsets()?;
    let lengths = offsets
        .windows(2)
        .map(|pair| u8::try_from(pair[1] - pair[0]).ok())
        .collect::<Option<Vec<_>>>()?;
    let mut node = Node::new(Role::TextRun);
    node.set_value(value);
    node.set_character_lengths(lengths);
    let bounds = semantic.bounds();
    node.set_bounds(Rect {
        x0: f64::from(bounds.x()),
        y0: f64::from(bounds.y()),
        x1: f64::from(bounds.x() + bounds.width()),
        y1: f64::from(bounds.y() + bounds.height()),
    });
    Some(node)
}

fn supports_editable_text_run(semantic: &SemanticNode) -> bool {
    semantic.editable().is_some_and(|editable| {
        editable.sensitivity() == TextSensitivity::Public
            && editable.value().is_some()
            && editable.caret_offsets().is_some_and(|offsets| {
                offsets
                    .windows(2)
                    .all(|pair| u8::try_from(pair[1] - pair[0]).is_ok())
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use runenui_core::{
        __runtime::RuntimeNamespace, EditIntent, EditResolution, EditableContribution,
        EditingSessionPolicy, Element, LogicalSize, NoHostProtocol, SemanticAction,
        SemanticActionData, SemanticAutocomplete, SemanticCheckedState, SemanticCollectionPosition,
        SemanticContribution, SemanticContributionContext, SemanticEditable,
        SemanticHierarchyLevel, SemanticInvalidState, SemanticItem, SemanticKey,
        SemanticNodeContribution, SemanticNumber, SemanticOrientation, SemanticPopupKind,
        SemanticPressedState, SemanticRange, SemanticReference, SemanticRelationship,
        SemanticRelationshipKind, SemanticRole, SemanticSelectionMode, SemanticState, SemanticText,
        SemanticValue, StyleEnvironment, TextDocumentId, TextDocumentRevision,
        TextDocumentSnapshot, TextSelection, TextSensitivity, UiApp, UpdateOutput, View, Widget,
        WidgetActivation, WidgetActivationContext, WidgetActivationOutput, WidgetInvalidation,
        WidgetMeasure, WidgetMeasureInput,
    };
    use runenui_runtime::{AppRuntime, FontFamilyName, GenericFontFamily, SurfaceBuildContext};

    #[derive(Clone, Copy, Debug)]
    struct FixtureAction;

    #[derive(Debug)]
    struct Fixture {
        phase: u8,
    }

    fn typed_semantics(known_size: u64) -> SemanticContribution {
        let combo_key = SemanticKey::from_static("combo").unwrap();
        let listbox_key = SemanticKey::from_static("listbox").unwrap();
        let option_key = SemanticKey::from_static("option").unwrap();
        let error_key = SemanticKey::from_static("error").unwrap();
        let slider_key = SemanticKey::from_static("slider").unwrap();
        let toggle_key = SemanticKey::from_static("toggle").unwrap();
        let dialog_key = SemanticKey::from_static("dialog").unwrap();
        let tree_key = SemanticKey::from_static("tree").unwrap();
        let tree_item_key = SemanticKey::from_static("tree-item").unwrap();

        let combo = SemanticNodeContribution::new(combo_key, SemanticRole::ComboBox)
            .with_name("Choice")
            .with_state(
                SemanticState::ENABLED
                    .with_expanded(true)
                    .with_required(true)
                    .with_invalid(SemanticInvalidState::Invalid),
            )
            .with_popup(SemanticPopupKind::ListBox)
            .with_placeholder("Filter choices")
            .with_autocomplete(SemanticAutocomplete::List)
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Local(listbox_key.clone()),
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                SemanticReference::Local(option_key.clone()),
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ErrorMessage,
                SemanticReference::Local(error_key.clone()),
            ));
        let option = SemanticNodeContribution::new(option_key, SemanticRole::Option)
            .with_name("One")
            .with_state(SemanticState::ENABLED.with_selected(true))
            .with_collection_position(
                SemanticCollectionPosition::new(0, Some(known_size))
                    .unwrap_or_else(|_| unreachable!("controlled collection position is valid")),
            );
        let listbox = SemanticNodeContribution::new(listbox_key, SemanticRole::ListBox)
            .with_name("Choices")
            .with_orientation(SemanticOrientation::Vertical)
            .with_selection_mode(SemanticSelectionMode::Multiple)
            .with_child(option);
        let error = SemanticNodeContribution::new(error_key, SemanticRole::Text)
            .with_name("Choice error node")
            .with_text(SemanticText::plain("Choice error"));

        let minimum = SemanticNumber::new(0.0).unwrap_or_else(|_| unreachable!("finite minimum"));
        let maximum = SemanticNumber::new(10.0).unwrap_or_else(|_| unreachable!("finite maximum"));
        let current = SemanticNumber::new(5.0).unwrap_or_else(|_| unreachable!("finite current"));
        let small_step =
            SemanticNumber::new(1.0).unwrap_or_else(|_| unreachable!("finite small step"));
        let large_step =
            SemanticNumber::new(5.0).unwrap_or_else(|_| unreachable!("finite large step"));
        let range = SemanticRange::new(Some(minimum), Some(maximum), Some(current))
            .and_then(|range| range.with_small_step(small_step))
            .and_then(|range| range.with_large_step(large_step))
            .and_then(|range| range.with_value_text("half"))
            .unwrap_or_else(|_| unreachable!("controlled range is valid"));
        let slider = SemanticNodeContribution::new(slider_key, SemanticRole::Slider)
            .with_name("Volume")
            .with_range(range)
            .with_orientation(SemanticOrientation::Horizontal);

        let toggle = SemanticNodeContribution::new(toggle_key, SemanticRole::Button)
            .with_name("Toggle")
            .with_state(SemanticState::ENABLED.with_pressed(SemanticPressedState::Pressed));
        let dialog = SemanticNodeContribution::new(dialog_key, SemanticRole::Dialog)
            .with_name("Settings dialog")
            .with_state(SemanticState::ENABLED.with_modal(true));

        let tree_item = SemanticNodeContribution::new(tree_item_key, SemanticRole::TreeItem)
            .with_name("Root item")
            .with_collection_position(
                SemanticCollectionPosition::new(0, Some(1))
                    .unwrap_or_else(|_| unreachable!("tree position is valid")),
            )
            .with_hierarchy_level(
                SemanticHierarchyLevel::new(1)
                    .unwrap_or_else(|_| unreachable!("tree level is positive")),
            );
        let tree = SemanticNodeContribution::new(tree_key, SemanticRole::Tree)
            .with_name("Navigation tree")
            .with_child(tree_item);

        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group)
                .with_child(combo)
                .with_child(listbox)
                .with_child(error)
                .with_child(slider)
                .with_child(toggle)
                .with_child(dialog)
                .with_child(tree),
        )
    }

    fn conflicting_collection_semantics() -> SemanticContribution {
        let first = SemanticNodeContribution::new(
            SemanticKey::from_static("conflicting-first").unwrap(),
            SemanticRole::Option,
        )
        .with_name("First conflicting")
        .with_collection_position(
            SemanticCollectionPosition::new(0, Some(1))
                .unwrap_or_else(|_| unreachable!("first collection position is valid")),
        );
        let second = SemanticNodeContribution::new(
            SemanticKey::from_static("conflicting-second").unwrap(),
            SemanticRole::Option,
        )
        .with_name("Second conflicting")
        .with_collection_position(
            SemanticCollectionPosition::new(1, Some(2))
                .unwrap_or_else(|_| unreachable!("second collection position is valid")),
        );
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group).with_child(
                SemanticNodeContribution::new(
                    SemanticKey::from_static("conflicting-listbox").unwrap(),
                    SemanticRole::ListBox,
                )
                .with_name("Conflicting choices")
                .with_child(first)
                .with_child(second),
            ),
        )
    }

    fn multiple_error_message_semantics() -> SemanticContribution {
        let listbox_key = SemanticKey::from_static("error-listbox").unwrap();
        let first_error_key = SemanticKey::from_static("first-error").unwrap();
        let second_error_key = SemanticKey::from_static("second-error").unwrap();
        let combo = SemanticNodeContribution::new(
            SemanticKey::from_static("error-combo").unwrap(),
            SemanticRole::ComboBox,
        )
        .with_name("Invalid choice")
        .with_state(SemanticState::ENABLED.with_invalid(SemanticInvalidState::Invalid))
        .with_popup(SemanticPopupKind::ListBox)
        .with_relationship(SemanticRelationship::new(
            SemanticRelationshipKind::Controls,
            SemanticReference::Local(listbox_key.clone()),
        ))
        .with_relationship(SemanticRelationship::new(
            SemanticRelationshipKind::ErrorMessage,
            SemanticReference::Local(first_error_key.clone()),
        ))
        .with_relationship(SemanticRelationship::new(
            SemanticRelationshipKind::ErrorMessage,
            SemanticReference::Local(second_error_key.clone()),
        ));
        let listbox =
            SemanticNodeContribution::new(listbox_key, SemanticRole::ListBox).with_name("Errors");
        let first_error = SemanticNodeContribution::new(first_error_key, SemanticRole::Text)
            .with_name("First error");
        let second_error = SemanticNodeContribution::new(second_error_key, SemanticRole::Text)
            .with_name("Second error");
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group)
                .with_child(combo)
                .with_child(listbox)
                .with_child(first_error)
                .with_child(second_error),
        )
    }

    impl Widget<FixtureAction> for Fixture {
        type State = ();
        fn create_state(&self) -> Self::State {}
        fn activation(&self, (): &Self::State) -> WidgetActivation {
            WidgetActivation::actionable(true)
        }
        fn activate(
            &mut self,
            (): &mut Self::State,
            context: &mut WidgetActivationContext<FixtureAction>,
        ) -> WidgetActivationOutput<FixtureAction> {
            context.invalidate(WidgetInvalidation::SEMANTICS);
            WidgetActivationOutput::changed_with_action(FixtureAction)
        }
        fn semantics(
            &self,
            (): &Self::State,
            _: SemanticContributionContext,
        ) -> SemanticContribution {
            if self.phase == 5 {
                return conflicting_collection_semantics();
            }
            if self.phase == 6 {
                return multiple_error_message_semantics();
            }
            if self.phase >= 3 {
                return typed_semantics(u64::from(self.phase - 2));
            }
            if self.phase == 2 {
                return SemanticContribution::new(vec![
                    SemanticItem::node(SemanticNodeContribution::new(
                        SemanticKey::from_static("first").unwrap(),
                        SemanticRole::Group,
                    )),
                    SemanticItem::node(SemanticNodeContribution::new(
                        SemanticKey::from_static("second").unwrap(),
                        SemanticRole::Group,
                    )),
                ]);
            }
            let text = SemanticNodeContribution::new(
                SemanticKey::from_static("text").unwrap(),
                SemanticRole::Text,
            )
            .with_name("Plain text")
            .with_text(SemanticText::plain("Plain text"));
            let mut button = SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name("Do it")
                .with_description("Activate this control through the native accessibility tree")
                .with_value(SemanticValue::Text("button value".into()))
                .with_action(SemanticAction::Activate)
                .with_action(SemanticAction::RequestFocus)
                .with_action(SemanticAction::OpenMenu)
                .with_action(SemanticAction::OpenContextMenu);
            if self.phase != 1 {
                button = button
                    .with_relationship(SemanticRelationship::new(
                        SemanticRelationshipKind::LabelledBy,
                        SemanticReference::Local(SemanticKey::from_static("text").unwrap()),
                    ))
                    .with_relationship(SemanticRelationship::new(
                        SemanticRelationshipKind::DescribedBy,
                        SemanticReference::Local(SemanticKey::from_static("text").unwrap()),
                    ))
                    .with_relationship(SemanticRelationship::new(
                        SemanticRelationshipKind::Controls,
                        SemanticReference::Local(SemanticKey::from_static("text").unwrap()),
                    ))
                    .with_child(text);
            }
            if self.phase == 0 {
                button = button
                    .with_child(
                        SemanticNodeContribution::new(
                            SemanticKey::from_static("check").unwrap(),
                            SemanticRole::Checkbox,
                        )
                        .with_name("Check it")
                        .with_state(
                            SemanticState::ENABLED.with_checked(SemanticCheckedState::Mixed),
                        ),
                    )
                    .with_child(
                        SemanticNodeContribution::new(
                            SemanticKey::from_static("diagnostic").unwrap(),
                            SemanticRole::Group,
                        )
                        .with_value(SemanticValue::Integer(7))
                        .with_state(SemanticState::ENABLED.with_disabled(true).with_inert(true)),
                    )
                    .with_child(
                        SemanticNodeContribution::new(
                            SemanticKey::from_static("boolean-value").unwrap(),
                            SemanticRole::Generic,
                        )
                        .with_name("Boolean value")
                        .with_value(SemanticValue::Boolean(true)),
                    );
            }
            SemanticContribution::single(button)
        }
        fn measure(
            &self,
            _: &Self::State,
            _input: runenui_core::WidgetMeasureInput,
        ) -> runenui_core::WidgetMeasure {
            runenui_core::WidgetMeasure::measured(100_u16.into(), 100_u16.into())
        }
        fn paint(
            &self,
            _: &Self::State,
            _: runenui_core::PaintContributionContext,
        ) -> runenui_core::PaintContribution {
            runenui_core::PaintContribution::empty()
        }
    }
    struct FixtureApp;
    impl UiApp for FixtureApp {
        type State = u8;
        type Action = FixtureAction;
        type HostProtocol = NoHostProtocol;
        fn root(phase: &Self::State) -> impl View<Self::Action> {
            Element::new(Fixture { phase: *phase })
                .id("fixture")
                .key("fixture")
                .focusable(true)
        }
        fn update(
            phase: &mut Self::State,
            FixtureAction: Self::Action,
        ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
            *phase = phase.saturating_add(1);
        }
    }

    fn publication(runtime: &mut AppRuntime<FixtureApp>) -> SemanticPublication {
        let style_environment = StyleEnvironment::default();
        runtime
            .publish_surface(&SurfaceBuildContext::tight(
                &style_environment,
                LogicalSize::try_new(100.0, 100.0).unwrap(),
            ))
            .unwrap()
            .semantic_publication()
            .clone()
    }

    #[test]
    fn initial_tree_maps_roles_text_and_custom_action_without_duplicate_label() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(0);
        let publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        let mut activation = adapter.activation_handler();
        assert!(activation.request_initial_tree().is_none());
        let update = adapter.update(&publication);
        assert_eq!(update.mode, UpdateMode::InitialFull);
        assert!(update.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic,
            AdapterDiagnostic::UnsupportedValueType(_)
                | AdapterDiagnostic::UnsupportedInertState(_)
        )));
        let text = update
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.value() == Some("Plain text"))
            .unwrap();
        assert_eq!(text.1.role(), Role::Label);
        assert_eq!(text.1.label(), None);
        let button = update
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Do it"))
            .unwrap();
        assert_eq!(button.1.role(), Role::Button);
        assert_eq!(button.1.value(), Some("button value"));
        assert_eq!(
            button.1.description(),
            Some("Activate this control through the native accessibility tree")
        );
        assert!(button.1.supports_action(Action::Click));
        assert!(button.1.supports_action(Action::Focus));
        assert!(button.1.supports_action(Action::CustomAction));
        assert_eq!(button.1.custom_actions()[0].description, "Open menu".into());
        assert_eq!(button.1.labelled_by(), &[text.0]);
        assert_eq!(button.1.described_by(), &[text.0]);
        assert_eq!(button.1.controls(), &[text.0]);
        let checkbox = update
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Check it"))
            .unwrap();
        assert_eq!(checkbox.1.role(), Role::CheckBox);
        assert_eq!(checkbox.1.toggled(), Some(Toggled::Mixed));
        let disabled_group = update
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::Group)
            .map(|(_, node)| node)
            .unwrap();
        assert!(disabled_group.is_disabled());
        let boolean_value = update
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Boolean value"))
            .map(|(_, node)| node)
            .unwrap();
        assert_eq!(boolean_value.role(), Role::GenericContainer);
        assert_eq!(boolean_value.value(), None);
        assert_eq!(boolean_value.toggled(), None);
        assert_eq!(
            update
                .diagnostics
                .iter()
                .filter(|diagnostic| matches!(
                    diagnostic,
                    AdapterDiagnostic::UnsupportedValueType(_)
                ))
                .count(),
            2
        );
        let activated = activation.request_initial_tree().unwrap();
        assert_eq!(activated.tree_id, update.tree_update.tree_id);
        assert!(activated.tree.is_some());
    }

    #[test]
    fn all_current_roles_map_exactly_and_unsupported_facts_diagnose() {
        let namespace = RuntimeNamespace::__runtime_new();
        let id = namespace.__runtime_semantic_id(0, 1);
        let mut diagnostics = Vec::new();
        assert_eq!(
            map_role(SemanticRole::Generic, &id, &mut diagnostics),
            Role::GenericContainer
        );
        assert_eq!(
            map_role(SemanticRole::Group, &id, &mut diagnostics),
            Role::Group
        );
        assert_eq!(
            map_role(SemanticRole::Text, &id, &mut diagnostics),
            Role::Label
        );
        assert_eq!(
            map_role(SemanticRole::Button, &id, &mut diagnostics),
            Role::Button
        );
        assert_eq!(
            map_role(SemanticRole::EditableText, &id, &mut diagnostics),
            Role::TextInput
        );
        assert_eq!(
            map_role(SemanticRole::Checkbox, &id, &mut diagnostics),
            Role::CheckBox
        );
        assert_eq!(
            map_role(SemanticRole::RadioButton, &id, &mut diagnostics),
            Role::RadioButton
        );
        assert_eq!(
            map_role(SemanticRole::RadioGroup, &id, &mut diagnostics),
            Role::RadioGroup
        );
        assert_eq!(
            map_role(SemanticRole::Switch, &id, &mut diagnostics),
            Role::Switch
        );
        for (semantic, accesskit) in [
            (SemanticRole::Link, Role::Link),
            (SemanticRole::Image, Role::Image),
            (SemanticRole::ComboBox, Role::ComboBox),
            (SemanticRole::Slider, Role::Slider),
            (SemanticRole::Progress, Role::ProgressIndicator),
            (SemanticRole::SpinButton, Role::SpinButton),
            (SemanticRole::ListBox, Role::ListBox),
            (SemanticRole::Option, Role::ListBoxOption),
            (SemanticRole::TabList, Role::TabList),
            (SemanticRole::Tab, Role::Tab),
            (SemanticRole::TabPanel, Role::TabPanel),
            (SemanticRole::Toolbar, Role::Toolbar),
            (SemanticRole::Menu, Role::Menu),
            (SemanticRole::MenuBar, Role::MenuBar),
            (SemanticRole::MenuItem, Role::MenuItem),
            (SemanticRole::MenuItemCheckbox, Role::MenuItemCheckBox),
            (SemanticRole::MenuItemRadio, Role::MenuItemRadio),
            (SemanticRole::Dialog, Role::Dialog),
            (SemanticRole::Tooltip, Role::Tooltip),
            (SemanticRole::Separator, Role::Splitter),
            (SemanticRole::Splitter, Role::Splitter),
            (SemanticRole::Tree, Role::Tree),
            (SemanticRole::TreeItem, Role::TreeItem),
        ] {
            assert_eq!(map_role(semantic, &id, &mut diagnostics), accesskit);
        }
        assert_eq!(
            map_checked_state(SemanticCheckedState::Unchecked, &id, &mut diagnostics),
            Some(Toggled::False)
        );
        assert_eq!(
            map_checked_state(SemanticCheckedState::Checked, &id, &mut diagnostics),
            Some(Toggled::True)
        );
        assert_eq!(
            map_checked_state(SemanticCheckedState::Mixed, &id, &mut diagnostics),
            Some(Toggled::Mixed)
        );
        assert!(diagnostics.is_empty());
        let mut runtime = AppRuntime::<FixtureApp>::mount(0);
        let mut adapter = SemanticAdapter::new();
        let update = adapter.update(&publication(&mut runtime));
        assert!(
            update
                .diagnostics
                .iter()
                .any(|diagnostic| matches!(diagnostic, AdapterDiagnostic::UnsupportedValueType(_)))
        );
    }

    #[test]
    fn typed_standard_semantics_project_exact_native_properties_and_collection_delta() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(3);
        let first_publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        let first = adapter.update(&first_publication);
        assert_eq!(first.mode, UpdateMode::InitialFull);
        assert!(first.diagnostics.is_empty());

        let find = |label: &str| {
            first
                .tree_update
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(label))
                .unwrap_or_else(|| unreachable!("typed semantic node is projected"))
        };
        let (combo_id, combo) = find("Choice");
        let (listbox_id, listbox) = find("Choices");
        let (option_id, option) = find("One");
        let (error_id, _) = find("Choice error node");
        let (_, slider) = find("Volume");
        let (_, toggle) = find("Toggle");
        let (_, dialog) = find("Settings dialog");
        let (_, tree) = find("Navigation tree");
        let (_, tree_item) = find("Root item");

        assert_eq!(combo.role(), Role::ComboBox);
        assert_eq!(combo.is_expanded(), Some(true));
        assert!(combo.is_required());
        assert_eq!(combo.invalid(), Some(AccessInvalid::True));
        assert_eq!(combo.has_popup(), Some(HasPopup::Listbox));
        assert_eq!(combo.placeholder(), Some("Filter choices"));
        assert_eq!(combo.auto_complete(), Some(AccessAutoComplete::List));
        assert_eq!(combo.controls(), &[*listbox_id]);
        assert_eq!(combo.active_descendant(), Some(*option_id));
        assert_eq!(combo.error_message(), Some(*error_id));

        assert_eq!(listbox.role(), Role::ListBox);
        assert_eq!(listbox.orientation(), Some(AccessOrientation::Vertical));
        assert!(listbox.is_multiselectable());
        assert_eq!(listbox.size_of_set(), Some(1));
        assert_eq!(option.role(), Role::ListBoxOption);
        assert_eq!(option.is_selected(), Some(true));
        assert_eq!(option.position_in_set(), Some(0));

        assert_eq!(slider.role(), Role::Slider);
        assert_eq!(slider.min_numeric_value(), Some(0.0));
        assert_eq!(slider.max_numeric_value(), Some(10.0));
        assert_eq!(slider.numeric_value(), Some(5.0));
        assert_eq!(slider.numeric_value_step(), Some(1.0));
        assert_eq!(slider.numeric_value_jump(), Some(5.0));
        assert_eq!(slider.value(), Some("half"));
        assert_eq!(slider.orientation(), Some(AccessOrientation::Horizontal));
        assert_eq!(toggle.toggled(), Some(Toggled::True));
        assert!(dialog.is_modal());
        assert_eq!(tree.size_of_set(), Some(1));
        assert_eq!(tree_item.level(), Some(0));
        assert_eq!(tree_item.position_in_set(), Some(0));

        runtime
            .submit_action(FixtureAction)
            .unwrap_or_else(|_| unreachable!("fixture phase transition is admitted"));
        runtime.pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64));
        let second_publication = publication(&mut runtime);
        let second = adapter.update(&second_publication);
        assert_eq!(second.mode, UpdateMode::Delta);
        assert!(second.diagnostics.is_empty());

        let changed_listbox = second
            .tree_update
            .nodes
            .iter()
            .find(|(id, _)| id == listbox_id)
            .map_or_else(
                || unreachable!("derived collection size change reprojects the native parent"),
                |(_, node)| node,
            );
        assert_eq!(changed_listbox.size_of_set(), Some(2));
        assert_ne!(combo_id, listbox_id);
    }

    #[test]
    fn conflicting_collection_sizes_diagnose_and_withhold_native_set_size() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(5);
        let publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        let update = adapter.update(&publication);

        assert_eq!(update.mode, UpdateMode::InitialFull);
        let listbox = update
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Conflicting choices"))
            .map_or_else(
                || unreachable!("conflicting ListBox is projected"),
                |(_, node)| node,
            );
        assert_eq!(listbox.size_of_set(), None);
        assert_eq!(
            update
                .diagnostics
                .iter()
                .filter(|diagnostic| matches!(
                    diagnostic,
                    AdapterDiagnostic::UnrepresentableCollectionMetadata(_)
                ))
                .count(),
            1
        );
    }

    #[test]
    fn multiple_error_messages_diagnose_and_withhold_native_error_target() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(6);
        let publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        let update = adapter.update(&publication);

        assert_eq!(update.mode, UpdateMode::InitialFull);
        let combo = update
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Invalid choice"))
            .map_or_else(
                || unreachable!("invalid ComboBox is projected"),
                |(_, node)| node,
            );
        assert_eq!(combo.error_message(), None);
        assert_eq!(
            update
                .diagnostics
                .iter()
                .filter(|diagnostic| matches!(
                    diagnostic,
                    AdapterDiagnostic::MultipleErrorMessages(_)
                ))
                .count(),
            1
        );
    }

    #[test]
    fn exhausted_initial_projection_diagnoses_without_publishing_partial_state() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(0);
        let publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        adapter.projection.next_node_id = None;
        let mut activation = adapter.activation_handler();

        let update = adapter.update(&publication);

        assert_eq!(update.mode, UpdateMode::Unchanged);
        assert_eq!(
            update.diagnostics,
            vec![AdapterDiagnostic::NodeIdSpaceExhausted]
        );
        assert!(update.tree_update.nodes.is_empty());
        assert!(update.tree_update.tree.is_none());
        assert!(adapter.projection.current_surface.is_none());
        assert!(adapter.projection.current_revision.is_none());
        assert!(adapter.projection.current_snapshot.is_none());
        assert!(adapter.projection.semantic_to_accesskit.is_empty());
        assert!(adapter.projection.accesskit_to_semantic.is_empty());
        assert!(adapter.projection.current_nodes.is_empty());
        assert!(activation.request_initial_tree().is_none());
    }

    #[test]
    fn node_id_capacity_preflight_reaches_exact_boundary_without_wrap() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(2);
        let publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        adapter.projection.next_node_id = Some(u64::MAX - 2);

        let update = adapter.update(&publication);

        assert_eq!(update.mode, UpdateMode::InitialFull);
        assert!(
            !update
                .diagnostics
                .contains(&AdapterDiagnostic::NodeIdSpaceExhausted)
        );
        assert!(adapter.projection.next_node_id.is_none());
        assert_eq!(adapter.projection.synthetic_root, Some(NodeId(u64::MAX)));
        let mut semantic_ids = adapter
            .projection
            .semantic_to_accesskit
            .values()
            .map(|id| id.0)
            .collect::<Vec<_>>();
        semantic_ids.sort_unstable();
        assert_eq!(semantic_ids, vec![u64::MAX - 2, u64::MAX - 1]);
    }

    #[test]
    fn node_id_exhaustion_preserves_the_last_coherent_projection() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(0);
        let first_publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        adapter.update(&first_publication);
        adapter.projection.next_node_id = None;

        let _ = runtime.submit_action(FixtureAction);
        runtime.pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64));
        let second_publication = publication(&mut runtime);
        let second = adapter.update(&second_publication);
        assert_eq!(second.mode, UpdateMode::Delta);
        assert!(
            !second
                .diagnostics
                .contains(&AdapterDiagnostic::NodeIdSpaceExhausted)
        );
        assert!(adapter.projection.next_node_id.is_none());

        let surface = second_publication.snapshot().surface_id().clone();
        let button = second_publication.snapshot().roots()[0].clone();
        let button_id = adapter.active_id(&surface, &button).unwrap();
        let before_semantic_to_accesskit = adapter.projection.semantic_to_accesskit.clone();
        let before_accesskit_to_semantic = adapter.projection.accesskit_to_semantic.clone();
        let before_retired_semantic = adapter.projection.retired_semantic.clone();
        let before_retired_accesskit = adapter.projection.retired_accesskit.clone();
        let before_synthetic_root = adapter.projection.synthetic_root;
        let before_revision = adapter.projection.current_revision;
        let before_node_ids = adapter
            .projection
            .current_nodes
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut activation = adapter.activation_handler();
        let before_tree = activation.request_initial_tree().unwrap();

        let _ = runtime.submit_action(FixtureAction);
        runtime.pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64));
        let third_publication = publication(&mut runtime);
        let rejected = adapter.update(&third_publication);

        assert_eq!(rejected.mode, UpdateMode::Unchanged);
        assert_eq!(
            rejected.diagnostics,
            vec![AdapterDiagnostic::NodeIdSpaceExhausted]
        );
        assert!(adapter.projection.next_node_id.is_none());
        assert_eq!(
            adapter.projection.semantic_to_accesskit,
            before_semantic_to_accesskit
        );
        assert_eq!(
            adapter.projection.accesskit_to_semantic,
            before_accesskit_to_semantic
        );
        assert_eq!(adapter.projection.retired_semantic, before_retired_semantic);
        assert_eq!(
            adapter.projection.retired_accesskit,
            before_retired_accesskit
        );
        assert_eq!(adapter.projection.synthetic_root, before_synthetic_root);
        assert_eq!(adapter.projection.current_revision, before_revision);
        assert_eq!(
            adapter
                .projection
                .current_nodes
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            before_node_ids
        );
        assert_eq!(rejected.tree_update.tree_id, before_tree.tree_id);
        assert_eq!(rejected.tree_update.focus, before_tree.focus);
        assert!(rejected.tree_update.nodes.is_empty());
        assert!(rejected.tree_update.tree.is_none());
        let after_tree = activation.request_initial_tree().unwrap();
        assert_eq!(after_tree.tree_id, before_tree.tree_id);
        assert_eq!(after_tree.focus, before_tree.focus);
        assert_eq!(
            after_tree
                .nodes
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            before_tree
                .nodes
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>()
        );

        let request = ActionRequest {
            action: Action::Click,
            target_tree: rejected.tree_update.tree_id,
            target_node: button_id,
            data: None,
        };
        let translated = adapter.action_request(&request).unwrap();
        assert_eq!(translated.surface_id(), &surface);
        assert_eq!(translated.target(), &button);
        assert_eq!(translated.action(), &SemanticAction::Activate);
    }

    #[test]
    fn exact_delta_and_skipped_revision_resync_keep_ids_stable_and_retired_ids_unused() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(0);
        let first_publication = publication(&mut runtime);
        let mut adapter = SemanticAdapter::new();
        let first = adapter.update(&first_publication);
        let surface = first_publication.snapshot().surface_id().clone();
        let button = first_publication.snapshot().roots()[0].clone();
        let first_id = adapter.active_id(&surface, &button).unwrap();
        let first_child = first_publication
            .snapshot()
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::Text)
            .map(SemanticNode::id)
            .unwrap()
            .clone();
        let first_child_id = adapter.active_id(&surface, &first_child).unwrap();
        assert_eq!(first.mode, UpdateMode::InitialFull);
        let request = runenui_core::SemanticActionRequest::new(
            surface.clone(),
            button.clone(),
            SemanticAction::Activate,
        );
        runtime.submit_semantic_action(request).unwrap();
        runtime.pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64));
        let second_publication = publication(&mut runtime);
        let delta = adapter.update(&second_publication);
        assert_eq!(delta.mode, UpdateMode::Delta);
        assert!(!delta.tree_update.nodes.is_empty());
        let mounted_target = runtime.index().nodes().first().unwrap().id().clone();
        runtime
            .submit_command(
                mounted_target,
                runenui_core::SemanticCommand::RequestFocus,
                runenui_core::CommandOrigin::programmatic(),
            )
            .unwrap();
        runtime.pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64));
        let third_publication = publication(&mut runtime);
        let mut skipped_adapter = SemanticAdapter::new();
        skipped_adapter.update(&first_publication);
        let skipped = skipped_adapter.update(&third_publication);
        assert_eq!(skipped.mode, UpdateMode::FullResync);
        let third = adapter.update(&third_publication);
        assert_eq!(third.mode, UpdateMode::Delta);
        assert_eq!(adapter.active_id(&surface, &first_child), None);
        let stale_child_request = ActionRequest {
            action: Action::Click,
            target_tree: third.tree_update.tree_id,
            target_node: first_child_id,
            data: None,
        };
        assert_eq!(
            adapter.action_request(&stale_child_request),
            Err(AdapterDiagnostic::RetiredNodeId)
        );
        assert_eq!(adapter.active_id(&surface, &button), Some(first_id));
    }

    #[test]
    fn surface_transition_full_resyncs_and_retires_old_action_ids() {
        let mut first_runtime = AppRuntime::<FixtureApp>::mount(0);
        let first_publication = publication(&mut first_runtime);
        let first_surface = first_publication.snapshot().surface_id().clone();
        let first_semantic = first_publication.snapshot().roots()[0].clone();
        let mut adapter = SemanticAdapter::new();
        let first = adapter.update(&first_publication);
        let first_node_id = adapter.active_id(&first_surface, &first_semantic).unwrap();
        let stale_request = ActionRequest {
            action: Action::Click,
            target_tree: first.tree_update.tree_id,
            target_node: first_node_id,
            data: None,
        };
        assert_eq!(
            adapter.action_request(&stale_request).unwrap().surface_id(),
            &first_surface
        );

        let mut second_runtime = AppRuntime::<FixtureApp>::mount(0);
        let second_publication = publication(&mut second_runtime);
        let second_surface = second_publication.snapshot().surface_id().clone();
        assert_ne!(first_surface, second_surface);
        let second_semantic = second_publication.snapshot().roots()[0].clone();
        let second = adapter.update(&second_publication);
        assert_eq!(second.mode, UpdateMode::FullResync);
        assert_eq!(second.tree_update.tree_id, TreeId::ROOT);
        assert_eq!(adapter.active_id(&first_surface, &first_semantic), None);
        assert_eq!(
            adapter.action_request(&stale_request),
            Err(AdapterDiagnostic::RetiredNodeId)
        );
        let second_node_id = adapter
            .active_id(&second_surface, &second_semantic)
            .unwrap();
        assert_ne!(first_node_id, second_node_id);
    }

    #[test]
    fn surface_transition_retires_synthetic_root_identity() {
        let mut first_runtime = AppRuntime::<FixtureApp>::mount(2);
        let first_publication = publication(&mut first_runtime);
        let mut adapter = SemanticAdapter::new();
        let first = adapter.update(&first_publication);
        assert_eq!(first.mode, UpdateMode::InitialFull);
        let first_root = adapter.projection.synthetic_root.unwrap();

        let mut second_runtime = AppRuntime::<FixtureApp>::mount(2);
        let second_publication = publication(&mut second_runtime);
        assert_ne!(
            first_publication.snapshot().surface_id(),
            second_publication.snapshot().surface_id()
        );
        let second = adapter.update(&second_publication);
        assert_eq!(second.mode, UpdateMode::FullResync);
        let second_root = adapter.projection.synthetic_root.unwrap();
        assert_ne!(first_root, second_root);
        assert!(adapter.projection.retired_accesskit.contains(&first_root));
    }

    #[test]
    fn synthetic_root_retires_when_same_surface_becomes_single_root() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(2);
        let first_publication = publication(&mut runtime);
        let surface = first_publication.snapshot().surface_id().clone();
        let mut adapter = SemanticAdapter::new();
        let first = adapter.update(&first_publication);
        assert_eq!(first.mode, UpdateMode::InitialFull);
        let first_root = adapter.projection.synthetic_root.unwrap();

        let _ = runtime.submit_action(FixtureAction);
        runtime.pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64));
        let second_publication = publication(&mut runtime);
        assert_eq!(second_publication.snapshot().surface_id(), &surface);
        assert_eq!(second_publication.snapshot().roots().len(), 1);
        let second = adapter.update(&second_publication);
        assert_eq!(second.mode, UpdateMode::Delta);
        assert_eq!(adapter.projection.synthetic_root, None);
        assert!(adapter.projection.retired_accesskit.contains(&first_root));
        let current_root = second_publication.snapshot().roots()[0].clone();
        assert_ne!(adapter.active_id(&surface, &current_root), Some(first_root));
    }

    #[test]
    fn action_translation_rejects_wrong_custom_and_foreign_requests() {
        let mut runtime = AppRuntime::<FixtureApp>::mount(0);
        let publication = publication(&mut runtime);
        let surface = publication.snapshot().surface_id().clone();
        let mut adapter = SemanticAdapter::new();
        let update = adapter.update(&publication);
        let button_id = adapter
            .active_id(&surface, &publication.snapshot().roots()[0])
            .unwrap();
        let tree_id = update.tree_update.tree_id;
        for (action, data, expected) in [
            (Action::Click, None, SemanticAction::Activate),
            (Action::Focus, None, SemanticAction::RequestFocus),
            (
                Action::ShowContextMenu,
                None,
                SemanticAction::OpenContextMenu,
            ),
            (
                Action::CustomAction,
                Some(ActionData::CustomAction(OPEN_MENU_CUSTOM_ACTION_ID)),
                SemanticAction::OpenMenu,
            ),
        ] {
            let request = ActionRequest {
                action,
                target_tree: tree_id,
                target_node: button_id,
                data,
            };
            assert_eq!(
                adapter.action_request(&request).unwrap().action(),
                &expected
            );
        }
        let request = ActionRequest {
            action: Action::CustomAction,
            target_tree: tree_id,
            target_node: button_id,
            data: Some(ActionData::CustomAction(99)),
        };
        assert_eq!(
            adapter.action_request(&request),
            Err(AdapterDiagnostic::WrongCustomActionId(99))
        );
        let foreign = ActionRequest {
            target_tree: TreeId(accesskit::Uuid::from_u128(9)),
            ..request
        };
        assert_eq!(
            adapter.action_request(&foreign),
            Err(AdapterDiagnostic::WrongTreeId)
        );
        let unsupported = ActionRequest {
            action: Action::Expand,
            target_tree: tree_id,
            target_node: button_id,
            data: None,
        };
        assert_eq!(
            adapter.action_request(&unsupported),
            Err(AdapterDiagnostic::UnexpectedActionData(Action::Expand))
        );
        let missing_custom_data = ActionRequest {
            action: Action::CustomAction,
            target_tree: tree_id,
            target_node: button_id,
            data: None,
        };
        assert_eq!(
            adapter.action_request(&missing_custom_data),
            Err(AdapterDiagnostic::CustomActionDataMissing)
        );
        let unknown_node = ActionRequest {
            target_node: NodeId(u64::MAX),
            ..missing_custom_data
        };
        assert_eq!(
            adapter.action_request(&unknown_node),
            Err(AdapterDiagnostic::UnknownNodeId)
        );
    }

    #[test]
    fn adapter_tree_ids_are_not_runtime_id_casts() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut runtime = AppRuntime::<FixtureApp>::mount(0);
        let publication = publication(&mut runtime);
        assert_ne!(surface, publication.snapshot().surface_id().clone());
        let mut adapter = SemanticAdapter::new();
        let update = adapter.update(&publication);
        assert_eq!(update.tree_update.tree_id, TreeId::ROOT);
    }

    #[derive(Clone)]
    struct EditableState {
        text: String,
        revision: u64,
        sensitivity: TextSensitivity,
        read_only: bool,
        role: SemanticRole,
    }

    enum EditableAction {
        Edit(EditIntent),
    }

    #[derive(Debug)]
    struct EditableFixture {
        snapshot: TextDocumentSnapshot,
        text: String,
        sensitivity: TextSensitivity,
        read_only: bool,
        role: SemanticRole,
    }

    impl Widget<EditableAction> for EditableFixture {
        type State = ();

        fn create_state(&self) -> Self::State {}

        fn activation(&self, (): &Self::State) -> WidgetActivation {
            WidgetActivation::NONE
        }

        fn editable(&self, (): &Self::State) -> Option<EditableContribution<EditableAction>> {
            EditableContribution::new(
                self.snapshot,
                self.text.clone(),
                TextSelection::collapsed(
                    TextPosition::new(
                        self.snapshot,
                        &self.text,
                        self.text.len(),
                        TextAffinity::Upstream,
                    )
                    .ok()?,
                ),
                self.sensitivity,
                self.read_only,
                false,
                EditingSessionPolicy::PreserveExact,
                EditableAction::Edit,
            )
            .ok()
        }

        fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
            WidgetMeasure::Text {
                content: self.text.clone(),
            }
        }

        fn semantics(
            &self,
            (): &Self::State,
            _: SemanticContributionContext,
        ) -> SemanticContribution {
            let selection = TextSelection::collapsed(
                TextPosition::new(
                    self.snapshot,
                    &self.text,
                    self.text.len(),
                    TextAffinity::Upstream,
                )
                .unwrap(),
            );
            let editable = SemanticEditable::new(
                self.snapshot,
                &self.text,
                selection,
                self.sensitivity,
                self.read_only,
            )
            .unwrap();
            let mut node = SemanticNodeContribution::primary(self.role)
                .with_state(SemanticState::ENABLED.with_read_only(self.read_only))
                .with_editable(editable)
                .with_action(SemanticAction::SetSelection)
                .with_action(SemanticAction::ReplaceSelection);
            if self.role == SemanticRole::ComboBox {
                node = node.with_popup(SemanticPopupKind::ListBox);
            }
            if self.role == SemanticRole::SpinButton {
                node = node.with_range(
                    SemanticRange::new(
                        None,
                        None,
                        Some(
                            SemanticNumber::new(0.0)
                                .unwrap_or_else(|_| unreachable!("spin value is finite")),
                        ),
                    )
                    .unwrap_or_else(|_| unreachable!("spin range is valid")),
                );
            }
            SemanticContribution::single(node)
        }
    }

    struct EditableApp;

    impl UiApp for EditableApp {
        type State = EditableState;
        type Action = EditableAction;
        type HostProtocol = NoHostProtocol;

        fn root(state: &Self::State) -> impl View<Self::Action> {
            Element::new(EditableFixture {
                snapshot: TextDocumentSnapshot::new(
                    TextDocumentId::new(17),
                    TextDocumentRevision::new(state.revision),
                ),
                text: state.text.clone(),
                sensitivity: state.sensitivity,
                read_only: state.read_only,
                role: state.role,
            })
            .id("editable")
            .key("editable")
            .focusable(true)
        }

        fn update(
            state: &mut Self::State,
            EditableAction::Edit(intent): Self::Action,
        ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
            let range = intent.replacement();
            state
                .text
                .replace_range(range.start()..range.end(), intent.replacement_text());
            state.revision += 1;
            UpdateOutput::edit(EditResolution::accepted(
                intent.request().clone(),
                TextDocumentSnapshot::new(
                    TextDocumentId::new(17),
                    TextDocumentRevision::new(state.revision),
                ),
            ))
        }
    }

    fn editable_publication(
        sensitivity: TextSensitivity,
        read_only: bool,
    ) -> (SemanticPublication, SemanticAdapter) {
        editable_publication_for_role(SemanticRole::EditableText, sensitivity, read_only)
    }

    fn editable_publication_for_role(
        role: SemanticRole,
        sensitivity: TextSensitivity,
        read_only: bool,
    ) -> (SemanticPublication, SemanticAdapter) {
        const FONT: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
        ));
        let mut runtime = AppRuntime::<EditableApp>::mount(EditableState {
            text: "ab".to_owned(),
            revision: 0,
            sensitivity,
            read_only,
            role,
        });
        assert!(runtime.register_text_font_bytes(FONT.to_vec()).unwrap() > 0);
        assert!(
            runtime
                .set_text_generic_family_mapping(
                    GenericFontFamily::SansSerif,
                    &[FontFamilyName::new("Cantarell").unwrap()],
                )
                .unwrap()
        );
        let style = StyleEnvironment::default();
        let publication = runtime
            .publish_surface(&SurfaceBuildContext::tight(
                &style,
                LogicalSize::try_new(100.0, 40.0).unwrap(),
            ))
            .unwrap()
            .semantic_publication()
            .clone();
        let mut adapter = SemanticAdapter::new();
        adapter.update(&publication);
        (publication, adapter)
    }

    #[test]
    fn editable_projection_translates_checked_selection_and_redacted_replacement_actions() {
        let (publication, adapter) = editable_publication(TextSensitivity::Public, false);
        let semantic = &publication.snapshot().nodes()[0];
        let parent = adapter
            .active_id(publication.snapshot().surface_id(), semantic.id())
            .unwrap();
        let text_run = adapter.projection.editable_text_runs[semantic.id()];
        let projected = &adapter.projection.current_nodes[&parent];
        assert_eq!(projected.role(), Role::TextInput);
        assert!(projected.supports_action(Action::SetTextSelection));
        assert!(projected.supports_action(Action::ReplaceSelectedText));

        let selection = adapter
            .action_request(&ActionRequest {
                action: Action::SetTextSelection,
                target_tree: TreeId::ROOT,
                target_node: parent,
                data: Some(ActionData::SetTextSelection(AccessTextSelection {
                    anchor: AccessTextPosition {
                        node: text_run,
                        character_index: 1,
                    },
                    focus: AccessTextPosition {
                        node: text_run,
                        character_index: 1,
                    },
                })),
            })
            .unwrap();
        let Some(SemanticActionData::Selection(selection)) = selection.data() else {
            unreachable!("selection action retains checked neutral data");
        };
        assert_eq!(selection.active().byte_offset(), 1);

        let replacement = adapter
            .action_request(&ActionRequest {
                action: Action::ReplaceSelectedText,
                target_tree: TreeId::ROOT,
                target_node: parent,
                data: Some(ActionData::Value("sensitive replacement".into())),
            })
            .unwrap();
        assert_eq!(replacement.action(), &SemanticAction::ReplaceSelection);
        assert!(!format!("{replacement:?}").contains("sensitive replacement"));
    }

    #[test]
    fn composite_editable_roles_project_exact_native_roles() {
        for (semantic_role, native_role) in [
            (SemanticRole::ComboBox, Role::EditableComboBox),
            (SemanticRole::SpinButton, Role::SpinButton),
        ] {
            let (publication, adapter) =
                editable_publication_for_role(semantic_role, TextSensitivity::Public, false);
            let semantic = &publication.snapshot().nodes()[0];
            assert_eq!(semantic.role(), semantic_role);
            let parent = adapter
                .active_id(publication.snapshot().surface_id(), semantic.id())
                .unwrap_or_else(|| unreachable!("editable semantic node is projected"));
            assert_eq!(adapter.projection.current_nodes[&parent].role(), native_role);
        }
    }

    #[test]
    fn secret_and_read_only_editable_projection_cannot_gain_native_disclosure_or_mutation() {
        let (secret_publication, secret_adapter) =
            editable_publication(TextSensitivity::Secret, false);
        let secret = &secret_publication.snapshot().nodes()[0];
        let secret_parent = secret_adapter
            .active_id(secret_publication.snapshot().surface_id(), secret.id())
            .unwrap();
        let projected_secret = &secret_adapter.projection.current_nodes[&secret_parent];
        assert_eq!(projected_secret.role(), Role::PasswordInput);
        assert_eq!(projected_secret.value(), None);
        assert!(!projected_secret.supports_action(Action::SetTextSelection));
        assert!(projected_secret.supports_action(Action::ReplaceSelectedText));
        assert!(
            !secret_adapter
                .projection
                .editable_text_runs
                .contains_key(secret.id())
        );

        let (read_only_publication, read_only_adapter) =
            editable_publication(TextSensitivity::Public, true);
        let read_only = &read_only_publication.snapshot().nodes()[0];
        let read_only_parent = read_only_adapter
            .active_id(
                read_only_publication.snapshot().surface_id(),
                read_only.id(),
            )
            .unwrap();
        let projected_read_only = &read_only_adapter.projection.current_nodes[&read_only_parent];
        assert!(projected_read_only.is_read_only());
        assert!(projected_read_only.supports_action(Action::SetTextSelection));
        assert!(!projected_read_only.supports_action(Action::ReplaceSelectedText));
    }
}
