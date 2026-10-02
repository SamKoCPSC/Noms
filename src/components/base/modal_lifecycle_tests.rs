use super::modal_lifecycle::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Default)]
struct FakeDom {
    active: usize,
    connected: Vec<usize>,
    controls: HashMap<usize, Vec<usize>>,
    opens: usize,
    focused: Vec<usize>,
    overflow: Vec<(String, String)>,
}
#[derive(Clone)]
struct Platform(Rc<RefCell<FakeDom>>);
impl ModalPlatform for Platform {
    type Element = usize;
    type ScrollSnapshot = Vec<(String, String)>;
    fn dialog(&self, id: &str) -> Option<usize> {
        id.parse()
            .ok()
            .filter(|id| self.0.borrow().connected.contains(id))
    }
    fn active(&self) -> Option<usize> {
        Some(self.0.borrow().active)
    }
    fn target(&self, dialog: &usize, selector: &str) -> Option<usize> {
        self.0
            .borrow()
            .controls
            .get(dialog)?
            .get(usize::from(selector == "input"))
            .copied()
    }
    fn open(&self, _: &usize, _: Option<&usize>) -> bool {
        self.0.borrow_mut().opens += 1;
        true
    }
    fn close(&self, _: &usize) {}
    fn lock_scroll(&self) -> Self::ScrollSnapshot {
        let mut dom = self.0.borrow_mut();
        let snapshot = dom.overflow.clone();
        dom.overflow = vec![("hidden".into(), "important".into()); 2];
        snapshot
    }
    fn restore_scroll(&self, snapshot: Self::ScrollSnapshot) {
        self.0.borrow_mut().overflow = snapshot;
    }
    fn connected(&self, element: &usize) -> bool {
        self.0.borrow().connected.contains(element)
    }
    fn contains(&self, parent: &usize, child: &usize) -> bool {
        parent == child
            || self
                .0
                .borrow()
                .controls
                .get(parent)
                .is_some_and(|controls| controls.contains(child))
    }
    fn focus(&self, element: &usize, _: bool) {
        let mut dom = self.0.borrow_mut();
        dom.active = *element;
        dom.focused.push(*element);
    }
    fn focus_dialog(&self, element: &usize, prevent_scroll: bool) {
        self.focus(element, prevent_scroll);
    }
    fn controls(&self, dialog: &usize) -> Vec<usize> {
        self.0
            .borrow()
            .controls
            .get(dialog)
            .cloned()
            .unwrap_or_default()
    }
    fn is_open(&self, _: &usize) -> bool {
        true
    }
}
fn fixture() -> (ModalLifecycle<Platform>, Rc<RefCell<FakeDom>>) {
    let dom = Rc::new(RefCell::new(FakeDom {
        active: 1,
        connected: vec![1, 10, 11, 12, 20, 21, 22],
        controls: HashMap::from([(10, vec![11, 12]), (20, vec![21, 22])]),
        overflow: vec![
            ("scroll".into(), "".into()),
            ("clip".into(), "important".into()),
        ],
        ..Default::default()
    }));
    (ModalLifecycle::new(Platform(dom.clone())), dom)
}
#[test]
fn modal_safe_focus_stage_changes_and_rerenders() {
    let (mut lifecycle, dom) = fixture();
    lifecycle.sync("10", "button", false);
    assert_eq!(dom.borrow().active, 11);
    assert_eq!(
        dom.borrow().overflow[0],
        ("hidden".into(), "important".into())
    );
    lifecycle.sync("10", "input", false);
    assert_eq!(dom.borrow().active, 12);
    dom.borrow_mut().active = 11;
    lifecycle.sync("10", "input", false);
    assert_eq!(dom.borrow().active, 11);
    assert_eq!(dom.borrow().opens, 1);
    lifecycle.dispose("10");
    assert_eq!(dom.borrow().active, 1);
    assert_eq!(
        dom.borrow().overflow,
        vec![
            ("scroll".into(), "".into()),
            ("clip".into(), "important".into())
        ]
    );
}
#[test]
fn modal_nested_and_out_of_order_cleanup_preserve_lock_and_focus() {
    let (mut lifecycle, dom) = fixture();
    lifecycle.sync("10", "button", false);
    lifecycle.sync("20", "input", false);
    assert!(!lifecycle.wrap_tab("10", true));
    lifecycle.dispose("20");
    assert_eq!(dom.borrow().active, 11);
    assert_eq!(dom.borrow().overflow[0].0, "hidden");
    lifecycle.sync("20", "input", false);
    lifecycle.dispose("10");
    assert_eq!(dom.borrow().active, 22);
    assert_eq!(dom.borrow().overflow[0].0, "hidden");
    lifecycle.dispose("20");
    assert_eq!(dom.borrow().overflow[0].0, "scroll");
}
#[test]
fn modal_missing_duplicate_disconnected_and_empty_controls_are_safe() {
    let (mut lifecycle, dom) = fixture();
    lifecycle.sync("missing", "button", false);
    lifecycle.dispose("missing");
    assert_eq!(dom.borrow().opens, 0);
    lifecycle.sync("10", "button", false);
    assert!(!lifecycle.wrap_tab("10", false));
    assert!(lifecycle.wrap_tab("10", true));
    assert_eq!(dom.borrow().active, 12);
    assert!(lifecycle.wrap_tab("10", false));
    assert_eq!(dom.borrow().active, 11);
    dom.borrow_mut().controls.insert(10, vec![]);
    assert!(lifecycle.wrap_tab("10", false));
    assert_eq!(dom.borrow().active, 10);
    dom.borrow_mut().connected.retain(|id| *id != 1);
    lifecycle.dispose("10");
    lifecycle.dispose("10");
    assert!(!dom.borrow().focused.contains(&1));
    assert!(!lifecycle.wrap_tab("10", false));
}
#[test]
fn modal_pending_anchors_focus_and_failure_restores_safe_target() {
    let (mut lifecycle, dom) = fixture();
    lifecycle.sync("10", "button", false);
    dom.borrow_mut().active = 1;
    lifecycle.sync("10", "button", true);
    assert_eq!(dom.borrow().active, 10);
    lifecycle.sync("10", "button", false);
    assert_eq!(dom.borrow().active, 11);
}

#[test]
fn modal_session_suspension_retains_entries_releases_scroll_and_restores_pending_focus() {
    let (mut lifecycle, dom) = fixture();
    lifecycle.sync("10", "input", false);
    lifecycle.sync("20", "button", false);
    lifecycle.suspend("20");
    assert_eq!(dom.borrow().overflow[0].0, "hidden");
    lifecycle.suspend("10");
    assert_eq!(dom.borrow().overflow[0].0, "scroll");
    assert!(!lifecycle.wrap_tab("20", false));
    dom.borrow_mut().active = 1;
    lifecycle.sync("20", "button", true);
    assert_eq!(
        dom.borrow().active,
        1,
        "Hidden pending updates must not take focus"
    );
    assert_eq!(dom.borrow().opens, 2);
    lifecycle.resume("10");
    lifecycle.resume("20");
    assert_eq!(dom.borrow().opens, 4);
    assert_eq!(dom.borrow().active, 20);
    assert_eq!(dom.borrow().overflow[0].0, "hidden");
    lifecycle.dispose("20");
    lifecycle.dispose("10");
    assert_eq!(dom.borrow().overflow[0].0, "scroll");
    assert_eq!(dom.borrow().active, 1);
}
