use crate::card::{card_ordinal, requires_target, CARD_COUNT};
use crate::combat::{available_actions, selection_card, Action, CombatPhase, CombatState, SelectionKind, SELECTION_PAGE_SIZE};
use crate::creature::CreatureState;
use crate::enemy::{EnemyId, Intent};
use crate::power::{PowerId, PowerState};

pub const INTERFACE_VERSION: u32 = 5;
pub const MAX_HAND: usize = crate::player::MAX_HAND;
pub const MAX_ENEMIES: usize = 5;
pub const TARGETS_PER_CARD: usize = MAX_ENEMIES + 1;
pub const UNTARGETED_SLOT: usize = MAX_ENEMIES;
pub const END_TURN_ACTION: usize = MAX_HAND * TARGETS_PER_CARD;
pub const SELECT_CARD_ACTION: usize = END_TURN_ACTION + 1;
pub const PREVIOUS_PAGE_ACTION: usize = SELECT_CARD_ACTION + SELECTION_PAGE_SIZE;
pub const NEXT_PAGE_ACTION: usize = PREVIOUS_PAGE_ACTION + 1;
pub const ACTION_SIZE: usize = NEXT_PAGE_ACTION + 1;

pub const PLAYER_FEATURES: usize = 8;
pub const HAND_FEATURES: usize = 4;
pub const POWER_FEATURES: usize = 18;
pub const ENEMY_FEATURES: usize = 8 + POWER_FEATURES;
pub const ENEMY_OFFSET: usize = PLAYER_FEATURES + MAX_HAND * HAND_FEATURES + POWER_FEATURES;
pub const SELECTION_OFFSET: usize = ENEMY_OFFSET + MAX_ENEMIES * ENEMY_FEATURES;
pub const CHOICE_OFFSET: usize = SELECTION_OFFSET + 6;
pub const CHOICE_FEATURES: usize = 4;
pub const OBS_SIZE: usize = CHOICE_OFFSET + SELECTION_PAGE_SIZE * CHOICE_FEATURES;

const POWER_SCALES: [(PowerId, f32); 14] = [
    (PowerId::Strength, 10.0), (PowerId::Vulnerable, 5.0),
    (PowerId::Weak, 5.0), (PowerId::Frail, 5.0), (PowerId::Ritual, 5.0),
    (PowerId::CurlUp, 12.0), (PowerId::Anger, 5.0), (PowerId::Metalicize, 10.0),
    (PowerId::DemonForm, 5.0), (PowerId::StrengthDown, 10.0),
    (PowerId::Artifact, 3.0),
    (PowerId::Dexterity, 10.0), (PowerId::ModeShift, 50.0), (PowerId::SharpHide, 4.0),
];

/// Observation slots enumerate living enemies in vector order. The same
/// mapping is used for target actions, even after dead enemies leave holes.
pub fn enemy_indices(state: &CombatState) -> Vec<usize> {
    state.enemies.iter().enumerate()
        .filter_map(|(i, enemy)| (!enemy.is_dead()).then_some(i)).collect()
}

pub fn fits_observation(state: &CombatState) -> bool {
    enemy_indices(state).len() <= MAX_ENEMIES && state.player.hand.len() <= MAX_HAND
}

