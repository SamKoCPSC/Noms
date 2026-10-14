use super::collection_picker_state::*;
use std::collections::HashSet;
use uuid::Uuid;

#[test]
fn selection_is_staged_independent_and_excludes_existing_memberships() {
    let existing = Uuid::from_u128(1);
    let parent = Uuid::from_u128(2);
    let child = Uuid::from_u128(3);
    let mut state = PickerSelection::new([existing].into());
    state.toggle(existing);
    state.toggle(child);
    assert_eq!(state.selected, HashSet::from([child]));
    assert!(!state.selected.contains(&parent));
    assert_eq!(state.confirmed, HashSet::from([existing]));
    state.toggle(child);
    assert!(state.begin().is_none());
}

#[test]
fn pending_rejects_edits_and_duplicate_submission_and_partial_retry_excludes_success() {
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    let mut state = PickerSelection::new(HashSet::new());
    state.toggle(first);
    state.toggle(second);
    assert_eq!(state.begin().unwrap().len(), 2);
    assert!(state.begin().is_none());
    state.toggle(first);
    assert_eq!(state.selected.len(), 2);
    state.finish(&[first].into(), vec![(second, "Unavailable".into())]);
    assert_eq!(state.confirmed, HashSet::from([first]));
    assert_eq!(state.selected, HashSet::from([second]));
    assert_eq!(state.begin(), Some(vec![second]));
    state.finish(&[second].into(), vec![]);
    assert!(state.selected.is_empty());
    assert!(!state.pending);
}

#[test]
fn reconciliation_treats_uncertain_but_confirmed_additions_as_success() {
    let id = Uuid::from_u128(1);
    let mut state = PickerSelection::new(HashSet::new());
    state.toggle(id);
    state.begin();
    state.finish(&[id].into(), vec![(id, "Response lost".into())]);
    assert!(state.errors.is_empty());
    assert!(state.selected.is_empty());
}
