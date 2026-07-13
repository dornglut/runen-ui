#![allow(clippy::redundant_pub_crate)]

struct Slot<T> {
    generation: u64,
    value: Option<T>,
    retired: bool,
}

pub(crate) struct MountedArena<T> {
    slots: Vec<Slot<T>>,
    live_count: usize,
}

impl<T> MountedArena<T> {
    pub(crate) const fn new() -> Self {
        Self {
            slots: Vec::new(),
            live_count: 0,
        }
    }

    pub(crate) fn insert_with(&mut self, create: impl FnOnce(usize, u64) -> T) -> (usize, u64) {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.value.is_none() && !slot.retired {
                let Some(generation) = slot.generation.checked_add(1) else {
                    slot.retired = true;
                    continue;
                };
                slot.generation = generation;
                slot.value = Some(create(index, generation));
                self.live_count += 1;
                return (index, generation);
            }
        }
        let index = self.slots.len();
        let generation = 1;
        self.slots.push(Slot {
            generation,
            value: Some(create(index, generation)),
            retired: false,
        });
        self.live_count += 1;
        (index, generation)
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
    use super::MountedArena;

    #[test]
    fn allocation_removal_and_lowest_reuse_are_generational() {
        let mut arena = MountedArena::new();
        assert_eq!(arena.insert_with(|_, _| "a"), (0, 1));
        assert_eq!(arena.insert_with(|_, _| "b"), (1, 1));
        assert_eq!(arena.live_count(), 2);
        assert_eq!(arena.remove(0, 1), Some("a"));
        assert!(arena.get(0, 1).is_none());
        assert_eq!(arena.insert_with(|_, _| "c"), (0, 2));
        assert_eq!(arena.get(0, 2), Some(&"c"));
        assert!(arena.get(0, 1).is_none());
        assert_eq!(arena.live_count(), 2);
    }

    #[test]
    fn overflow_retires_slot_permanently() {
        let mut arena = MountedArena::new();
        arena.seed_vacant(u64::MAX);
        assert_eq!(arena.insert_with(|_, _| 1), (1, 1));
        assert_eq!(arena.remove(1, 1), Some(1));
        assert_eq!(arena.insert_with(|_, _| 2), (1, 2));
    }
}