pub fn encode_obs(state: &CombatState) -> Vec<f32> {
    assert!(fits_observation(state), "state exceeds observation capacity");
    let p = &state.player;
    let mut obs = Vec::with_capacity(OBS_SIZE);
    obs.extend_from_slice(&[
        p.creature.hp.max(0) as f32 / p.creature.max_hp as f32,
        p.creature.block as f32 / 100.0,
        p.energy as f32 / p.energy_master as f32,
        p.draw_pile.len() as f32 / 10.0,
        p.discard_pile.len() as f32 / 10.0,
        p.exhaust_pile.len() as f32 / 10.0,
        p.hand.len() as f32 / MAX_HAND as f32,
        state.turn as f32 / 100.0,
    ]);
    let legal = available_actions(state);
    for i in 0..MAX_HAND {
        if let Some(card) = p.hand.get(i) {
            let playable = legal.iter().any(|a| matches!(a, Action::PlayCard { hand_idx, .. } if *hand_idx == i));
            obs.extend_from_slice(&[
                card_ordinal(card.id) as f32 / CARD_COUNT as f32,
                card.cost as f32 / 3.0,
                if card.upgraded { 1.0 } else { 0.0 },
                if playable { 1.0 } else { 0.0 },
            ]);
        } else {
            obs.extend_from_slice(&[0.0; HAND_FEATURES]);
        }
    }
    encode_powers(&p.creature, &mut obs);
    let indices = enemy_indices(state);
    for slot in 0..MAX_ENEMIES {
        if let Some(&idx) = indices.get(slot) {
            let enemy = &state.enemies[idx];
            let (kind, damage, hits) = encode_intent(&enemy.intent);
            obs.extend_from_slice(&[
                1.0,
                enemy_ordinal(enemy.id) as f32 / 21.0,
                enemy.creature.hp as f32 / enemy.creature.max_hp as f32,
                enemy.creature.max_hp as f32 / 300.0,
                enemy.creature.block as f32 / 100.0,
                kind,
                damage / 20.0,
                hits / 4.0,
            ]);
            encode_powers(&enemy.creature, &mut obs);
        } else {
            obs.extend_from_slice(&[0.0; ENEMY_FEATURES]);
        }
    }
    if let Some(choice) = &state.selection {
        for kind in [SelectionKind::UpgradeHand, SelectionKind::ExhaustHand,
                     SelectionKind::TopdeckHand, SelectionKind::TopdeckDiscard] {
            obs.push(if choice.kind == kind { 1.0 } else { 0.0 });
        }
        obs.push(choice.page as f32 / 10.0);
        obs.push(choice.indices.len() as f32 / 10.0);
        for slot in 0..SELECTION_PAGE_SIZE {
            if let Some(card) = selection_card(state, choice.page * SELECTION_PAGE_SIZE + slot) {
                obs.extend_from_slice(&[1.0, card_ordinal(card.id) as f32 / CARD_COUNT as f32,
                    card.cost as f32 / 3.0, if card.upgraded { 1.0 } else { 0.0 }]);
            } else { obs.extend_from_slice(&[0.0; CHOICE_FEATURES]); }
        }
    } else { obs.resize(OBS_SIZE, 0.0); }
    debug_assert_eq!(obs.len(), OBS_SIZE);
    obs
}

fn encode_powers(creature: &CreatureState, obs: &mut Vec<f32>) {
    for (id, scale) in POWER_SCALES {
        obs.push(creature.power_amount(id) as f32 / scale);
    }
    for id in [PowerId::Vulnerable, PowerId::Weak, PowerId::Frail] {
        obs.push(if creature.fresh_debuffs.contains(&id) { 1.0 } else { 0.0 });
    }
    obs.push(if creature.powers.iter().any(|p| matches!(p, PowerState::Ritual(r) if r.skip_first)) { 1.0 } else { 0.0 });
}

/// Ten hand slots, each with five enemy targets and one untargeted action.
pub fn action_mask(state: &CombatState) -> Vec<bool> {
    assert!(fits_observation(state), "state exceeds observation capacity");
    let mut mask = vec![false; ACTION_SIZE];
    for action in available_actions(state) {
        if let Some(index) = encode_action(&action, state) {
            mask[index] = true;
        }
    }
    mask
}

