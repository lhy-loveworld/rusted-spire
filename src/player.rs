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
        for _ in 0..n {
            if self.hand.len() >= MAX_HAND {
                break;
            }
            if self.draw_pile.is_empty() {
                if self.discard_pile.is_empty() {
                    break;
                }
                self.draw_pile.append(&mut self.discard_pile);
                shuffle(&mut self.draw_pile, shuffle_rng);
            }
            let card = self.draw_pile.remove(0);
            self.hand.push(card);
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
        for card in self.hand.drain(..) {
            if crate::card::is_ethereal(card.id) {
                self.exhaust_pile.push(card);
            } else {
                self.discard_pile.push(card);
            }
        }
    }
}

/// Fisher-Yates shuffle using the game's shuffle rng stream.
fn shuffle(cards: &mut Vec<Card>, rng: &mut Rng) {
    for i in (1..cards.len()).rev() {
        let j = rng.random_int(i as i32) as usize;
        cards.swap(i, j);
    }
}
