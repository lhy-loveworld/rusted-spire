use crate::card::{Card, CardId, requires_target};
use crate::damage::{apply_powers, DamageType};
use crate::enemy::{EnemyId, EnemyState};
use crate::player::PlayerState;
use crate::power::PowerId;
use crate::rng::RngBundle;

// ---------------------------------------------------------------------------
// Top-level state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CombatState {
    pub player: PlayerState,
    pub enemies: Vec<EnemyState>,
    pub rng: RngBundle,
    pub turn: u32,
    pub phase: CombatPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CombatPhase {
    PlayerTurn,
    Over(CombatResult),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CombatResult {
    Victory,
    Defeat,
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    PlayCard { hand_idx: usize, target_idx: usize },
    EndTurn,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl CombatState {
    pub fn new(deck: Vec<Card>, enemy_ids: &[EnemyId], seed: u64) -> Self {
        let mut rng = RngBundle::new(seed);

        let enemies = enemy_ids
            .iter()
            .map(|&id| EnemyState::new(id, 0, &mut rng.monster_hp, &mut rng.ai))
            .collect();

        let mut player = PlayerState::new(80, 3, deck, &mut rng.shuffle);
        // Draw opening hand (no start_turn block reset on turn 1 — player starts with no block)
        player.draw(crate::player::HAND_SIZE, &mut rng.shuffle);

        CombatState {
            player,
            enemies,
            rng,
            turn: 1,
            phase: CombatPhase::PlayerTurn,
        }
    }
}

// ---------------------------------------------------------------------------
// Available actions
// ---------------------------------------------------------------------------

pub fn available_actions(state: &CombatState) -> Vec<Action> {
    if state.phase != CombatPhase::PlayerTurn {
        return vec![];
    }

    let living_enemies: Vec<usize> = state.enemies
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.is_dead())
        .map(|(i, _)| i)
        .collect();

    let mut actions = vec![Action::EndTurn];

    for (hand_idx, card) in state.player.hand.iter().enumerate() {
        if card.cost > state.player.energy {
            continue;
        }
        if requires_target(card.id) {
            for &target_idx in &living_enemies {
                actions.push(Action::PlayCard { hand_idx, target_idx });
            }
        } else {
            actions.push(Action::PlayCard { hand_idx, target_idx: 0 });
        }
    }

    actions
}

// ---------------------------------------------------------------------------
// Step
// ---------------------------------------------------------------------------

/// Applies `action` to `state` and returns the combat result if combat ended.
/// Panics if the action is illegal.
pub fn step(state: &mut CombatState, action: Action) -> Option<CombatResult> {
    assert_eq!(state.phase, CombatPhase::PlayerTurn, "step called outside player turn");

    match action {
        Action::PlayCard { hand_idx, target_idx } => {
            play_card(state, hand_idx, target_idx);
            if all_dead(&state.enemies) {
                let result = CombatResult::Victory;
                state.phase = CombatPhase::Over(result.clone());
                return Some(result);
            }
            if state.player.creature.is_dead() {
                let result = CombatResult::Defeat;
                state.phase = CombatPhase::Over(result.clone());
                return Some(result);
            }
        }
        Action::EndTurn => {
            end_player_turn(state);
            if state.player.creature.is_dead() {
                let result = CombatResult::Defeat;
                state.phase = CombatPhase::Over(result.clone());
                return Some(result);
            }
            if all_dead(&state.enemies) {
                let result = CombatResult::Victory;
                state.phase = CombatPhase::Over(result.clone());
                return Some(result);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Card execution
// ---------------------------------------------------------------------------

fn play_card(state: &mut CombatState, hand_idx: usize, target_idx: usize) {
    let card = state.player.hand[hand_idx].clone();
    state.player.energy -= card.cost;

    match card.id {
        CardId::Strike => {
            let base = if card.upgraded { 9 } else { 6 };
            let dmg = apply_powers(
                base,
                DamageType::Normal,
                &state.player.creature.powers,
                &state.enemies[target_idx].creature.powers,
            );
            state.enemies[target_idx].creature.receive_damage(dmg, DamageType::Normal);
            state.player.discard_from_hand(hand_idx);
        }
        CardId::Defend => {
            let base = if card.upgraded { 8 } else { 5 };
            state.player.creature.add_block(base);
            state.player.discard_from_hand(hand_idx);
        }
        CardId::Bash => {
            let base_dmg = if card.upgraded { 10 } else { 8 };
            let vuln = if card.upgraded { 3 } else { 2 };
            let dmg = apply_powers(
                base_dmg,
                DamageType::Normal,
                &state.player.creature.powers,
                &state.enemies[target_idx].creature.powers,
            );
            state.enemies[target_idx].creature.receive_damage(dmg, DamageType::Normal);
            state.enemies[target_idx].creature.apply_power(PowerId::Vulnerable, vuln);
            state.player.discard_from_hand(hand_idx);
        }
    }
}

// ---------------------------------------------------------------------------
// Turn transitions
// ---------------------------------------------------------------------------

fn end_player_turn(state: &mut CombatState) {
    // Tick player debuffs
    state.player.creature.tick_powers_end_of_turn(true);

    // Enemy turns
    let enemy_count = state.enemies.len();
    for i in 0..enemy_count {
        if state.enemies[i].is_dead() {
            continue;
        }
        // Borrow enemy separately from player
        let player_powers = state.player.creature.powers.clone();
        state.enemies[i].take_turn(
            &mut state.player.creature.hp,
            &mut state.player.creature.block,
            &player_powers,
            &mut state.rng.ai,
        );
        state.enemies[i].creature.tick_powers_end_of_turn(false);
    }

    // Enemy block resets at start of their turn (we reset it here for simplicity)
    for e in &mut state.enemies {
        e.creature.lose_block();
    }

    // Start next player turn
    state.turn += 1;
    state.player.start_turn(&mut state.rng.shuffle);
    state.phase = CombatPhase::PlayerTurn;
}

fn all_dead(enemies: &[EnemyState]) -> bool {
    enemies.iter().all(|e| e.is_dead())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ironclad_starter() -> Vec<Card> {
        let mut deck = vec![];
        for _ in 0..5 { deck.push(Card::new(CardId::Strike)); }
        for _ in 0..4 { deck.push(Card::new(CardId::Defend)); }
        deck.push(Card::new(CardId::Bash));
        deck
    }

    #[test]
    fn combat_initializes() {
        let state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 0);
        assert_eq!(state.player.hand.len(), 5);
        assert_eq!(state.player.energy, 3);
        assert_eq!(state.enemies.len(), 1);
        assert!(!state.enemies[0].is_dead());
        assert_eq!(state.phase, CombatPhase::PlayerTurn);
    }

    #[test]
    fn available_actions_nonempty() {
        let state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 42);
        let actions = available_actions(&state);
        assert!(actions.contains(&Action::EndTurn));
        assert!(actions.len() > 1); // at least one card playable
    }

    #[test]
    fn end_turn_advances_turn_counter() {
        let mut state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 1);
        assert_eq!(state.turn, 1);
        step(&mut state, Action::EndTurn);
        assert_eq!(state.turn, 2);
    }

    #[test]
    fn combat_can_be_won() {
        // Exhaust the jaw worm with enough strikes
        let mut state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 7);
        let mut turns = 0;
        loop {
            if let CombatPhase::Over(_) = &state.phase { break; }
            let actions = available_actions(&state);
            // Prefer playing Strike on enemy 0, fall back to EndTurn
            let action = actions.iter()
                .find(|a| matches!(a, Action::PlayCard { .. }))
                .cloned()
                .unwrap_or(Action::EndTurn);
            step(&mut state, action);
            turns += 1;
            assert!(turns < 200, "combat did not end in 200 turns");
        }
        assert!(matches!(state.phase, CombatPhase::Over(_)));
    }
}
