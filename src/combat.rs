use crate::card::{Card, CardId, CardType, card_type, is_playable, requires_target};
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
    pub const MAX_HP: i32 = 80;

    pub fn new(deck: Vec<Card>, enemy_ids: &[EnemyId], seed: u64, ascension: u8, starting_hp: i32) -> Self {
        let mut rng = RngBundle::new(seed);

        let enemies = enemy_ids
            .iter()
            .map(|&id| EnemyState::new(id, ascension, &mut rng.monster_hp, &mut rng.ai))
            .collect();

        let mut player = PlayerState::new(starting_hp, Self::MAX_HP, 3, deck, &mut rng.shuffle);
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
        if card.cost > state.player.energy { continue; }
        if !is_playable(card.id) { continue; }
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
        // --- basics ---
        CardId::Strike => {
            let base = if card.upgraded { 9 } else { 6 };
            deal_to(state, hand_idx, target_idx, base, true);
        }
        CardId::Defend => {
            let base = if card.upgraded { 8 } else { 5 };
            state.player.creature.add_block(base);
            state.player.discard_from_hand(hand_idx);
        }
        CardId::Bash => {
            let base = if card.upgraded { 10 } else { 8 };
            let vuln = if card.upgraded { 3 } else { 2 };
            deal_to(state, hand_idx, target_idx, base, true);
            if !state.enemies[target_idx].is_dead() {
                state.enemies[target_idx].creature.apply_power(PowerId::Vulnerable, vuln);
            }
        }

        // --- attacks ---
        CardId::TwinStrike => {
            let base = if card.upgraded { 7 } else { 5 };
            deal_to(state, hand_idx, target_idx, base, false); // first hit (don't discard yet)
            // second hit — re-clone in case card was not yet discarded
            if !state.enemies[target_idx].is_dead() {
                let attacker_powers = state.player.creature.powers.clone();
                let dmg = apply_powers(base, DamageType::Normal, &attacker_powers, &state.enemies[target_idx].creature.powers);
                let hp_lost = state.enemies[target_idx].creature.receive_damage(dmg, DamageType::Normal);
                state.enemies[target_idx].creature.trigger_on_attacked(hp_lost, DamageType::Normal);
                process_death(state, target_idx);
            }
            state.player.discard_from_hand(hand_idx);
        }
        CardId::IronWave => {
            let base = if card.upgraded { 7 } else { 5 };
            state.player.creature.add_block(base);
            deal_to(state, hand_idx, target_idx, base, true);
        }
        CardId::Cleave => {
            let base = if card.upgraded { 11 } else { 8 };
            deal_all(state, base);
            state.player.discard_from_hand(hand_idx);
        }
        CardId::Clothesline => {
            let base = if card.upgraded { 14 } else { 12 };
            let weak = if card.upgraded { 3 } else { 2 };
            deal_to(state, hand_idx, target_idx, base, false);
            if !state.enemies[target_idx].is_dead() {
                state.enemies[target_idx].creature.apply_power(PowerId::Weak, weak);
            }
            state.player.discard_from_hand(hand_idx);
        }
        CardId::HeavyBlade => {
            // Strength bonus is ×3 (upgraded ×5) instead of ×1
            let str_bonus = state.player.creature.power_amount(PowerId::Strength);
            let extra = str_bonus * if card.upgraded { 4 } else { 2 }; // extra beyond the 1× already in apply_powers
            let base = if card.upgraded { 14 } else { 14 };
            let effective_base = base + extra;
            let attacker_powers = state.player.creature.powers.clone();
            let dmg = apply_powers(effective_base, DamageType::Normal, &attacker_powers, &state.enemies[target_idx].creature.powers);
            let hp_lost = state.enemies[target_idx].creature.receive_damage(dmg, DamageType::Normal);
            state.enemies[target_idx].creature.trigger_on_attacked(hp_lost, DamageType::Normal);
            process_death(state, target_idx);
            state.player.discard_from_hand(hand_idx);
        }
        CardId::BodySlam => {
            let base = state.player.creature.block;
            deal_to(state, hand_idx, target_idx, base, true);
        }
        CardId::Thunderclap => {
            let base = if card.upgraded { 7 } else { 4 };
            deal_all(state, base);
            let n = state.enemies.len();
            for i in 0..n {
                if !state.enemies[i].is_dead() {
                    state.enemies[i].creature.apply_power(PowerId::Vulnerable, 1);
                }
            }
            state.player.discard_from_hand(hand_idx);
        }
        CardId::PommelStrike => {
            let base = if card.upgraded { 10 } else { 9 };
            let draw = if card.upgraded { 2 } else { 1 };
            deal_to(state, hand_idx, target_idx, base, true);
            state.player.draw(draw, &mut state.rng.shuffle);
        }
        CardId::Anger => {
            let base = if card.upgraded { 8 } else { 6 };
            let copy = card.clone();
            deal_to(state, hand_idx, target_idx, base, false);
            state.player.discard_pile.push(copy);
            state.player.discard_from_hand(hand_idx);
        }
        CardId::WildStrike => {
            let base = if card.upgraded { 17 } else { 12 };
            deal_to(state, hand_idx, target_idx, base, false);
            state.player.draw_pile.push(Card::new(CardId::Wound));
            state.player.discard_from_hand(hand_idx);
        }
        CardId::SwordBoomerang => {
            let base = if card.upgraded { 4 } else { 3 };
            let hits = if card.upgraded { 4 } else { 3 };
            let attacker_powers = state.player.creature.powers.clone();
            for _ in 0..hits {
                let living: Vec<usize> = (0..state.enemies.len())
                    .filter(|&i| !state.enemies[i].is_dead())
                    .collect();
                if living.is_empty() { break; }
                let roll = state.rng.card.random_int((living.len() - 1) as i32) as usize;
                let t = living[roll];
                let dmg = apply_powers(base, DamageType::Normal, &attacker_powers, &state.enemies[t].creature.powers);
                let hp_lost = state.enemies[t].creature.receive_damage(dmg, DamageType::Normal);
                state.enemies[t].creature.trigger_on_attacked(hp_lost, DamageType::Normal);
                process_death(state, t);
            }
            state.player.discard_from_hand(hand_idx);
        }
        CardId::Dropkick => {
            let base = if card.upgraded { 8 } else { 5 };
            let vulnerable = state.enemies[target_idx].creature.power_amount(PowerId::Vulnerable) > 0;
            deal_to(state, hand_idx, target_idx, base, false);
            if vulnerable {
                state.player.energy += 1;
                state.player.draw(1, &mut state.rng.shuffle);
            }
            state.player.discard_from_hand(hand_idx);
        }

        // --- skills ---
        CardId::ShrugItOff => {
            let base = if card.upgraded { 11 } else { 8 };
            state.player.creature.add_block(base);
            state.player.discard_from_hand(hand_idx);
            state.player.draw(1, &mut state.rng.shuffle);
        }
        CardId::TrueGrit => {
            let base = if card.upgraded { 9 } else { 7 };
            state.player.creature.add_block(base);
            // Exhaust a random OTHER card in hand (before discarding TrueGrit)
            let other: Vec<usize> = (0..state.player.hand.len())
                .filter(|&i| i != hand_idx)
                .collect();
            state.player.discard_from_hand(hand_idx); // removes TrueGrit first
            if !other.is_empty() {
                let roll = state.rng.card.random_int((other.len() - 1) as i32) as usize;
                let mut exhaust_idx = other[roll];
                if exhaust_idx > hand_idx { exhaust_idx -= 1; } // adjust for removed card
                state.player.exhaust_from_hand(exhaust_idx);
            }
        }
        CardId::Flex => {
            let n = if card.upgraded { 4 } else { 2 };
            state.player.creature.apply_power(PowerId::Strength, n);
            state.player.creature.apply_power(PowerId::StrengthDown, n);
            state.player.discard_from_hand(hand_idx);
        }
        CardId::Intimidate => {
            let weak = if card.upgraded { 2 } else { 1 };
            let n = state.enemies.len();
            for i in 0..n {
                if !state.enemies[i].is_dead() {
                    state.enemies[i].creature.apply_power(PowerId::Weak, weak);
                }
            }
            state.player.discard_from_hand(hand_idx);
        }
        CardId::Armaments => {
            let base = if card.upgraded { 5 } else { 5 };
            state.player.creature.add_block(base);
            state.player.discard_from_hand(hand_idx);
            // Upgrade mechanic omitted for now
        }
        CardId::Warcry => {
            state.player.discard_from_hand(hand_idx);
            let draw_n = if card.upgraded { 3 } else { 2 };
            let before = state.player.hand.len();
            state.player.draw(draw_n, &mut state.rng.shuffle);
            let drew = state.player.hand.len() - before;
            // Put last drawn card back on top of draw pile
            if drew > 0 {
                let top = state.player.hand.remove(state.player.hand.len() - 1);
                state.player.draw_pile.insert(0, top);
            }
        }
        CardId::Headbutt => {
            let base = if card.upgraded { 12 } else { 9 };
            deal_to(state, hand_idx, target_idx, base, true);
            // Topdeck last card from discard
            if !state.player.discard_pile.is_empty() {
                let last = state.player.discard_pile.len() - 1;
                let card = state.player.discard_pile.remove(last);
                state.player.draw_pile.insert(0, card);
            }
        }
        CardId::Entrench => {
            let cur = state.player.creature.block;
            state.player.creature.add_block(cur);
            state.player.discard_from_hand(hand_idx);
        }

        // --- powers ---
        CardId::Inflame => {
            let n = if card.upgraded { 3 } else { 2 };
            state.player.creature.apply_power(PowerId::Strength, n);
            state.player.exhaust_from_hand(hand_idx);
        }
        CardId::Metallicize => {
            let n = if card.upgraded { 4 } else { 3 };
            state.player.creature.apply_power(PowerId::Metalicize, n);
            state.player.exhaust_from_hand(hand_idx);
        }
        CardId::DemonForm => {
            let n = if card.upgraded { 3 } else { 2 };
            state.player.creature.apply_power(PowerId::DemonForm, n);
            state.player.exhaust_from_hand(hand_idx);
        }

        // --- status ---
        CardId::Slimed => {
            state.player.exhaust_from_hand(hand_idx);
        }
        CardId::Wound => {
            // Unplayable — should never reach here
            state.player.discard_from_hand(hand_idx);
        }
    }

    // Trigger Anger power on all living enemies when a Skill is played
    if card_type(card.id) == CardType::Skill {
        for e in &mut state.enemies {
            if !e.is_dead() {
                e.creature.trigger_on_skill_played();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Damage helpers
// ---------------------------------------------------------------------------

/// Deal `base` damage to a single target, then process its death triggers.
/// If `and_discard` is true, also discard the card at `hand_idx`.
fn deal_to(state: &mut CombatState, hand_idx: usize, target_idx: usize, base: i32, and_discard: bool) {
    let attacker_powers = state.player.creature.powers.clone();
    let dmg = apply_powers(base, DamageType::Normal, &attacker_powers, &state.enemies[target_idx].creature.powers);
    let hp_lost = state.enemies[target_idx].creature.receive_damage(dmg, DamageType::Normal);
    state.enemies[target_idx].creature.trigger_on_attacked(hp_lost, DamageType::Normal);
    process_death(state, target_idx);
    if and_discard {
        state.player.discard_from_hand(hand_idx);
    }
}

/// Deal `base` damage to ALL living enemies (AoE).
fn deal_all(state: &mut CombatState, base: i32) {
    let attacker_powers = state.player.creature.powers.clone();
    let n = state.enemies.len();
    for i in 0..n {
        if !state.enemies[i].is_dead() {
            let dmg = apply_powers(base, DamageType::Normal, &attacker_powers, &state.enemies[i].creature.powers);
            let hp_lost = state.enemies[i].creature.receive_damage(dmg, DamageType::Normal);
            state.enemies[i].creature.trigger_on_attacked(hp_lost, DamageType::Normal);
        }
    }
    // Process all deaths after the full AoE sweep
    for i in 0..n {
        process_death(state, i);
    }
}

/// Process death effects for enemy at `idx` if it just died and hasn't been processed yet.
/// Spawns children (slime splits) and applies on-death power effects to the player.
fn process_death(state: &mut CombatState, idx: usize) {
    if !state.enemies[idx].is_dead() || state.enemies[idx].death_processed {
        return;
    }
    state.enemies[idx].death_processed = true;

    // On-death power applied to player
    if let Some((pid, amt)) = state.enemies[idx].on_death_effect() {
        state.player.creature.apply_power(pid, amt);
    }

    // Slime splits: spawn children into state.enemies
    let children = state.enemies[idx].spawn_on_death(&mut state.rng.monster_hp, &mut state.rng.ai);
    state.enemies.extend(children);
}

// ---------------------------------------------------------------------------
// Turn transitions
// ---------------------------------------------------------------------------

fn end_player_turn(state: &mut CombatState) {
    state.player.creature.tick_powers_end_of_turn(true);

    let enemy_count = state.enemies.len();
    for i in 0..enemy_count {
        if state.enemies[i].is_dead() { continue; }
        state.enemies[i].creature.trigger_start_of_turn(); // Metalicize etc.
        let queued_move = state.enemies[i].next_move;
        let enemy_id    = state.enemies[i].id;
        state.enemies[i].take_turn(&mut state.player.creature, &mut state.rng.ai);
        // Post-turn Slimed card spawning for slime enemies
        slimed_cards_for_move(enemy_id, queued_move, &mut state.player.discard_pile);
        state.enemies[i].creature.tick_powers_end_of_turn(false);
    }

    for e in &mut state.enemies {
        e.creature.lose_block();
    }

    state.turn += 1;
    state.player.start_turn(&mut state.rng.shuffle);
    state.phase = CombatPhase::PlayerTurn;
}

/// Adds Slimed status cards to the player's discard pile based on which move the enemy used.
fn slimed_cards_for_move(id: EnemyId, mv: u8, discard: &mut Vec<Card>) {
    let count: usize = match id {
        EnemyId::SpikeSlimeSmall  => match mv { 1 | 2 => 1, _ => 0 }, // TACKLE or LICK
        EnemyId::SpikeSlimeMedium => match mv { 1 | 2 => 2, _ => 0 }, // TACKLE or LICK
        EnemyId::SpikeSlimeLarge  => match mv { 1 | 2 => 3, _ => 0 },
        EnemyId::AcidSlimeMedium  => match mv { 1 => 2, _ => 0 },      // SPIT only
        EnemyId::AcidSlimeLarge   => match mv { 1 => 3, _ => 0 },
        EnemyId::SlimeBoss        => match mv { 1 => 5, _ => 0 },       // GOOP_SPRAY
        _ => 0,
    };
    for _ in 0..count {
        discard.push(Card::new(CardId::Slimed));
    }
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
        let state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 0, 0, CombatState::MAX_HP);
        assert_eq!(state.player.hand.len(), 5);
        assert_eq!(state.player.energy, 3);
        assert_eq!(state.enemies.len(), 1);
        assert!(!state.enemies[0].is_dead());
        assert_eq!(state.phase, CombatPhase::PlayerTurn);
    }

    #[test]
    fn available_actions_nonempty() {
        let state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 42, 0, CombatState::MAX_HP);
        let actions = available_actions(&state);
        assert!(actions.contains(&Action::EndTurn));
        assert!(actions.len() > 1);
    }

    #[test]
    fn end_turn_advances_turn_counter() {
        let mut state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 1, 0, CombatState::MAX_HP);
        assert_eq!(state.turn, 1);
        step(&mut state, Action::EndTurn);
        assert_eq!(state.turn, 2);
    }

    #[test]
    fn cultist_gains_strength_from_ritual() {
        let mut state = CombatState::new(ironclad_starter(), &[EnemyId::Cultist], 0, 0, CombatState::MAX_HP);
        step(&mut state, Action::EndTurn);
        assert_eq!(state.enemies[0].creature.power_amount(crate::power::PowerId::Strength), 0);
        step(&mut state, Action::EndTurn);
        assert_eq!(state.enemies[0].creature.power_amount(crate::power::PowerId::Strength), 3);
        step(&mut state, Action::EndTurn);
        assert_eq!(state.enemies[0].creature.power_amount(crate::power::PowerId::Strength), 6);
    }

    #[test]
    fn combat_can_be_won() {
        let mut state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm], 7, 0, CombatState::MAX_HP);
        let mut turns = 0;
        loop {
            if let CombatPhase::Over(_) = &state.phase { break; }
            let actions = available_actions(&state);
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

    #[test]
    fn flex_grants_and_removes_strength() {
        let mut deck = vec![Card::new(CardId::Flex)];
        for _ in 0..4 { deck.push(Card::new(CardId::Strike)); }
        for _ in 0..4 { deck.push(Card::new(CardId::Defend)); }
        deck.push(Card::new(CardId::Bash));
        let mut state = CombatState::new(deck, &[EnemyId::JawWorm], 0, 0, CombatState::MAX_HP);

        // Find Flex in hand
        let flex_idx = state.player.hand.iter().position(|c| c.id == CardId::Flex).unwrap();
        step(&mut state, Action::PlayCard { hand_idx: flex_idx, target_idx: 0 });
        assert_eq!(state.player.creature.power_amount(PowerId::Strength), 2);

        // After end of turn, Strength should be gone
        step(&mut state, Action::EndTurn);
        assert_eq!(state.player.creature.power_amount(PowerId::Strength), 0);
    }

    #[test]
    fn demon_form_grants_strength_each_turn() {
        let mut deck = vec![Card::new(CardId::DemonForm)];
        for _ in 0..9 { deck.push(Card::new(CardId::Strike)); }
        let mut state = CombatState::new(deck, &[EnemyId::JawWorm], 0, 0, CombatState::MAX_HP);

        let df_idx = state.player.hand.iter().position(|c| c.id == CardId::DemonForm).unwrap();
        step(&mut state, Action::PlayCard { hand_idx: df_idx, target_idx: 0 });

        step(&mut state, Action::EndTurn); // turn 2 starts
        assert_eq!(state.player.creature.power_amount(PowerId::Strength), 2);

        step(&mut state, Action::EndTurn); // turn 3
        assert_eq!(state.player.creature.power_amount(PowerId::Strength), 4);
    }
}
