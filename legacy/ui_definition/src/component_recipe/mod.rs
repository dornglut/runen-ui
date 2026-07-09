//! Runtime-neutral component, widget, and surface recipe contracts.

use crate::{
    UiDefinitionDiagnostic, UiDefinitionDiagnosticSeverity, UiNodeDefinition, UiNodeId,
    UiSourceLocation, identity::AuthoredId,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ui_theme::{ThemeTokenFamily, ThemeTokenId};

pub type UiRecipeId = AuthoredId;
pub type UiRecipeSourcePackageId = AuthoredId;
pub type UiRecipeSlotId = AuthoredId;
pub type UiRecipeTargetProfileId = AuthoredId;
pub type UiRecipeStateVariantId = AuthoredId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiRecipeKind {
    Component,
    Widget,
    Surface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiRecipeActivationMode {
    Preview,
    Activate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiRecipeActivationImpact {
    None,
    BlocksActivation,
    PreviewOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiRecipeDiagnosticDomain {
    UiDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiRecipeAccessibilityRole {
    Button,
    Group,
    Label,
    List,
    ListItem,
    Navigation,
    Region,
    Slider,
    Surface,
    TextInput,
    Toggle,
    Toolbar,
    Tree,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiRecipeAccessibilityLabel {
    Static(String),
    FromBinding(AuthoredId),
    FromSlot(UiRecipeSlotId),
}

impl UiRecipeAccessibilityLabel {
    fn is_empty(&self) -> bool {
        matches!(self, Self::Static(value) if value.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRecipeAccessibilityDefinition {
    pub role: UiRecipeAccessibilityRole,
    pub label: UiRecipeAccessibilityLabel,
    pub required_semantics: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiRecipeLayoutBehavior {
    #[serde(default)]
    pub min_width: Option<f32>,
    #[serde(default)]
    pub min_height: Option<f32>,
    pub resizable: bool,
}

impl UiRecipeLayoutBehavior {
    pub fn fixed() -> Self {
        Self {
            min_width: None,
            min_height: None,
            resizable: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRecipeFocusNavigation {
    pub focusable: bool,
    #[serde(default)]
    pub tab_order: Option<i32>,
    pub directional: bool,
}

impl UiRecipeFocusNavigation {
    pub fn passive() -> Self {
        Self {
            focusable: false,
            tab_order: None,
            directional: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRecipeTokenRequirement {
    pub family: ThemeTokenFamily,
    #[serde(default)]
    pub token: Option<ThemeTokenId>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRecipeSlotDefinition {
    pub id: UiRecipeSlotId,
    pub accepted_kinds: BTreeSet<UiRecipeKind>,
    pub required: bool,
    pub mount_node: UiNodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiRecipeDeclaration {
    pub id: UiRecipeId,
    pub kind: UiRecipeKind,
    pub label: String,
    pub category: String,
    pub source_package: UiRecipeSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiRecipeTargetProfileId>,
    pub template: UiNodeDefinition,
    #[serde(default)]
    pub slots: Vec<UiRecipeSlotDefinition>,
    #[serde(default)]
    pub token_requirements: Vec<UiRecipeTokenRequirement>,
    #[serde(default)]
    pub state_variants: BTreeSet<UiRecipeStateVariantId>,
    #[serde(default)]
    pub accessibility: Option<UiRecipeAccessibilityDefinition>,
    pub layout: UiRecipeLayoutBehavior,
    pub focus_navigation: UiRecipeFocusNavigation,
    pub preview_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiRecipeLibrary {
    #[serde(default)]
    pub declarations: Vec<UiRecipeDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRecipeChildRecipeRef {
    pub recipe: UiRecipeId,
    #[serde(default)]
    pub slot_children: BTreeMap<UiRecipeSlotId, Vec<UiRecipeChildRecipeRef>>,
}

impl UiRecipeChildRecipeRef {
    pub fn new(recipe: impl Into<UiRecipeId>) -> Self {
        Self {
            recipe: recipe.into(),
            slot_children: BTreeMap::new(),
        }
    }

    pub fn with_slot_child(
        mut self,
        slot: impl Into<UiRecipeSlotId>,
        child: impl Into<UiRecipeId>,
    ) -> Self {
        self.slot_children
            .entry(slot.into())
            .or_default()
            .push(UiRecipeChildRecipeRef::new(child));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRecipeExpansionRequest {
    pub recipe: UiRecipeId,
    pub target_profile: UiRecipeTargetProfileId,
    pub activation: UiRecipeActivationMode,
    #[serde(default)]
    pub slot_children: BTreeMap<UiRecipeSlotId, Vec<UiRecipeChildRecipeRef>>,
}

impl UiRecipeExpansionRequest {
    pub fn activate(
        recipe: impl Into<UiRecipeId>,
        target_profile: impl Into<UiRecipeTargetProfileId>,
    ) -> Self {
        Self {
            recipe: recipe.into(),
            target_profile: target_profile.into(),
            activation: UiRecipeActivationMode::Activate,
            slot_children: BTreeMap::new(),
        }
    }

    pub fn preview(
        recipe: impl Into<UiRecipeId>,
        target_profile: impl Into<UiRecipeTargetProfileId>,
    ) -> Self {
        Self {
            recipe: recipe.into(),
            target_profile: target_profile.into(),
            activation: UiRecipeActivationMode::Preview,
            slot_children: BTreeMap::new(),
        }
    }

    pub fn with_slot_child(
        mut self,
        slot: impl Into<UiRecipeSlotId>,
        child: impl Into<UiRecipeId>,
    ) -> Self {
        self.slot_children
            .entry(slot.into())
            .or_default()
            .push(UiRecipeChildRecipeRef::new(child));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRecipeDiagnostic {
    pub severity: UiDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub recipe: Option<UiRecipeId>,
    #[serde(default)]
    pub slot_path: Vec<UiRecipeSlotId>,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub target_profile: Option<UiRecipeTargetProfileId>,
    pub owning_domain: UiRecipeDiagnosticDomain,
    #[serde(default)]
    pub source_package: Option<UiRecipeSourcePackageId>,
    #[serde(default)]
    pub winning_source: Option<UiRecipeSourcePackageId>,
    #[serde(default)]
    pub losing_sources: Vec<UiRecipeSourcePackageId>,
    pub activation_impact: UiRecipeActivationImpact,
    pub suggested_fix: String,
}

impl UiRecipeDiagnostic {
    fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            severity: UiDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            recipe: None,
            slot_path: Vec::new(),
            source_location: None,
            target_profile: None,
            owning_domain: UiRecipeDiagnosticDomain::UiDefinition,
            source_package: None,
            winning_source: None,
            losing_sources: Vec::new(),
            activation_impact: UiRecipeActivationImpact::BlocksActivation,
            suggested_fix: suggested_fix.into(),
        }
    }

    fn for_declaration(mut self, declaration: &UiRecipeDeclaration) -> Self {
        self.recipe = Some(declaration.id.clone());
        self.source_location = declaration.source_location.clone();
        self.source_package = Some(declaration.source_package.clone());
        self
    }

    fn with_recipe(mut self, recipe: UiRecipeId) -> Self {
        self.recipe = Some(recipe);
        self
    }

    fn with_target_profile(mut self, target_profile: UiRecipeTargetProfileId) -> Self {
        self.target_profile = Some(target_profile);
        self
    }

    fn with_slot(mut self, slot: UiRecipeSlotId) -> Self {
        self.slot_path.push(slot);
        self
    }

    fn preview_only(mut self) -> Self {
        self.activation_impact = UiRecipeActivationImpact::PreviewOnly;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiRecipeExpansionReport {
    #[serde(default)]
    pub root: Option<UiNodeDefinition>,
    #[serde(default)]
    pub diagnostics: Vec<UiRecipeDiagnostic>,
}

impl UiRecipeExpansionReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error)
    }

    pub fn as_definition_diagnostics(&self) -> Vec<UiDefinitionDiagnostic> {
        self.diagnostics
            .iter()
            .map(|diagnostic| UiDefinitionDiagnostic {
                severity: diagnostic.severity,
                code: diagnostic.code.clone(),
                message: diagnostic.message.clone(),
                path: None,
            })
            .collect()
    }
}

pub fn expand_ui_recipe(
    library: &UiRecipeLibrary,
    request: &UiRecipeExpansionRequest,
) -> UiRecipeExpansionReport {
    let mut diagnostics = Vec::new();
    let recipes = index_recipes(library, &mut diagnostics);

    let mut expansion = RecipeExpansion {
        recipes,
        request,
        diagnostics,
        stack: Vec::new(),
    };

    let root = expansion.expand_recipe(&request.recipe, None, &request.slot_children);
    let diagnostics = expansion.diagnostics;
    let has_errors = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error);

    UiRecipeExpansionReport {
        root: if has_errors { None } else { root },
        diagnostics,
    }
}

fn index_recipes<'a>(
    library: &'a UiRecipeLibrary,
    diagnostics: &mut Vec<UiRecipeDiagnostic>,
) -> BTreeMap<UiRecipeId, &'a UiRecipeDeclaration> {
    let mut recipes: BTreeMap<UiRecipeId, &'a UiRecipeDeclaration> = BTreeMap::new();

    for declaration in &library.declarations {
        if let Some(existing) = recipes.get(&declaration.id) {
            diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.duplicate_id",
                    format!("Recipe id '{}' is declared more than once.", declaration.id),
                    "Keep one recipe declaration for each stable recipe id.",
                )
                .for_declaration(declaration)
                .with_winning_source(existing.source_package.clone())
                .with_losing_source(declaration.source_package.clone()),
            );
        } else {
            recipes.insert(declaration.id.clone(), declaration);
        }
    }

    recipes
}

impl UiRecipeDiagnostic {
    fn with_winning_source(mut self, source: UiRecipeSourcePackageId) -> Self {
        self.winning_source = Some(source);
        self
    }

    fn with_losing_source(mut self, source: UiRecipeSourcePackageId) -> Self {
        self.losing_sources.push(source);
        self
    }
}

struct RecipeExpansion<'a> {
    recipes: BTreeMap<UiRecipeId, &'a UiRecipeDeclaration>,
    request: &'a UiRecipeExpansionRequest,
    diagnostics: Vec<UiRecipeDiagnostic>,
    stack: Vec<UiRecipeId>,
}

impl RecipeExpansion<'_> {
    fn expand_recipe(
        &mut self,
        recipe_id: &UiRecipeId,
        parent_slot: Option<&UiRecipeSlotDefinition>,
        slot_children: &BTreeMap<UiRecipeSlotId, Vec<UiRecipeChildRecipeRef>>,
    ) -> Option<UiNodeDefinition> {
        if self.stack.contains(recipe_id) {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.reference.cycle",
                    format!(
                        "Recipe '{}' creates a recursive recipe expansion.",
                        recipe_id
                    ),
                    "Break the recipe cycle by moving repeated structure into a slot-owned child.",
                )
                .with_recipe(recipe_id.clone())
                .with_target_profile(self.request.target_profile.clone()),
            );
            return None;
        }

        let Some(declaration) = self.recipes.get(recipe_id).copied() else {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.reference.unknown",
                    format!(
                        "Recipe '{}' does not exist in the recipe library.",
                        recipe_id
                    ),
                    "Add the recipe declaration or remove the reference.",
                )
                .with_recipe(recipe_id.clone())
                .with_target_profile(self.request.target_profile.clone()),
            );
            return None;
        };

        if let Some(slot) = parent_slot {
            self.validate_child_kind(declaration, slot);
        }

        self.validate_declaration(declaration);

        self.stack.push(recipe_id.clone());
        let mut root = declaration.template.clone();
        self.expand_slots(declaration, &mut root, slot_children);
        self.stack.pop();

        Some(root)
    }

    fn validate_child_kind(
        &mut self,
        declaration: &UiRecipeDeclaration,
        slot: &UiRecipeSlotDefinition,
    ) {
        if !slot.accepted_kinds.contains(&declaration.kind) {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.slot.child_kind_unsupported",
                    format!(
                        "Recipe '{}' of kind '{:?}' cannot be mounted in slot '{}'.",
                        declaration.id, declaration.kind, slot.id
                    ),
                    "Change the child recipe kind or add the kind to the slot contract.",
                )
                .for_declaration(declaration)
                .with_target_profile(self.request.target_profile.clone())
                .with_slot(slot.id.clone()),
            );
        }
    }

    fn validate_declaration(&mut self, declaration: &UiRecipeDeclaration) {
        self.validate_target_profile(declaration);
        self.validate_preview_activation(declaration);
        self.validate_slot_definitions(declaration);
        self.validate_token_requirements(declaration);
        self.validate_accessibility(declaration);
        self.validate_layout(declaration);
        self.validate_focus_navigation(declaration);
    }

    fn validate_target_profile(&mut self, declaration: &UiRecipeDeclaration) {
        if !declaration.target_profiles.is_empty()
            && !declaration
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.target_profile.unsupported",
                    format!(
                        "Recipe '{}' does not support target profile '{}'.",
                        declaration.id, self.request.target_profile
                    ),
                    "Add target-profile support to the recipe or choose a compatible recipe.",
                )
                .for_declaration(declaration)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_preview_activation(&mut self, declaration: &UiRecipeDeclaration) {
        if declaration.preview_only && self.request.activation == UiRecipeActivationMode::Activate {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.preview_only_activation",
                    format!(
                        "Recipe '{}' is preview-only and cannot be activated.",
                        declaration.id
                    ),
                    "Use preview expansion or remove the preview-only flag before activation.",
                )
                .for_declaration(declaration)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }

    fn validate_slot_definitions(&mut self, declaration: &UiRecipeDeclaration) {
        let mut slot_ids = BTreeSet::new();
        for slot in &declaration.slots {
            if !slot_ids.insert(slot.id.clone()) {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.slot.duplicate_id",
                        format!(
                            "Recipe '{}' declares slot '{}' more than once.",
                            declaration.id, slot.id
                        ),
                        "Keep one slot declaration for each stable slot id in a recipe.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone())
                    .with_slot(slot.id.clone()),
                );
            }

            if slot.accepted_kinds.is_empty() {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.slot.accepted_kinds_missing",
                        format!(
                            "Recipe '{}' slot '{}' does not accept any child recipe kinds.",
                            declaration.id, slot.id
                        ),
                        "Add at least one accepted recipe kind to the slot contract.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone())
                    .with_slot(slot.id.clone()),
                );
            }
        }
    }

    fn validate_token_requirements(&mut self, declaration: &UiRecipeDeclaration) {
        let mut required_families = BTreeSet::new();
        for requirement in &declaration.token_requirements {
            if requirement.required && requirement.token.is_none() {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.token_family.missing",
                        format!(
                            "Recipe '{}' requires a {:?} token but does not reference one.",
                            declaration.id, requirement.family
                        ),
                        "Reference a default token id for each required token family.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            if requirement.required && !required_families.insert(requirement.family) {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.token_family.duplicate_requirement",
                        format!(
                            "Recipe '{}' repeats required {:?} token-family metadata.",
                            declaration.id, requirement.family
                        ),
                        "Keep one required token-family declaration per recipe and family.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }

    fn validate_accessibility(&mut self, declaration: &UiRecipeDeclaration) {
        let Some(accessibility) = &declaration.accessibility else {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.accessibility.missing",
                    format!(
                        "Recipe '{}' does not define accessibility metadata.",
                        declaration.id
                    ),
                    "Add role, label, and required semantic metadata to the recipe.",
                )
                .for_declaration(declaration)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        if accessibility.label.is_empty() || accessibility.required_semantics.is_empty() {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.accessibility.missing",
                    format!(
                        "Recipe '{}' is missing required accessibility label or semantics.",
                        declaration.id
                    ),
                    "Add a non-empty label strategy and at least one required semantic.",
                )
                .for_declaration(declaration)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }

        for semantic in &accessibility.required_semantics {
            if semantic.trim().is_empty() {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.accessibility.missing",
                        format!(
                            "Recipe '{}' contains an empty accessibility semantic.",
                            declaration.id
                        ),
                        "Remove empty semantic entries or replace them with stable semantic names.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }

    fn validate_layout(&mut self, declaration: &UiRecipeDeclaration) {
        for (name, value) in [
            ("min_width", declaration.layout.min_width),
            ("min_height", declaration.layout.min_height),
        ] {
            if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.layout.invalid",
                        format!(
                            "Recipe '{}' has invalid layout field '{}'.",
                            declaration.id, name
                        ),
                        "Use finite non-negative layout constraints.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }

    fn validate_focus_navigation(&mut self, declaration: &UiRecipeDeclaration) {
        if declaration
            .focus_navigation
            .tab_order
            .is_some_and(|tab_order| tab_order < 0)
        {
            self.diagnostics.push(
                UiRecipeDiagnostic::error(
                    "ui.recipe.focus_navigation.invalid",
                    format!(
                        "Recipe '{}' has a negative focus tab order.",
                        declaration.id
                    ),
                    "Use a non-negative tab order or omit tab order for passive recipes.",
                )
                .for_declaration(declaration)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn expand_slots(
        &mut self,
        declaration: &UiRecipeDeclaration,
        root: &mut UiNodeDefinition,
        slot_children: &BTreeMap<UiRecipeSlotId, Vec<UiRecipeChildRecipeRef>>,
    ) {
        let slots_by_id: BTreeMap<_, _> = declaration
            .slots
            .iter()
            .map(|slot| (slot.id.clone(), slot))
            .collect();

        for requested_slot in slot_children.keys() {
            if !slots_by_id.contains_key(requested_slot) {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.slot.unknown",
                        format!(
                            "Recipe '{}' does not declare requested slot '{}'.",
                            declaration.id, requested_slot
                        ),
                        "Remove the slot assignment or add the slot to the recipe declaration.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone())
                    .with_slot(requested_slot.clone()),
                );
            }
        }

        for slot in &declaration.slots {
            let children = slot_children.get(&slot.id);
            if slot.required && children.is_none_or(Vec::is_empty) {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.slot.required_missing",
                        format!(
                            "Recipe '{}' required slot '{}' has no child recipe.",
                            declaration.id, slot.id
                        ),
                        "Provide a child recipe for the required slot or make the slot optional.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone())
                    .with_slot(slot.id.clone()),
                );
                continue;
            }

            let Some(children) = children else {
                continue;
            };

            let Some(mount) = find_node_mut(root, &slot.mount_node) else {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.slot.mount_invalid",
                        format!(
                            "Recipe '{}' slot '{}' mount node '{}' does not exist.",
                            declaration.id, slot.id, slot.mount_node
                        ),
                        "Point the slot mount at a container node in the recipe template.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone())
                    .with_slot(slot.id.clone()),
                );
                continue;
            };

            let Some(mounted_children) = mount.children_mut() else {
                self.diagnostics.push(
                    UiRecipeDiagnostic::error(
                        "ui.recipe.slot.mount_invalid",
                        format!(
                            "Recipe '{}' slot '{}' mount node '{}' cannot contain children.",
                            declaration.id, slot.id, slot.mount_node
                        ),
                        "Point the slot mount at a container node in the recipe template.",
                    )
                    .for_declaration(declaration)
                    .with_target_profile(self.request.target_profile.clone())
                    .with_slot(slot.id.clone()),
                );
                continue;
            };

            for child in children {
                if let Some(child_root) =
                    self.expand_recipe(&child.recipe, Some(slot), &child.slot_children)
                {
                    mounted_children.push(child_root);
                }
            }
        }
    }
}

fn find_node_mut<'a>(
    node: &'a mut UiNodeDefinition,
    id: &UiNodeId,
) -> Option<&'a mut UiNodeDefinition> {
    if node.id() == id {
        return Some(node);
    }

    for child in node.children_mut()? {
        if let Some(found) = find_node_mut(child, id) {
            return Some(found);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiValueBinding;

    fn id(value: &str) -> AuthoredId {
        AuthoredId::from(value)
    }

    fn profiles(values: &[&str]) -> BTreeSet<UiRecipeTargetProfileId> {
        values.iter().copied().map(AuthoredId::from).collect()
    }

    fn kinds(values: &[UiRecipeKind]) -> BTreeSet<UiRecipeKind> {
        values.iter().copied().collect()
    }

    fn semantics(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn label_node(id_value: &str, label: &str) -> UiNodeDefinition {
        UiNodeDefinition::Label {
            id: id(id_value),
            label: UiValueBinding::static_text(label),
            availability: None,
        }
    }

    fn recipe(
        recipe_id: &str,
        kind: UiRecipeKind,
        template: UiNodeDefinition,
    ) -> UiRecipeDeclaration {
        UiRecipeDeclaration {
            id: id(recipe_id),
            kind,
            label: recipe_id.to_string(),
            category: "test".to_string(),
            source_package: id("test.package"),
            source_location: None,
            target_profiles: profiles(&["editor.workbench", "game.runtime"]),
            template,
            slots: Vec::new(),
            token_requirements: vec![UiRecipeTokenRequirement {
                family: ThemeTokenFamily::Color,
                token: Some(ThemeTokenId::from("color.surface")),
                required: true,
            }],
            state_variants: BTreeSet::new(),
            accessibility: Some(UiRecipeAccessibilityDefinition {
                role: UiRecipeAccessibilityRole::Region,
                label: UiRecipeAccessibilityLabel::Static(recipe_id.to_string()),
                required_semantics: semantics(&["name"]),
            }),
            layout: UiRecipeLayoutBehavior::fixed(),
            focus_navigation: UiRecipeFocusNavigation::passive(),
            preview_only: false,
        }
    }

    fn surface_recipe() -> UiRecipeDeclaration {
        let mut declaration = recipe(
            "inspector.surface",
            UiRecipeKind::Surface,
            UiNodeDefinition::Column {
                id: id("inspector-root"),
                children: vec![label_node("title", "Inspector")],
            },
        );
        declaration.slots = vec![UiRecipeSlotDefinition {
            id: id("body"),
            accepted_kinds: kinds(&[UiRecipeKind::Component, UiRecipeKind::Widget]),
            required: true,
            mount_node: id("inspector-root"),
        }];
        declaration
    }

    fn component_recipe() -> UiRecipeDeclaration {
        recipe(
            "stats.component",
            UiRecipeKind::Component,
            UiNodeDefinition::Panel {
                id: id("stats-root"),
                children: vec![label_node("stats-label", "Stats")],
                availability: None,
            },
        )
    }

    fn library(declarations: Vec<UiRecipeDeclaration>) -> UiRecipeLibrary {
        UiRecipeLibrary { declarations }
    }

    fn codes(report: &UiRecipeExpansionReport) -> BTreeSet<&str> {
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    #[test]
    fn component_recipe_expands_deterministically_and_preserves_ids() {
        let library = library(vec![surface_recipe(), component_recipe()]);
        let request = UiRecipeExpansionRequest::activate("inspector.surface", "editor.workbench")
            .with_slot_child("body", "stats.component");

        let first = expand_ui_recipe(&library, &request);
        let second = expand_ui_recipe(&library, &request);

        assert!(!first.has_errors(), "{:?}", first.diagnostics);
        assert_eq!(first.root, second.root);

        let root = first.root.expect("valid recipe should expand");
        assert_eq!(root.id().as_str(), "inspector-root");
        assert_eq!(
            root.children()
                .iter()
                .map(|child| child.id().as_str())
                .collect::<Vec<_>>(),
            vec!["title", "stats-root"]
        );
    }

    #[test]
    fn component_recipe_rejects_invalid_slot_child_kind() {
        let mut child = component_recipe();
        child.kind = UiRecipeKind::Surface;
        let library = library(vec![surface_recipe(), child]);
        let request = UiRecipeExpansionRequest::activate("inspector.surface", "editor.workbench")
            .with_slot_child("body", "stats.component");

        let report = expand_ui_recipe(&library, &request);

        assert!(report.root.is_none());
        assert!(codes(&report).contains("ui.recipe.slot.child_kind_unsupported"));
    }

    #[test]
    fn component_recipe_rejects_missing_required_token_family() {
        let mut declaration = component_recipe();
        declaration.token_requirements = vec![UiRecipeTokenRequirement {
            family: ThemeTokenFamily::Color,
            token: None,
            required: true,
        }];
        let library = library(vec![declaration]);
        let request = UiRecipeExpansionRequest::activate("stats.component", "editor.workbench");

        let report = expand_ui_recipe(&library, &request);

        assert!(report.root.is_none());
        assert!(codes(&report).contains("ui.recipe.token_family.missing"));
    }

    #[test]
    fn component_recipe_rejects_missing_accessibility_metadata() {
        let mut declaration = component_recipe();
        declaration.accessibility = None;
        let library = library(vec![declaration]);
        let request = UiRecipeExpansionRequest::activate("stats.component", "editor.workbench");

        let report = expand_ui_recipe(&library, &request);

        assert!(report.root.is_none());
        assert!(codes(&report).contains("ui.recipe.accessibility.missing"));
    }

    #[test]
    fn component_recipe_rejects_unsupported_target_profile() {
        let library = library(vec![component_recipe()]);
        let request = UiRecipeExpansionRequest::activate("stats.component", "console.runtime");

        let report = expand_ui_recipe(&library, &request);

        assert!(report.root.is_none());
        assert!(codes(&report).contains("ui.recipe.target_profile.unsupported"));
    }

    #[test]
    fn component_recipe_rejects_preview_only_activation() {
        let mut declaration = component_recipe();
        declaration.preview_only = true;
        let library = library(vec![declaration]);
        let request = UiRecipeExpansionRequest::activate("stats.component", "editor.workbench");

        let report = expand_ui_recipe(&library, &request);

        assert!(report.root.is_none());
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "ui.recipe.preview_only_activation")
            .expect("preview-only activation diagnostic");
        assert_eq!(
            diagnostic.activation_impact,
            UiRecipeActivationImpact::PreviewOnly
        );
    }

    #[test]
    fn component_recipe_rejects_duplicate_recipe_ids() {
        let library = library(vec![component_recipe(), component_recipe()]);
        let request = UiRecipeExpansionRequest::activate("stats.component", "editor.workbench");

        let report = expand_ui_recipe(&library, &request);

        assert!(report.root.is_none());
        assert!(codes(&report).contains("ui.recipe.duplicate_id"));
    }

    #[test]
    fn component_recipe_supports_editor_and_runtime_target_profiles() {
        let library = library(vec![component_recipe()]);

        let editor = expand_ui_recipe(
            &library,
            &UiRecipeExpansionRequest::preview("stats.component", "editor.workbench"),
        );
        let runtime = expand_ui_recipe(
            &library,
            &UiRecipeExpansionRequest::preview("stats.component", "game.runtime"),
        );

        assert!(!editor.has_errors(), "{:?}", editor.diagnostics);
        assert!(!runtime.has_errors(), "{:?}", runtime.diagnostics);
        assert_eq!(editor.root, runtime.root);
    }

    #[test]
    fn game_runtime_recipe_expansion_preserves_accessibility_layout_and_focus() {
        let mut declaration = component_recipe();
        declaration.layout.min_width = Some(120.0);
        declaration.focus_navigation = UiRecipeFocusNavigation {
            focusable: true,
            tab_order: Some(0),
            directional: true,
        };
        let library = library(vec![declaration.clone()]);
        let report = expand_ui_recipe(
            &library,
            &UiRecipeExpansionRequest::activate("stats.component", "game.runtime"),
        );

        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let root = report.root.expect("game runtime recipe expands");
        assert!(declaration.accessibility.is_some());
        assert_eq!(declaration.layout.min_width, Some(120.0));
        assert!(declaration.focus_navigation.focusable);
        assert_eq!(root.id().as_str(), "stats-root");
    }
}
