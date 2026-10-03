use rusted_spire::{
    card::{Card, CardId},
    combat::{available_actions, step, Action, CombatState},
    enemy::EnemyId,
    obs::*,
    power::PowerId,
};

fn state(cards: &[CardId], enemies: &[EnemyId]) -> CombatState {
    CombatState::new(cards.iter().copied().map(Card::new).collect(), enemies, 42, 0, 80)
}

#[test]
fn every_legal_action_round_trips_and_every_mask_bit_is_legal() {
    let mut s = state(&[CardId::Strike, CardId::Defend, CardId::Wound], &[EnemyId::JawWorm; 3]);
    s.enemies[0].creature.hp = 0;
    let legal = available_actions(&s);
    let mask = action_mask(&s);
    assert_eq!(mask.iter().filter(|&&b| b).count(), legal.len());
    for a in &legal {
        let index = encode_action(a, &s).unwrap();
        assert!(mask[index]);
        assert_eq!(decode_action(index, &s).as_ref(), Some(a));
    }
    for (index, &allowed) in mask.iter().enumerate() {
        if allowed { assert!(legal.contains(&decode_action(index, &s).unwrap())); }
    }
    assert!(decode_action(ACTION_SIZE, &s).is_none());
}

#[test]
fn targeting_second_enemy_damages_only_that_enemy() {
    let mut s = state(&[CardId::Strike], &[EnemyId::JawWorm; 2]);
    let before = [s.enemies[0].creature.hp, s.enemies[1].creature.hp];
    assert!(action_mask(&s)[1]);
    let action = decode_action(1, &s).unwrap();
    step(&mut s, action);
    assert_eq!(s.enemies[0].creature.hp, before[0]);
    assert_eq!(s.enemies[1].creature.hp, before[1] - 6);
}

#[test]
fn untargeted_cards_have_one_action_and_work_after_enemy_death() {
    let mut s = state(&[CardId::Defend], &[EnemyId::JawWorm; 2]);
    s.enemies[0].creature.hp = 0;
    let mask = action_mask(&s);
    assert_eq!(mask.iter().filter(|&&b| b).count(), 2); // Defend, EndTurn
    assert!(mask[UNTARGETED_SLOT]);
    assert!(decode_action(0, &s).is_none());
    let action = decode_action(UNTARGETED_SLOT, &s).unwrap();
    step(&mut s, action);
    assert_eq!(s.player.creature.block, 5);
}

#[test]
fn each_enemy_has_its_own_power_features_and_dead_slots_compact() {
    let mut s = state(&[], &[EnemyId::JawWorm; 3]);
    s.enemies[0].creature.hp = 0;
    s.enemies[1].creature.apply_power(PowerId::Strength, 2);
    s.enemies[2].creature.apply_power(PowerId::Strength, 7);
    let obs = encode_obs(&s);
    assert_eq!(obs.len(), OBS_SIZE);
    assert_eq!(obs[ENEMY_OFFSET + 8], 0.2);
    assert_eq!(obs[ENEMY_OFFSET + ENEMY_FEATURES + 8], 0.7);
    assert!(obs[ENEMY_OFFSET + 2 * ENEMY_FEATURES..].iter().all(|&v| v == 0.0));
}

#[test]
fn dead_entries_do_not_hide_spawned_children() {
    let mut s = state(&[CardId::Strike], &[EnemyId::AcidSlimeMedium]);
    s.enemies[0].creature.hp = 1;
    step(&mut s, Action::PlayCard { hand_idx: 0, target_idx: 0 });
    assert_eq!(enemy_indices(&s), vec![1, 2]);
    assert!(fits_observation(&s));
    let obs = encode_obs(&s);
    assert_eq!(obs[ENEMY_OFFSET], 1.0);
    assert_eq!(obs[ENEMY_OFFSET + ENEMY_FEATURES], 1.0);
    step(&mut s, Action::EndTurn);
    assert!(action_mask(&s)[0] && action_mask(&s)[1]);
}

#[test]
fn overflow_is_detected_instead_of_truncating_enemies() {
    let s = state(&[], &[EnemyId::JawWorm; 6]);
    assert!(!fits_observation(&s));
}

#[test]
fn unplayable_status_is_not_marked_playable_even_with_energy() {
    let mut s = state(&[CardId::Wound], &[EnemyId::JawWorm]);
    s.player.energy = 100;
    assert_eq!(encode_obs(&s)[PLAYER_FEATURES + 3], 0.0);
    assert_eq!(action_mask(&s).iter().filter(|&&b| b).count(), 1);
}
