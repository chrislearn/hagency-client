//! Track final changed values in a batch, including replacement and removal.
use eyeball_im::VectorDiff;
use imbl::Vector;
use std::{collections::HashMap, hash::Hash, sync::Arc};

pub struct Changes<K, T>(HashMap<K, (usize, Arc<T>)>);
impl<K: Eq + Hash, T> Default for Changes<K, T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}
impl<K: Eq + Hash, T> Changes<K, T> {
    pub fn observe(
        &mut self,
        old: &Vector<Arc<T>>,
        diff: &VectorDiff<Arc<T>>,
        key: impl Fn(&T) -> K,
    ) {
        match diff {
            VectorDiff::Append { values } => {
                for (offset, value) in values.iter().enumerate() {
                    self.0
                        .insert(key(value), (old.len() + offset, value.clone()));
                }
            }
            VectorDiff::PushFront { value } => {
                for (index, _) in self.0.values_mut() {
                    *index += 1;
                }
                self.0.insert(key(value), (0, value.clone()));
            }
            VectorDiff::PushBack { value } => {
                self.0.insert(key(value), (old.len(), value.clone()));
            }
            VectorDiff::Insert { index, value } => {
                for (position, _) in self.0.values_mut() {
                    if *position >= *index {
                        *position += 1;
                    }
                }
                self.0.insert(key(value), (*index, value.clone()));
            }
            VectorDiff::Set { index, value } => {
                if let Some(previous) = old.get(*index) {
                    self.0.remove(&key(previous));
                }
                self.0.insert(key(value), (*index, value.clone()));
            }
            VectorDiff::PopFront => {
                if let Some(value) = old.front() {
                    self.0.remove(&key(value));
                }
                for (position, _) in self.0.values_mut() {
                    *position = position.saturating_sub(1);
                }
            }
            VectorDiff::PopBack => {
                if let Some(value) = old.back() {
                    self.0.remove(&key(value));
                }
            }
            VectorDiff::Remove { index } => {
                if let Some(value) = old.get(*index) {
                    self.0.remove(&key(value));
                }
                for (position, _) in self.0.values_mut() {
                    if *position > *index {
                        *position -= 1;
                    }
                }
            }
            VectorDiff::Truncate { length } => {
                self.0.retain(|_, (position, _)| *position < *length)
            }
            VectorDiff::Clear => self.0.clear(),
            VectorDiff::Reset { values } => {
                self.0.clear();
                for (position, value) in values.iter().enumerate() {
                    self.0.insert(key(value), (position, value.clone()));
                }
            }
        }
    }
    /// Preserve final timeline order for approval and discovery protocol events.
    pub fn into_values(self) -> impl Iterator<Item = Arc<T>> {
        let mut values = self.0.into_values().collect::<Vec<_>>();
        values.sort_unstable_by_key(|(position, _)| *position);
        values.into_iter().map(|(_, value)| value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepends_and_mid_batch_moves_preserve_final_protocol_order() {
        let mut old = Vector::new();
        let mut changes = Changes::default();
        for value in [3, 2, 1] {
            let diff = VectorDiff::PushFront {
                value: Arc::new(value),
            };
            changes.observe(&old, &diff, |v| *v);
            old.push_front(Arc::new(value));
        }
        let diff = VectorDiff::Remove { index: 1 };
        changes.observe(&old, &diff, |v| *v);
        old.remove(1);
        changes.observe(
            &old,
            &VectorDiff::Insert {
                index: 1,
                value: Arc::new(4),
            },
            |v| *v,
        );
        assert_eq!(
            changes.into_values().map(|v| *v).collect::<Vec<_>>(),
            [1, 4, 3]
        );
    }
    #[test]
    fn ordinary_update_does_not_revisit_large_history() {
        let old: Vector<_> = (0..1_000_000).map(Arc::new).collect();
        let mut changes = Changes::default();
        let visited = std::cell::Cell::new(0);
        changes.observe(
            &old,
            &VectorDiff::PushBack {
                value: Arc::new(1_000_000),
            },
            |v| {
                visited.set(visited.get() + 1);
                *v
            },
        );
        assert_eq!(visited.get(), 1);
        assert_eq!(
            changes.into_values().map(|v| *v).collect::<Vec<_>>(),
            [1_000_000]
        );
    }
    #[test]
    fn removed_or_superseded_values_cannot_be_ingested() {
        let mut old = Vector::new();
        let mut changes = Changes::default();
        let first = Arc::new((1, "stale"));
        changes.observe(
            &old,
            &VectorDiff::PushBack {
                value: first.clone(),
            },
            |v| v.0,
        );
        old.push_back(first);
        let newest = Arc::new((1, "current"));
        changes.observe(
            &old,
            &VectorDiff::Set {
                index: 0,
                value: newest.clone(),
            },
            |v| v.0,
        );
        old.set(0, newest);
        assert_eq!(changes.0.len(), 1);
        assert_eq!(changes.0[&1].1.1, "current");
        changes.observe(&old, &VectorDiff::Remove { index: 0 }, |v| v.0);
        assert_eq!(changes.into_values().count(), 0);
    }
    #[test]
    fn reset_clear_and_truncation_remove_pending_candidates() {
        let old: Vector<_> = (0..10).map(Arc::new).collect();
        let mut changes = Changes::default();
        changes.observe(
            &Vector::new(),
            &VectorDiff::Reset {
                values: old.clone(),
            },
            |v| *v,
        );
        changes.observe(&old, &VectorDiff::Truncate { length: 2 }, |v| *v);
        assert_eq!(changes.0.len(), 2);
        changes.observe(&old, &VectorDiff::Clear, |v| *v);
        assert_eq!(changes.into_values().count(), 0);
    }
}
