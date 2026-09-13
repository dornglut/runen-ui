use runenui_core::{LogicalTransform, PaintContribution, WidgetDiagnostic};

use crate::scene::SceneClip;

use super::groups::{ExplicitGroupId, ResolvedExplicitGroup};

fn group_clip_diagnostic(
    group_order: usize,
    clip_order: usize,
    non_finite: bool,
) -> WidgetDiagnostic {
    if non_finite {
        WidgetDiagnostic::new(
            "runenui.scene.paint-group-clip-transform-non-finite",
            format!(
                "paint group {group_order} clip {clip_order} final transform cannot be represented finitely; the explicit group subtree is excluded"
            ),
        )
    } else {
        WidgetDiagnostic::new(
            "runenui.scene.paint-group-clip-transform-non-invertible",
            format!(
                "paint group {group_order} clip {clip_order} final transform is non-invertible; logical coverage is empty"
            ),
        )
    }
}

/// Resolves one widget's normalized explicit groups into surface-space neutral facts.
///
/// A non-finite group clip rejects that complete explicit subtree. The returned
/// mapping therefore carries `None` for the rejected group and every descendant;
/// contribution items that name such a group are excluded instead of leaking back
/// into an enclosing node group or the scene root.
pub(super) fn append_resolved_explicit_groups(
    contribution: &PaintContribution,
    owner: usize,
    owner_to_surface: LogicalTransform,
    diagnostics: &mut Vec<WidgetDiagnostic>,
    resolved: &mut Vec<ResolvedExplicitGroup>,
) -> Vec<Option<ExplicitGroupId>> {
    let mut local_to_resolved = Vec::with_capacity(contribution.__runtime_group_count());
    for local_group in 0..contribution.__runtime_group_count() {
        let parent_local = contribution.__runtime_group_parent(local_group);
        if parent_local
            .is_some_and(|parent| local_to_resolved.get(parent).copied().flatten().is_none())
        {
            local_to_resolved.push(None);
            continue;
        }

        let clips = contribution
            .__runtime_group_clips(local_group)
            .unwrap_or_else(|| unreachable!("runtime iterates normalized group range"));
        let mut scene_clips = Vec::with_capacity(clips.len());
        let mut finite = true;
        for (clip_order, clip) in clips.iter().enumerate() {
            let Ok(clip_to_surface) = clip.local_to_owner().then(owner_to_surface) else {
                diagnostics.push(group_clip_diagnostic(local_group, clip_order, true));
                finite = false;
                break;
            };
            if clip_to_surface.inverse().is_none() {
                diagnostics.push(group_clip_diagnostic(local_group, clip_order, false));
            }
            scene_clips.push(SceneClip::new(clip.shape().clone(), clip_to_surface));
        }
        if !finite {
            local_to_resolved.push(None);
            continue;
        }

        let parent = parent_local.and_then(|parent| local_to_resolved[parent]);
        let opacity = contribution
            .__runtime_group_opacity(local_group)
            .unwrap_or_else(|| unreachable!("normalized group opacity exists"));
        let shadows = contribution
            .__runtime_group_shadows(local_group)
            .unwrap_or_else(|| unreachable!("normalized group shadows exist"))
            .to_vec();
        let id = ExplicitGroupId::new(resolved.len());
        resolved.push(ResolvedExplicitGroup::new(
            owner,
            parent,
            scene_clips,
            opacity,
            shadows,
        ));
        local_to_resolved.push(Some(id));
    }
    local_to_resolved
}
