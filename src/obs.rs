use crate::card::{card_ordinal, CARD_COUNT};
use crate::combat::{available_actions, Action, CombatState};
use crate::enemy::Intent;
use crate::power::PowerId;

// Fixed dimensions — must stay in sync with train.py constants.
pub const MAX_HAND: usize = 10;
pub const MAX_ENEMIES: usize = 5;

// Breakdown:
//   player:           4
//   hand slots:       MAX_HAND * 4 = 40
//   enemy slots:      MAX_ENEMIES * 5 = 25
//   player powers:    6  (Strength, Vulnerable, Weak, Frail, Metalicize, DemonForm)
//   enemy powers:     8  (Strength, Vulnerable, Weak, Frail, Ritual, CurlUp, Anger, Metalicize)
pub const OBS_SIZE: usize = 4 + MAX_HAND * 4 + MAX_ENEMIES * 5 + 6 + 8;

pub const ACTION_SIZE: usize = MAX_HAND + 1;
pub const END_TURN_ACTION: usize = MAX_HAND;

pub fn encode_obs(state: &CombatState) -> Vec<f32> {
    let mut obs = Vec::with_capacity(OBS_SIZE);
    let p = &state.player;

    // --- Player (4) ---
    obs.push(p.creature.hp as f32 / p.creature.max_hp as f32);
    obs.push(p.creature.block as f32 / 100.0);
    obs.push(p.energy as f32 / p.energy_master as f32);
    obs.push(p.draw_pile.len() as f32 / 10.0);

    // --- Hand slots (MAX_HAND * 4) ---
    for i in 0..MAX_HAND {
        if let Some(card) = p.hand.get(i) {
            obs.push(card_ordinal(card.id) as f32 / CARD_COUNT as f32);
            obs.push(card.cost as f32 / 3.0);
            obs.push(if card.upgraded { 1.0 } else { 0.0 });
            obs.push(if card.cost <= p.energy { 1.0 } else { 0.0 });
        } else {
            obs.extend_from_slice(&[0.0, 0.0, 0.0, 0.0]);
        }
    }

    // --- Enemy slots (MAX_ENEMIES * 5) ---
    for i in 0..MAX_ENEMIES {
        if let Some(enemy) = state.enemies.get(i) {
            if enemy.is_dead() {
                obs.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 0.0]);
            } else {
                obs.push(1.0);
                obs.push(enemy.creature.hp as f32 / enemy.creature.max_hp as f32);
                obs.push(enemy.creature.block as f32 / 100.0);
                let (intent_type, intent_dmg) = encode_intent(&enemy.intent);
                obs.push(intent_type);
                obs.push(intent_dmg);
            }
        } else {
            obs.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 0.0]);
        }
    }

    // --- Player powers (6) ---
    obs.push(p.creature.power_amount(PowerId::Strength)   as f32 / 10.0);
    obs.push(p.creature.power_amount(PowerId::Vulnerable) as f32 /  5.0);
    obs.push(p.creature.power_amount(PowerId::Weak)       as f32 /  5.0);
    obs.push(p.creature.power_amount(PowerId::Frail)      as f32 /  5.0);
    obs.push(p.creature.power_amount(PowerId::Metalicize) as f32 / 10.0);
    obs.push(p.creature.power_amount(PowerId::DemonForm)  as f32 /  5.0);

    // --- First enemy powers (8) ---
    let e = state.enemies.first();
    obs.push(e.map(|e| e.creature.power_amount(PowerId::Strength)   as f32 / 10.0).unwrap_or(0.0));
    obs.push(e.map(|e| e.creature.power_amount(PowerId::Vulnerable) as f32 /  5.0).unwrap_or(0.0));
    obs.push(e.map(|e| e.creature.power_amount(PowerId::Weak)       as f32 /  5.0).unwrap_or(0.0));
    obs.push(e.map(|e| e.creature.power_amount(PowerId::Frail)      as f32 /  5.0).unwrap_or(0.0));
    obs.push(e.map(|e| e.creature.power_amount(PowerId::Ritual)     as f32 /  5.0).unwrap_or(0.0));
    obs.push(e.map(|e| e.creature.power_amount(PowerId::CurlUp)     as f32 / 12.0).unwrap_or(0.0));
    obs.push(e.map(|e| e.creature.power_amount(PowerId::Anger)      as f32 /  5.0).unwrap_or(0.0));
    obs.push(e.map(|e| e.creature.power_amount(PowerId::Metalicize) as f32 / 12.0).unwrap_or(0.0));

    debug_assert_eq!(obs.len(), OBS_SIZE);
    obs
}

/// Returns a boolean mask aligned to ACTION_SIZE.
pub fn action_mask(state: &CombatState) -> Vec<bool> {
    let legal = available_actions(state);
    let mut mask = vec![false; ACTION_SIZE];

    for action in &legal {
        match action {
            Action::EndTurn => mask[END_TURN_ACTION] = true,
            Action::PlayCard { hand_idx, .. } => {
                if *hand_idx < MAX_HAND {
                    mask[*hand_idx] = true;
                }
            }
        }
    }
    mask
}

/// Map an integer action index back to an `Action`.
/// Always targets the first living enemy for card plays.
pub fn decode_action(action_idx: usize, state: &CombatState) -> Action {
    if action_idx == END_TURN_ACTION {
        return Action::EndTurn;
    }
    let hand_idx = action_idx;
    let target_idx = state.enemies.iter().position(|e| !e.is_dead()).unwrap_or(0);
    Action::PlayCard { hand_idx, target_idx }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn encode_intent(intent: &Intent) -> (f32, f32) {
    match intent {
        Intent::Attack(dmg)       => (1.0 / 6.0, *dmg as f32 / 20.0),
        Intent::AttackDebuff(dmg) => (2.0 / 6.0, *dmg as f32 / 20.0),
        Intent::AttackDefend(dmg) => (3.0 / 6.0, *dmg as f32 / 20.0),
        Intent::Buff              => (4.0 / 6.0, 0.0),
        Intent::Debuff            => (5.0 / 6.0, 0.0),
        Intent::Defend            => (6.0 / 6.0, 0.0),
        Intent::Unknown           => (0.0,        0.0),
    }
}
