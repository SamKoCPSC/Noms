//! Entry waits for uploaded assets and a complete warm-up frame before opening.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gate {
    Waiting,
    Warming,
    Presenting,
    Open,
}
impl Gate {
    pub fn blocked(self) -> bool {
        self != Self::Open
    }
    pub fn after_frame(self) -> Self {
        match self {
            Self::Warming => Self::Presenting,
            Self::Presenting => Self::Open,
            other => other,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::Gate;
    #[test]
    fn frames_without_assets_cannot_open_the_tour() {
        let mut gate = Gate::Waiting;
        for _ in 0..10 {
            gate = gate.after_frame();
            assert!(gate.blocked());
        }
    }
    #[test]
    fn uploaded_assets_still_need_warmup_and_presentation() {
        let gate = Gate::Warming.after_frame();
        assert_eq!(gate, Gate::Presenting);
        assert!(gate.blocked());
        let gate = gate.after_frame();
        assert_eq!(gate, Gate::Open);
        assert!(!gate.blocked());
        assert_eq!(gate.after_frame(), Gate::Open);
    }
}
