use teloxide::dispatching::dialogue::{Dialogue, InMemStorage};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BotDialogState {
    #[default]
    Idle,
    WaitingEmergencyText,
    WaitingForInvite,
}

pub type BotDialogue = Dialogue<BotDialogState, InMemStorage<BotDialogState>>;
