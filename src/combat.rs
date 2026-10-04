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

        let mut enemies: Vec<_> = enemy_ids.iter()
            .map(|&id| EnemyState::construct(id, ascension, &mut rng.monster_hp)).collect();
        for (position, enemy) in enemies.iter_mut().enumerate() {
            enemy.roll_move(&mut rng.ai, true);
            enemy.set_formation_position(position);
        }
        for enemy in &mut enemies {
            enemy.pre_battle(&mut rng.monster_hp);
        }

        let mut player = PlayerState::new(starting_hp, Self::MAX_HP, 3, deck, &mut rng.shuffle);
        player.draw(crate::player::HAND_SIZE, &mut rng.shuffle);

        let mut state = CombatState {
            player,
            enemies,
            rng,
            turn: 1,
            phase: CombatPhase::PlayerTurn,
        };
        state.refresh_intents();
        state
    }

    pub fn refresh_intents(&mut self) {
        for enemy in &mut self.enemies {
            if !enemy.is_dead() {
                enemy.refresh_intent(&self.player.creature);
            }
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
    assert!(available_actions(state).contains(&action), "illegal combat action");

    match action {
        Action::PlayCard { hand_idx, target_idx } => {
            play_card(state, hand_idx, target_idx);
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
    state.refresh_intents();
    None
}

// ---------------------------------------------------------------------------
// Card execution
// ---------------------------------------------------------------------------

fn play_card(state: &mut CombatState, hand_idx: usize, target_idx: usize) {
    // A card in use is outside every pile until its effects finish.
    let card = state.player.hand.remove(hand_idx);
    state.player.energy -= card.cost;
    // onUseCard runs before queued card effects. Capture every living owner's
    // Sharp Hide now; its THORNS action survives even if this card kills it.
    let retaliation: Vec<_> = if card_type(card.id) == CardType::Attack {
        state.enemies.iter().filter(|e| !e.is_dead())
            .map(|e| e.creature.power_amount(PowerId::SharpHide)).filter(|&n| n > 0).collect()
    } else { vec![] };

    match card.id {
        // --- basics ---
        CardId::Strike => {
            let base = if card.upgraded { 9 } else { 6 };
            deal_to(state, target_idx, base);
        }
        CardId::Defend => {
            let base = if card.upgraded { 8 } else { 5 };
            state.player.creature.add_block(base);
        }
        CardId::Bash => {
            let base = if card.upgraded { 10 } else { 8 };
            let vuln = if card.upgraded { 3 } else { 2 };
            deal_to(state, target_idx, base);
            if !state.enemies[target_idx].is_dead() {
                state.enemies[target_idx].creature.apply_power(PowerId::Vulnerable, vuln);
            }
        }

        // --- attacks ---
        CardId::TwinStrike => {
            let base = if card.upgraded { 7 } else { 5 };
            deal_to(state, target_idx, base); // first hit
            // second hit
            if !state.enemies[target_idx].is_dead() {
                let attacker_powers = state.player.creature.powers.clone();
                let dmg = apply_powers(base, DamageType::Normal, &attacker_powers, &state.enemies[target_idx].creature.powers);
                let hp_lost = state.enemies[target_idx].creature.receive_damage(dmg, DamageType::Normal);
                state.enemies[target_idx].creature.trigger_on_attacked(hp_lost, DamageType::Normal);
                state.enemies[target_idx].on_hp_lost(hp_lost);
                process_death(state, target_idx);
            }
        }
        CardId::IronWave => {
            let base = if card.upgraded { 7 } else { 5 };
            state.player.creature.add_block(base);
            deal_to(state, target_idx, base);
        }
        CardId::Cleave => {
            let base = if card.upgraded { 11 } else { 8 };
            deal_all(state, base);
        }
        CardId::Clothesline => {
            let base = if card.upgraded { 14 } else { 12 };
            let weak = if card.upgraded { 3 } else { 2 };
            deal_to(state, target_idx, base);
            if !state.enemies[target_idx].is_dead() {
                state.enemies[target_idx].creature.apply_power(PowerId::Weak, weak);
            }
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
            state.enemies[target_idx].on_hp_lost(hp_lost);
            process_death(state, target_idx);
        }
        CardId::BodySlam => {
            let base = state.player.creature.block;
            deal_to(state, target_idx, base);
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
        }
        CardId::PommelStrike => {
            let base = if card.upgraded { 10 } else { 9 };
            let draw = if card.upgraded { 2 } else { 1 };
            deal_to(state, target_idx, base);
            state.player.draw(draw, &mut state.rng.shuffle);
        }
        CardId::Anger => {
            let base = if card.upgraded { 8 } else { 6 };
            let copy = card.clone();
            deal_to(state, target_idx, base);
            state.player.discard_pile.push(copy);
        }
        CardId::WildStrike => {
            let base = if card.upgraded { 17 } else { 12 };
            deal_to(state, target_idx, base);
            state.player.draw_pile.push(Card::new(CardId::Wound));
        }
        CardId::SwordBoomerang => {
            let base = 3;
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
                state.enemies[t].on_hp_lost(hp_lost);
                process_death(state, t);
            }
        }
        CardId::Dropkick => {
            let base = if card.upgraded { 8 } else { 5 };
            let vulnerable = state.enemies[target_idx].creature.power_amount(PowerId::Vulnerable) > 0;
            deal_to(state, target_idx, base);
            if vulnerable {
                state.player.energy += 1;
                state.player.draw(1, &mut state.rng.shuffle);
            }
        }

        // --- skills ---
        CardId::ShrugItOff => {
            let base = if card.upgraded { 11 } else { 8 };
            state.player.creature.add_block(base);
            state.player.draw(1, &mut state.rng.shuffle);
        }
        CardId::TrueGrit => {
            let base = if card.upgraded { 9 } else { 7 };
            state.player.creature.add_block(base);
            if !state.player.hand.is_empty() {
                let index = state.rng.card.random_int((state.player.hand.len() - 1) as i32) as usize;
                state.player.exhaust_from_hand(index);
            }
        }
        CardId::Flex => {
            let n = if card.upgraded { 4 } else { 2 };
            state.player.creature.apply_power(PowerId::Strength, n);
            state.player.creature.apply_power(PowerId::StrengthDown, n);
        }
        CardId::Intimidate => {
            let weak = if card.upgraded { 2 } else { 1 };
            let n = state.enemies.len();
            for i in 0..n {
                if !state.enemies[i].is_dead() {
                    state.enemies[i].creature.apply_power(PowerId::Weak, weak);
                }
            }
        }
        CardId::Armaments => {
            let base = if card.upgraded { 5 } else { 5 };
            state.player.creature.add_block(base);

            // Upgrade mechanic omitted for now
        }
        CardId::Warcry => {
            let draw_n = if card.upgraded { 2 } else { 1 };
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
            deal_to(state, target_idx, base);
            // Topdeck last card from discard
            if !state.player.discard_pile.is_empty() {
                let last = state.player.discard_pile.len() - 1;
                let card = state.player.discard_pile.remove(last);
                state.player.draw_pile.insert(0, card);
            }
        }
        CardId::Entrench => {
            let cur = state.player.creature.block;
            state.player.creature.block = (cur * 2).min(999);
        }

        // --- powers ---
        CardId::Inflame => {
            let n = if card.upgraded { 3 } else { 2 };
            state.player.creature.apply_power(PowerId::Strength, n);
        }
        CardId::Metallicize => {
            let n = if card.upgraded { 4 } else { 3 };
            state.player.creature.apply_power(PowerId::Metalicize, n);
        }
        CardId::DemonForm => {
            let n = if card.upgraded { 3 } else { 2 };
            state.player.creature.apply_power(PowerId::DemonForm, n);
        }

        // --- status ---
        CardId::Slimed => {}
        CardId::Wound | CardId::Dazed => {
            // Unplayable — should never reach here
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
    for damage in retaliation {
        state.player.creature.receive_damage(damage, DamageType::Thorns);
        if state.player.creature.is_dead() { break; }
    }
    // UseCardAction: powers leave combat; exhaust cards enter exhaust; all
    // other played cards reach discard only after their queued effects.
    if card_type(card.id) != CardType::Power {
        if matches!(card.id, CardId::Intimidate | CardId::Warcry | CardId::Slimed) {
            state.player.exhaust_pile.push(card);
        } else {
            state.player.discard_pile.push(card);
        }
    }
    for enemy in &mut state.enemies {
        enemy.resolve_card_reactions();
    }
}

// ---------------------------------------------------------------------------
// Damage helpers
// ---------------------------------------------------------------------------

/// Deal `base` damage to a single target, then process its death triggers.
fn deal_to(state: &mut CombatState, target_idx: usize, base: i32) {
    let attacker_powers = state.player.creature.powers.clone();
    let dmg = apply_powers(base, DamageType::Normal, &attacker_powers, &state.enemies[target_idx].creature.powers);
    let hp_lost = state.enemies[target_idx].creature.receive_damage(dmg, DamageType::Normal);
    state.enemies[target_idx].creature.trigger_on_attacked(hp_lost, DamageType::Normal);
    state.enemies[target_idx].on_hp_lost(hp_lost);
    process_death(state, target_idx);
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
            state.enemies[i].on_hp_lost(hp_lost);
        }
    }
    // Process all deaths after the full AoE sweep
    for i in 0..n {
        process_death(state, i);
    }
}

/// Interrupt surviving slimes at half HP, or process a death exactly once.
fn process_death(state: &mut CombatState, idx: usize) {
    if !state.enemies[idx].is_dead() {
        state.enemies[idx].queue_split_if_needed();
        return;
    }
    if state.enemies[idx].death_processed {
        return;
    }
    state.enemies[idx].death_processed = true;

    // On-death power applied to player
    if let Some((pid, amt)) = state.enemies[idx].on_death_effect() {
        state.player.creature.apply_power(pid, amt);
    }
}

// ---------------------------------------------------------------------------
// Turn transitions
// ---------------------------------------------------------------------------

fn end_player_turn(state: &mut CombatState) {
    state.player.creature.tick_powers_end_of_turn(true);
    state.player.end_turn();

    // Clear the previous round's block before any enemy acts. Block gained
    // during this enemy phase must survive through the next player turn.
    for enemy in &mut state.enemies {
        enemy.creature.lose_block();
    }

    let mut i = 0;
    while i < state.enemies.len() {
        if state.enemies[i].is_dead() { i += 1; continue; }
        state.enemies[i].creature.trigger_start_of_turn();
        if state.enemies[i].is_splitting() {
            let children = state.enemies[i].split_children(&mut state.rng.ai);
            // SpikeSlime_L.takeTurn queues RollMoveAction even after splitting.
            // Its now-discarded parent has Split history, so no fallback coin
            // is needed. AcidSlime_L and SlimeBoss omit this parent roll.
            if state.enemies[i].id == EnemyId::SpikeSlimeLarge {
                state.rng.ai.random_int(99);
            }
            state.enemies[i].creature.hp = 0;
            state.enemies[i].death_processed = true;
            let count = children.len();
            // Preserve formation order; newborns do not act this phase.
            state.enemies.splice(i + 1..i + 1, children);
            i += 1 + count;
            continue;
        }
        let queued_move = state.enemies[i].next_move;
        let enemy_id    = state.enemies[i].id;
        let ascension   = state.enemies[i].ascension;
        state.enemies[i].take_turn(&mut state.player.creature, &mut state.rng.ai);
        status_cards_for_move(enemy_id, queued_move, ascension, &mut state.player.discard_pile);
        if state.player.creature.is_dead() {
            return;
        }
        i += 1;
    }

    // MonsterGroup.applyEndOfTurnPowers: all monster end-turn hooks, then
    // player round-end hooks, then all monster round-end hooks.
    for enemy in &mut state.enemies {
        if !enemy.is_dead() {
            enemy.creature.tick_powers_end_of_turn(false);
        }
    }
    state.player.creature.tick_powers_end_of_round();
    for enemy in &mut state.enemies {
        if !enemy.is_dead() {
            enemy.creature.tick_powers_end_of_round();
        }
    }

    state.turn += 1;
    state.player.start_turn(&mut state.rng.shuffle);
    state.phase = CombatPhase::PlayerTurn;
}

/// Enemy-generated statuses enter discard, even if their attack was blocked.
fn status_cards_for_move(id: EnemyId, mv: u8, asc: u8, discard: &mut Vec<Card>) {
    let (card, count) = match (id, mv) {
        (EnemyId::Sentry, 2) => (CardId::Dazed, if asc >= 18 { 3 } else { 2 }),
        (EnemyId::SpikeSlimeMedium | EnemyId::AcidSlimeMedium, 1) => (CardId::Slimed, 1),
        (EnemyId::SpikeSlimeLarge | EnemyId::AcidSlimeLarge, 1) => (CardId::Slimed, 2),
        (EnemyId::SlimeBoss, 1) => (CardId::Slimed, if asc >= 19 { 5 } else { 3 }),
        _ => return,
    };
    for _ in 0..count {
        discard.push(Card::new(card));
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
    fn combat_terminates() {
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
    fn enemy_block_survives_until_next_enemy_phase() {
        let mut state = CombatState::new(vec![Card::new(CardId::Strike)], &[EnemyId::JawWorm], 0, 0, 80);
        state.enemies[0].creature.block = 20;
        state.enemies[0].next_move = 2; // Bellow: +6 block, +3 Strength
        step(&mut state, Action::EndTurn);
        assert_eq!(state.enemies[0].creature.block, 6);
        let hp = state.enemies[0].creature.hp;
        step(&mut state, Action::PlayCard { hand_idx: 0, target_idx: 0 });
        assert_eq!(state.enemies[0].creature.hp, hp);
        assert_eq!(state.enemies[0].creature.block, 0);
        state.enemies[0].creature.block = 4;
        state.enemies[0].next_move = 1; // Chomp gains no block
        step(&mut state, Action::EndTurn);
        assert_eq!(state.enemies[0].creature.block, 0);
    }

    #[test]
    fn lethal_enemy_attack_stops_round_without_drawing() {
        let mut state = CombatState::new(ironclad_starter(), &[EnemyId::JawWorm, EnemyId::JawWorm], 0, 0, 1);
        let draw_count = state.player.draw_pile.len();
        assert_eq!(step(&mut state, Action::EndTurn), Some(CombatResult::Defeat));
        assert!(state.enemies[1].move_history.is_empty());
        assert_eq!(state.turn, 1);
        assert_eq!(state.player.draw_pile.len(), draw_count);
    }

    #[test]
    fn lethal_card_wins_combat() {
        let mut state = CombatState::new(vec![Card::new(CardId::Strike)], &[EnemyId::JawWorm], 0, 0, 80);
        state.enemies[0].creature.hp = 6;
        assert_eq!(step(&mut state, Action::PlayCard { hand_idx: 0, target_idx: 0 }), Some(CombatResult::Victory));
        assert!(available_actions(&state).is_empty());
    }

    #[test]
    #[should_panic(expected = "illegal combat action")]
    fn unaffordable_card_is_rejected() {
        let mut state = CombatState::new(vec![Card::new(CardId::Bash)], &[EnemyId::JawWorm], 0, 0, 80);
        state.player.energy = 1;
        step(&mut state, Action::PlayCard { hand_idx: 0, target_idx: 0 });
    }

    #[test]
    fn flex_grants_and_removes_strength() {
        let mut deck = vec![Card::new(CardId::Flex)];
        for _ in 0..4 { deck.push(Card::new(CardId::Strike)); }
        for _ in 0..4 { deck.push(Card::new(CardId::Defend)); }
        deck.push(Card::new(CardId::Bash));
        let mut state = CombatState::new(deck, &[EnemyId::JawWorm], 0, 0, CombatState::MAX_HP);

        // This test concerns power timing, not a particular shuffled opening.
        state.player.hand = vec![Card::new(CardId::Flex)];
        let flex_idx = 0;
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

        state.player.hand = vec![Card::new(CardId::DemonForm)];
        let df_idx = 0;
        step(&mut state, Action::PlayCard { hand_idx: df_idx, target_idx: 0 });

        step(&mut state, Action::EndTurn); // turn 2 starts
        assert_eq!(state.player.creature.power_amount(PowerId::Strength), 2);

        step(&mut state, Action::EndTurn); // turn 3
        assert_eq!(state.player.creature.power_amount(PowerId::Strength), 4);
    }
}
