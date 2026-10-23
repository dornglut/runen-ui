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
    SemanticInvalidState, SemanticNodeId, SemanticNumber, SemanticOrientation, SemanticPopupKind,
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
                SemanticAction::Increment => node.add_action(Action::Increment),
                SemanticAction::Decrement => node.add_action(Action::Decrement),
                SemanticAction::SetValue => node.add_action(Action::SetValue),
                SemanticAction::Expand => node.add_action(Action::Expand),
                SemanticAction::Collapse => node.add_action(Action::Collapse),
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
        if request.action == Action::SetValue {
            let Some(ActionData::NumericValue(value)) = request.data.as_ref() else {
                return Err(AdapterDiagnostic::UnexpectedActionData(request.action));
            };
            if !node.supported_actions().contains(&SemanticAction::SetValue) {
                return Err(AdapterDiagnostic::UnsupportedSemanticAction {
                    target: semantic.clone(),
                    action: SemanticAction::SetValue,
                });
            }
            let value = SemanticNumber::new(*value)
                .map_err(|_| AdapterDiagnostic::UnexpectedActionData(request.action))?;
            return Ok(runenui_core::SemanticActionRequest::set_value(
                snapshot.surface_id().clone(),
                semantic.clone(),
                value,
            ));
        }
        let action = match request.action {
            Action::Click => SemanticAction::Activate,
            Action::Focus => SemanticAction::RequestFocus,
            Action::ShowContextMenu => SemanticAction::OpenContextMenu,
            Action::Increment => SemanticAction::Increment,
            Action::Decrement => SemanticAction::Decrement,
            Action::Expand => SemanticAction::Expand,
            Action::Collapse => SemanticAction::Collapse,
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
        SemanticRole::ScrollBar => Role::ScrollBar,
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
mod tests;
