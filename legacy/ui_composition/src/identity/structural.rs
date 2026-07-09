use core::fmt;

use id::{InvalidRawId, TypedId};
use serde::{Deserialize, Serialize};

macro_rules! structural_id {
    ($name:ident, $tag:ident) => {
        pub enum $tag {}

        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(TypedId<$tag>);

        impl $name {
            pub const fn new(raw: u64) -> Self {
                Self(TypedId::new(raw))
            }

            pub const fn try_from_raw(raw: u64) -> Result<Self, InvalidRawId> {
                match TypedId::try_from_raw(raw) {
                    Ok(value) => Ok(Self(value)),
                    Err(error) => Err(error),
                }
            }

            pub const fn raw(self) -> u64 {
                self.0.raw()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({})", stringify!($name), self.raw())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.raw().fmt(formatter)
            }
        }

        impl TryFrom<u64> for $name {
            type Error = InvalidRawId;

            fn try_from(raw: u64) -> Result<Self, Self::Error> {
                Self::try_from_raw(raw)
            }
        }
    };
}

structural_id!(CompositionDefinitionId, CompositionDefinitionTag);
structural_id!(PresentationTargetId, PresentationTargetTag);
structural_id!(CompositionRootId, CompositionRootTag);
structural_id!(RegionId, RegionTag);
structural_id!(MountedUnitId, MountedUnitTag);
structural_id!(CompositionTransactionId, CompositionTransactionTag);
structural_id!(CompositionFixtureId, CompositionFixtureTag);
structural_id!(DefinitionRevision, DefinitionRevisionTag);
structural_id!(StateRevision, StateRevisionTag);
