use crate::card::Card;
use crate::creature::CreatureState;
use crate::rng::Rng;

pub const HAND_SIZE: usize = 5;
pub const MAX_HAND: usize = 10;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerState {
    pub creature: CreatureState,
    pub energy: i32,
    pub energy_master: i32,   // energy restored at start of each turn
    pub hand: Vec<Card>,
    pub draw_pile: Vec<Card>,
    pub discard_pile: Vec<Card>,
    pub exhaust_pile: Vec<Card>,
}

impl PlayerState {
    pub fn new(hp: i32, max_hp: i32, energy: i32, deck: Vec<Card>, shuffle_rng: &mut Rng) -> Self {
        let mut draw_pile = deck;
        shuffle(&mut draw_pile, shuffle_rng);
        PlayerState {
            creature: CreatureState::new(hp, max_hp),
            energy,
            energy_master: energy,
            hand: vec![],
            draw_pile,
            discard_pile: vec![],
            exhaust_pile: vec![],
        }
    }

    /// Draw `n` cards, shuffling discard into draw when draw pile runs out.
    pub fn draw(&mut self, n: usize, shuffle_rng: &mut Rng) {
        let count = n.min(MAX_HAND.saturating_sub(self.hand.len()));
        if count == 0 || (self.draw_pile.is_empty() && self.discard_pile.is_empty()) {
            return;
        }
        let from_draw = count.min(self.draw_pile.len());
        self.hand.extend(self.draw_pile.drain(..from_draw));
        if count > from_draw {
            // DrawCardAction queues a shuffle before the remaining draw, even
            // if drawing the last available cards leaves an empty discard pile.
            // EmptyDeckShuffleAction still consumes one shuffle RNG call.
            self.draw_pile.append(&mut self.discard_pile);
            shuffle(&mut self.draw_pile, shuffle_rng);
            self.draw(count - from_draw, shuffle_rng);
        }
    }

    /// Move card at `hand_idx` to discard.
    pub fn discard_from_hand(&mut self, hand_idx: usize) {
        let card = self.hand.remove(hand_idx);
        self.discard_pile.push(card);
    }

    /// Move card at `hand_idx` to exhaust pile.
    pub fn exhaust_from_hand(&mut self, hand_idx: usize) {
        let card = self.hand.remove(hand_idx);
        self.exhaust_pile.push(card);
    }

    /// Called at the start of each player turn.
    pub fn start_turn(&mut self, shuffle_rng: &mut Rng) {
        self.creature.lose_block();
        self.creature.trigger_start_of_turn();
        self.energy = self.energy_master;
        self.draw(HAND_SIZE, shuffle_rng);
        self.creature.trigger_start_of_turn_post_draw();
    }

    /// Exhaust Ethereal cards, then discard the rest before enemies act.
    pub fn end_turn(&mut self) {
        let mut discard = Vec::with_capacity(self.hand.len());
        for card in self.hand.drain(..) {
            if crate::card::is_ethereal(card.id) {
                self.exhaust_pile.push(card);
            } else {
                discard.push(card);
            }
        }
        // DiscardAction repeatedly removes CardGroup.getTopCard(): the last
        // hand entry. This order becomes observable after the next shuffle.
        self.discard_pile.extend(discard.into_iter().rev());
    }
}

/// Match CardGroup's Java shuffle, storing the next card at index zero.
fn shuffle(cards: &mut [Card], rng: &mut Rng) {
    crate::rng::shuffle_draw_pile(cards, rng);
}
