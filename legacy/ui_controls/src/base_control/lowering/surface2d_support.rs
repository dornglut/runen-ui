use super::super::{ControlDef, ControlPreset};
use crate::{ControlKindId, ControlSurface2DDescriptor};

pub(crate) fn lower_surface2d_descriptor(
    def: &ControlDef,
    kind_id: ControlKindId,
) -> Option<ControlSurface2DDescriptor> {
    match def.preset() {
        ControlPreset::Surface2D => Some(ControlSurface2DDescriptor::new(kind_id)),
        _ => None,
    }
}
