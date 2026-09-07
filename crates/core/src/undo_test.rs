use super::*;
use crate::project::SequenceExportSettings;
use crate::timeline::Timeline;

fn sequence(name: &str) -> Sequence {
    Sequence {
        id: 1,
        name: name.to_string(),
        timeline: Timeline {
            tracks: Vec::new(),
            playhead_secs: 0.0,
            markers: Vec::new(),
            multicam_groups: Vec::new(),
        },
        export_settings: SequenceExportSettings::default(),
    }
}

#[test]
fn a_fresh_stack_can_neither_undo_nor_redo() {
    let stack = UndoStack::new();
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());
}

#[test]
fn undo_with_nothing_pushed_returns_none() {
    let mut stack = UndoStack::new();
    assert_eq!(stack.undo(sequence("current")), None);
}

#[test]
fn redo_with_nothing_undone_returns_none() {
    let mut stack = UndoStack::new();
    assert_eq!(stack.redo(sequence("current")), None);
}

#[test]
fn push_then_undo_restores_the_pushed_state() {
    let mut stack = UndoStack::new();
    stack.push(sequence("before"));
    assert!(stack.can_undo());
    let restored = stack.undo(sequence("after"));
    assert_eq!(restored.map(|s| s.name), Some("before".to_string()));
}

#[test]
fn undo_then_redo_round_trips_back_to_the_state_undo_was_called_with() {
    let mut stack = UndoStack::new();
    stack.push(sequence("before"));
    let restored = stack.undo(sequence("after")).unwrap();
    assert_eq!(restored.name, "before");
    assert!(stack.can_redo());

    let redone = stack.redo(sequence("before")).unwrap();
    assert_eq!(redone.name, "after");
    assert!(!stack.can_redo());
    assert!(stack.can_undo());
}

#[test]
fn a_fresh_push_after_an_undo_discards_the_redo_branch() {
    let mut stack = UndoStack::new();
    stack.push(sequence("v1"));
    stack.undo(sequence("v2"));
    assert!(stack.can_redo());

    // A new edit after undoing invalidates whatever redo would have replayed.
    stack.push(sequence("v1-again"));
    assert!(!stack.can_redo());
    assert!(stack.can_undo());
}

#[test]
fn push_beyond_capacity_evicts_the_oldest_entry() {
    let mut stack = UndoStack::with_capacity(2);
    stack.push(sequence("v1"));
    stack.push(sequence("v2"));
    stack.push(sequence("v3"));

    // Capacity 2: only "v2" and "v3" should still be recoverable, oldest ("v1") evicted.
    let first_undo = stack.undo(sequence("v4")).unwrap();
    assert_eq!(first_undo.name, "v3");
    let second_undo = stack.undo(first_undo).unwrap();
    assert_eq!(second_undo.name, "v2");
    assert!(!stack.can_undo());
}

#[test]
fn with_capacity_treats_zero_as_at_least_one() {
    let mut stack = UndoStack::with_capacity(0);
    stack.push(sequence("v1"));
    assert!(stack.can_undo());
    let restored = stack.undo(sequence("v2")).unwrap();
    assert_eq!(restored.name, "v1");
}

#[test]
fn clear_drops_both_undo_and_redo_history() {
    let mut stack = UndoStack::new();
    stack.push(sequence("v1"));
    stack.undo(sequence("v2"));
    assert!(stack.can_redo());

    stack.clear();
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());
}

#[test]
fn multiple_undo_calls_pop_in_lifo_order() {
    let mut stack = UndoStack::new();
    stack.push(sequence("v1"));
    stack.push(sequence("v2"));

    let first = stack.undo(sequence("v3")).unwrap();
    assert_eq!(first.name, "v2");
    let second = stack.undo(first).unwrap();
    assert_eq!(second.name, "v1");
    assert!(!stack.can_undo());
}
