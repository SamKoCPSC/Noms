use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Staged additions only. Confirmed memberships cannot be removed by this picker.
#[derive(Clone)]
pub(super) struct PickerSelection {
    pub confirmed: HashSet<Uuid>,
    pub selected: HashSet<Uuid>,
    pub errors: HashMap<Uuid, String>,
    pub pending: bool,
}
impl PickerSelection {
    pub fn new(confirmed: HashSet<Uuid>) -> Self {
        Self {
            confirmed,
            selected: HashSet::new(),
            errors: HashMap::new(),
            pending: false,
        }
    }
    pub fn toggle(&mut self, id: Uuid) {
        if self.pending || self.confirmed.contains(&id) {
            return;
        }
        if !self.selected.remove(&id) {
            self.selected.insert(id);
        }
        self.errors.remove(&id);
    }
    pub fn begin(&mut self) -> Option<Vec<Uuid>> {
        if self.pending || self.selected.is_empty() {
            return None;
        }
        self.pending = true;
        self.errors.clear();
        let mut ids: Vec<_> = self.selected.iter().copied().collect();
        ids.sort();
        Some(ids)
    }
    pub fn finish(&mut self, confirmed: &HashSet<Uuid>, errors: Vec<(Uuid, String)>) {
        self.confirmed.extend(confirmed);
        self.selected.retain(|id| !self.confirmed.contains(id));
        self.errors = errors
            .into_iter()
            .filter(|(id, _)| !self.confirmed.contains(id))
            .collect();
        self.pending = false;
    }
}
