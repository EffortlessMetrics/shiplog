use std::collections::BTreeSet;

/// Tracks which source events have been assigned to a workstream.
///
/// The LLM can emit repeated or out-of-range indices. This small type owns the
/// first-wins claim policy so workstream assembly does not need to know about
/// duplicate handling.
pub(super) struct ClaimTracker {
    event_count: usize,
    claimed: BTreeSet<usize>,
}

impl ClaimTracker {
    pub(super) fn new(event_count: usize) -> Self {
        Self {
            event_count,
            claimed: BTreeSet::new(),
        }
    }

    pub(super) fn claim_available_indices(&mut self, indices: Vec<usize>) -> Vec<usize> {
        indices
            .into_iter()
            .filter(|&index| self.claim_index(index))
            .collect()
    }

    pub(super) fn orphan_indices(&self) -> Vec<usize> {
        (0..self.event_count)
            .filter(|index| !self.claimed.contains(index))
            .collect()
    }

    fn claim_index(&mut self, index: usize) -> bool {
        if index >= self.event_count || self.claimed.contains(&index) {
            return false;
        }

        self.claimed.insert(index);
        true
    }
}
