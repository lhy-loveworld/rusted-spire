/// All implemented cards. Extend this enum as more cards are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CardId {
    // Ironclad basics
    Strike,
    Defend,
    Bash,
}

/// A card instance in the player's deck or hand.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Card {
    pub id: CardId,
    pub upgraded: bool,
    pub cost: i32,
}

impl Card {
    pub fn new(id: CardId) -> Self {
        let cost = base_cost(id);
        Card { id, upgraded: false, cost }
    }

    pub fn upgraded(id: CardId) -> Self {
        Card { id, upgraded: true, cost: upgraded_cost(id) }
    }
}

fn base_cost(id: CardId) -> i32 {
    match id {
        CardId::Strike => 1,
        CardId::Defend => 1,
        CardId::Bash   => 2,
    }
}

fn upgraded_cost(id: CardId) -> i32 {
    base_cost(id) // none of these three change cost on upgrade
}

/// Whether a card requires selecting an enemy target.
pub fn requires_target(id: CardId) -> bool {
    match id {
        CardId::Strike => true,
        CardId::Defend => false,
        CardId::Bash   => true,
    }
}
