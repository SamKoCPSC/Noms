//! Client-side confirmation authority, separate from the server's authentication boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AccountDeleteStep {
    Confirming,
    Typing,
    Final,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccountDeletionState {
    pub step: Option<AccountDeleteStep>,
    pub confirmation: String,
    pub error: Option<String>,
    pub pending: bool,
}

impl AccountDeletionState {
    pub fn open(&mut self) {
        if !self.pending {
            *self = Self {
                step: Some(AccountDeleteStep::Confirming),
                ..Self::default()
            };
        }
    }
    pub fn confirm(&mut self) {
        if !self.pending && self.step == Some(AccountDeleteStep::Confirming) {
            self.step = Some(AccountDeleteStep::Typing);
        }
    }
    pub fn cancel(&mut self) {
        if !self.pending {
            *self = Self::default();
        }
    }
    pub fn set_confirmation(&mut self, value: String) {
        if !self.pending && self.step == Some(AccountDeleteStep::Typing) {
            self.confirmation = value;
            self.error = None;
        }
    }
    pub fn advance(&mut self, username: &str) {
        if !self.pending && self.step == Some(AccountDeleteStep::Typing) && self.matches(username) {
            self.step = Some(AccountDeleteStep::Final);
        }
    }
    pub fn go_back(&mut self) {
        if !self.pending && self.step == Some(AccountDeleteStep::Final) {
            self.step = Some(AccountDeleteStep::Typing);
            self.error = None;
        }
    }
    fn matches(&self, username: &str) -> bool {
        !username.is_empty() && self.confirmation == format!("delete {username}")
    }
    /// Claim the submission synchronously, before spawning any asynchronous work.
    pub fn begin_delete(&mut self, username: &str) -> bool {
        if self.pending || self.step != Some(AccountDeleteStep::Final) || !self.matches(username) {
            return false;
        }
        self.pending = true;
        self.error = None;
        true
    }
    /// Ignore stale completions after cancellation; retain the phrase for a deliberate retry.
    pub fn fail_delete(&mut self, error: String) {
        if self.pending {
            self.pending = false;
            self.error = Some(error);
        }
    }
}
