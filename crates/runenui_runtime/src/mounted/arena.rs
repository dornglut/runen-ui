#![allow(clippy::redundant_pub_crate)]

struct Slot<T> {
    generation: u64,
    value: Option<T>,
    retired: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ArenaCapacityError;

pub(crate) struct GenerationalArena<T> {
    slots: Vec<Slot<T>>,
    live_count: usize,
}

impl<T> GenerationalArena<T> {
    pub(crate) const fn new() -> Self {
        Self {
            slots: Vec::new(),
            live_count: 0,
        }
    }

    pub(crate) fn preflight_live_count(
        &self,
        live_count: usize,
        public_slot_limit: u64,
    ) -> Result<(), ArenaCapacityError> {
        self.preflight_live_count_after_retirement(live_count, 0, public_slot_limit)
    }

    pub(crate) fn preflight_live_count_after_retirement(
        &self,
        live_count: usize,
        additionally_retired: usize,
        public_slot_limit: u64,
    ) -> Result<(), ArenaCapacityError> {
        let bounded_limit = public_slot_limit.min(u64::from(u32::MAX) + 1);
        let unavailable = self
            .slots
            .iter()
            .filter(|slot| slot.retired || (slot.value.is_none() && slot.generation == u64::MAX))
            .count()
            .checked_add(additionally_retired)
            .ok_or(ArenaCapacityError)?;
        let unavailable = u64::try_from(unavailable).map_err(|_| ArenaCapacityError)?;
        let usable = bounded_limit
            .checked_sub(unavailable)
            .ok_or(ArenaCapacityError)?;
        let live_count = u64::try_from(live_count).map_err(|_| ArenaCapacityError)?;
        (live_count <= usable)
            .then_some(())
            .ok_or(ArenaCapacityError)
    }

    pub(crate) fn insert_with(
        &mut self,
        create: impl FnOnce(usize, u64) -> T,
    ) -> Result<(usize, u64), ArenaCapacityError> {
        self.insert_with_public_slot_limit(u64::from(u32::MAX) + 1, create)
    }

    pub(crate) fn insert_with_public_slot_limit(
        &mut self,
        public_slot_limit: u64,
        create: impl FnOnce(usize, u64) -> T,
    ) -> Result<(usize, u64), ArenaCapacityError> {
        let bounded_limit = public_slot_limit.min(u64::from(u32::MAX) + 1);
        let mut create = Some(create);

        for (index, slot) in self.slots.iter_mut().enumerate() {
            let public_index = u64::try_from(index).map_err(|_| ArenaCapacityError)?;
            if public_index >= bounded_limit {
                break;
            }
            if slot.value.is_none() && !slot.retired {
                let Some(generation) = slot.generation.checked_add(1) else {
                    slot.retired = true;
                    continue;
                };
                slot.generation = generation;
                let create = create.take().ok_or(ArenaCapacityError)?;
                slot.value = Some(create(index, generation));
                self.live_count += 1;
                return Ok((index, generation));
            }
        }

        let index = self.slots.len();
        let public_index = u64::try_from(index).map_err(|_| ArenaCapacityError)?;
        if public_index >= bounded_limit {
            return Err(ArenaCapacityError);
        }
        let _ = u32::try_from(index).map_err(|_| ArenaCapacityError)?;
        let generation = 1;
        let create = create.take().ok_or(ArenaCapacityError)?;
        self.slots.push(Slot {
            generation,
            value: Some(create(index, generation)),
            retired: false,
        });
        self.live_count += 1;
        Ok((index, generation))
    }

    pub(crate) fn get(&self, index: usize, generation: u64) -> Option<&T> {
        let slot = self.slots.get(index)?;
        (slot.generation == generation)
            .then_some(slot.value.as_ref())
            .flatten()
    }

    pub(crate) fn get_mut(&mut self, index: usize, generation: u64) -> Option<&mut T> {
        let slot = self.slots.get_mut(index)?;
        (slot.generation == generation)
            .then_some(slot.value.as_mut())
            .flatten()
    }

    pub(crate) fn contains_slot(&self, index: usize) -> bool {
        self.slots.get(index).is_some()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn remove(&mut self, index: usize, generation: u64) -> Option<T> {
        let slot = self.slots.get_mut(index)?;
        if slot.generation != generation {
            return None;
        }
        let value = slot.value.take()?;
        self.live_count -= 1;
        if slot.generation == u64::MAX {
            slot.retired = true;
        }
        Some(value)
    }

    pub(crate) const fn live_count(&self) -> usize {
        self.live_count
    }

    #[cfg(test)]
    fn seed_vacant(&mut self, generation: u64) {
        self.slots.push(Slot {
            generation,
            value: None,
            retired: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{ArenaCapacityError, GenerationalArena};

    #[test]
    fn allocation_removal_and_lowest_reuse_are_generational() {
        let mut arena = GenerationalArena::new();
        assert_eq!(arena.insert_with(|_, _| "a"), Ok((0, 1)));
        assert_eq!(arena.insert_with(|_, _| "b"), Ok((1, 1)));
        assert_eq!(arena.live_count(), 2);
        assert_eq!(arena.remove(0, 1), Some("a"));
        assert!(arena.get(0, 1).is_none());
        assert_eq!(arena.insert_with(|_, _| "c"), Ok((0, 2)));
        assert_eq!(arena.get(0, 2), Some(&"c"));
        assert!(arena.get(0, 1).is_none());
        assert_eq!(arena.live_count(), 2);
    }

    #[test]
    fn public_slot_capacity_is_checked_before_create_runs() {
        let mut arena = GenerationalArena::new();
        let mut calls = 0usize;
        assert_eq!(arena.preflight_live_count(2, 1), Err(ArenaCapacityError));
        assert_eq!(calls, 0);
        assert_eq!(arena.preflight_live_count(1, 1), Ok(()));
        assert_eq!(
            arena.insert_with_public_slot_limit(1, |_, _| {
                calls += 1;
                "only"
            }),
            Ok((0, 1))
        );
        assert_eq!(calls, 1);
        assert_eq!(
            arena.insert_with_public_slot_limit(1, |_, _| {
                calls += 1;
                "overflow"
            }),
            Err(ArenaCapacityError)
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn exhausted_vacancy_is_skipped_and_counted_unusable() {
        let mut arena = GenerationalArena::new();
        arena.seed_vacant(u64::MAX);
        assert_eq!(arena.preflight_live_count(1, 1), Err(ArenaCapacityError));
        assert_eq!(arena.insert_with(|_, _| "next"), Ok((1, 1)));
        assert_eq!(arena.get(1, 1), Some(&"next"));
    }

    #[test]
    fn removal_at_max_generation_retires_slot_permanently() {
        let mut arena = GenerationalArena::new();
        arena.seed_vacant(u64::MAX - 1);
        assert_eq!(arena.insert_with(|_, _| 1), Ok((0, u64::MAX)));
        assert_eq!(arena.remove(0, u64::MAX), Some(1));
        assert_eq!(arena.insert_with(|_, _| 2), Ok((1, 1)));
    }

    #[test]
    fn transition_preflight_accounts_for_newly_retired_removed_slots() {
        let mut arena = GenerationalArena::new();
        arena.seed_vacant(u64::MAX - 1);
        assert_eq!(arena.insert_with(|_, _| "old"), Ok((0, u64::MAX)));
        assert_eq!(
            arena.preflight_live_count_after_retirement(1, 1, 1),
            Err(ArenaCapacityError)
        );
        assert_eq!(arena.preflight_live_count_after_retirement(1, 1, 2), Ok(()));
    }
}