pub fn encode_action(action: &Action, state: &CombatState) -> Option<usize> {
    match *action {
        Action::SelectCard { option_idx } => {
            let choice = state.selection.as_ref()?;
            let slot = option_idx.checked_sub(choice.page * SELECTION_PAGE_SIZE)?;
            (slot < SELECTION_PAGE_SIZE && option_idx < choice.indices.len()).then_some(SELECT_CARD_ACTION + slot)
        }
        Action::SelectionPage { page } => {
            let current = state.selection.as_ref()?.page;
            if page.checked_add(1) == Some(current) { Some(PREVIOUS_PAGE_ACTION) }
            else if current.checked_add(1) == Some(page) { Some(NEXT_PAGE_ACTION) }
            else { None }
        }
        Action::EndTurn => Some(END_TURN_ACTION),
        Action::PlayCard { hand_idx, target_idx } => {
            if hand_idx >= MAX_HAND { return None; }
            let card = state.player.hand.get(hand_idx)?;
            let slot = if requires_target(card.id) {
                enemy_indices(state).iter().position(|&idx| idx == target_idx)?
            } else {
                UNTARGETED_SLOT
            };
            if requires_target(card.id) && slot >= MAX_ENEMIES { return None; }
            Some(hand_idx * TARGETS_PER_CARD + slot)
        }
    }
}

/// Decode a structural action; legality (energy, phase, status) is checked by
/// action_mask / combat::step. Untargeted cards have only one representation.
pub fn decode_action(index: usize, state: &CombatState) -> Option<Action> {
    if state.phase == CombatPhase::SelectingCard {
        let choice = state.selection.as_ref()?;
        let action = match index {
            PREVIOUS_PAGE_ACTION => Action::SelectionPage { page: choice.page.checked_sub(1)? },
            NEXT_PAGE_ACTION => Action::SelectionPage { page: choice.page + 1 },
            i if (SELECT_CARD_ACTION..PREVIOUS_PAGE_ACTION).contains(&i) =>
                Action::SelectCard { option_idx: choice.page * SELECTION_PAGE_SIZE + i - SELECT_CARD_ACTION },
            _ => return None,
        };
        return available_actions(state).contains(&action).then_some(action);
    }
    if state.phase != CombatPhase::PlayerTurn { return None; }
    if index == END_TURN_ACTION { return Some(Action::EndTurn); }
    if index >= END_TURN_ACTION { return None; }
    let hand_idx = index / TARGETS_PER_CARD;
    let slot = index % TARGETS_PER_CARD;
    let card = state.player.hand.get(hand_idx)?;
    let target_idx = if requires_target(card.id) {
        if slot == UNTARGETED_SLOT { return None; }
        *enemy_indices(state).get(slot)?
    } else {
        if slot != UNTARGETED_SLOT { return None; }
        0
    };
    Some(Action::PlayCard { hand_idx, target_idx })
}

fn encode_intent(intent: &Intent) -> (f32, f32, f32) {
    match *intent {
        Intent::Attack(d) => (1.0 / 7.0, d as f32, 1.0),
        Intent::MultiAttack { damage, hits } => (1.0 / 7.0, damage as f32, hits as f32),
        Intent::AttackDebuff(d) => (2.0 / 7.0, d as f32, 1.0),
        Intent::AttackDefend(d) => (3.0 / 7.0, d as f32, 1.0),
        Intent::Buff => (4.0 / 7.0, 0.0, 0.0),
        Intent::Debuff => (5.0 / 7.0, 0.0, 0.0),
        Intent::Defend => (6.0 / 7.0, 0.0, 0.0),
        Intent::Split => (1.0, 0.0, 0.0),
        Intent::Sleep => (8.0 / 7.0, 0.0, 0.0),
        Intent::Stun => (9.0 / 7.0, 0.0, 0.0),
        Intent::Unknown => (0.0, 0.0, 0.0),
    }
}

fn enemy_ordinal(id: EnemyId) -> u8 {
    use EnemyId::*;
    match id {
        JawWorm => 1, Cultist => 2, LouseNormal => 3, LouseDefensive => 4,
        FungiBeast => 5, AcidSlimeSmall => 6, AcidSlimeMedium => 7,
        SpikeSlimeSmall => 8, SpikeSlimeMedium => 9, MadGremlin => 10,
        SneakyGremlin => 11, FatGremlin => 12, ShieldGremlin => 13,
        GremlinWizard => 14, GremlinNob => 15, Lagavulin => 16, Sentry => 17,
        SlimeBoss => 18, AcidSlimeLarge => 19, SpikeSlimeLarge => 20, TheGuardian => 21,
    }
}
