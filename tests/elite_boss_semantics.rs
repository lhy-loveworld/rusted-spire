//! Source-derived expectations from Lagavulin/TheGuardian and their powers.
//! These are regression tests, not execution of the original combat engine.
use rusted_spire::{
    card::{Card, CardId},
    combat::{step, Action, CombatResult, CombatState},
    creature::CreatureState,
    enemy::{EnemyId, Intent},
    obs::{encode_obs, ENEMY_OFFSET, POWER_FEATURES},
    power::PowerId,
    rng::Rng,
};

fn state(id: EnemyId, asc: u8) -> CombatState {
    CombatState::new(vec![], &[id], 42, asc, 80)
}

fn play(s: &mut CombatState, id: CardId, target_idx: usize) -> Option<CombatResult> {
    s.player.hand = vec![Card::new(id)];
    s.player.energy = 3;
    step(s, Action::PlayCard { hand_idx: 0, target_idx })
}

#[test]
fn encounter_constructors_precede_all_louse_curl_rolls() {
    for asc in [0, 2, 7, 17] {
        for seed in 0..32 {
            let s = CombatState::new(vec![], &[EnemyId::LouseNormal, EnemyId::LouseDefensive], seed, asc, 80);
            let mut hp = Rng::new(seed);
            let expected_hp0 = if asc >= 7 { hp.random_range(11, 16) } else { hp.random_range(10, 15) };
            let damage0 = if asc >= 2 { hp.random_range(6, 8) } else { hp.random_range(5, 7) };
            let expected_hp1 = if asc >= 7 { hp.random_range(12, 18) } else { hp.random_range(11, 17) };
            let damage1 = if asc >= 2 { hp.random_range(6, 8) } else { hp.random_range(5, 7) };
            for (enemy, health, damage) in [(&s.enemies[0], expected_hp0, damage0), (&s.enemies[1], expected_hp1, damage1)] {
                let curl = if asc >= 17 { hp.random_range(9, 12) }
                    else if asc >= 7 { hp.random_range(4, 8) } else { hp.random_range(3, 7) };
                assert_eq!(enemy.creature.hp, health);
                assert_eq!(enemy.var_damage, damage);
                assert_eq!(enemy.creature.power_amount(PowerId::CurlUp), curl);
            }
            assert_eq!(s.rng.monster_hp.counter, 6);
            assert_eq!(s.rng.monster_hp.copy().random_long(), hp.random_long());
        }
    }
}

#[test]
fn fixed_hp_consumes_a_roll_but_split_children_do_not() {
    for asc in [0, 8, 9, 19] {
        let s = CombatState::new(vec![], &[EnemyId::TheGuardian, EnemyId::SlimeBoss, EnemyId::Lagavulin], 42, asc, 80);
        let mut hp = Rng::new(42);
        assert_eq!(s.enemies[0].creature.hp, hp.random_int(0) + if asc >= 9 { 250 } else { 240 });
        assert_eq!(s.enemies[1].creature.hp, hp.random_int(0) + if asc >= 9 { 150 } else { 140 });
        assert_eq!(s.enemies[2].creature.hp, if asc >= 8 { hp.random_range(112, 115) } else { hp.random_range(109, 111) });
        assert_eq!(s.rng.monster_hp.copy().random_long(), hp.random_long());
    }
    let mut s = state(EnemyId::SlimeBoss, 0);
    s.enemies[0].creature.hp = 71;
    play(&mut s, CardId::Strike, 0);
    let before = s.rng.monster_hp.copy().random_long();
    step(&mut s, Action::EndTurn);
    assert_eq!(s.rng.monster_hp.copy().random_long(), before);
}

#[test]
fn wizard_hp_uses_the_java_bounds_and_one_roll() {
    for asc in [0, 6, 7, 20] {
        for seed in 0..64 {
            let s = CombatState::new(vec![], &[EnemyId::GremlinWizard], seed, asc, 80);
            let mut hp = Rng::new(seed);
            let expected = if asc >= 7 { hp.random_range(22, 26) } else { hp.random_range(21, 25) };
            assert_eq!(s.enemies[0].creature.hp, expected);
            assert_eq!(s.rng.monster_hp.counter, 1);
            assert_eq!(s.rng.monster_hp.copy().random_long(), hp.random_long());
        }
    }
}

#[test]
fn lagavulin_sleeps_three_turns_then_repeats_two_attacks_and_siphon() {
    for asc in [0, 3, 8, 18, 20] {
        let mut s = state(EnemyId::Lagavulin, asc);
        s.player.creature.hp = 1000;
        assert_eq!(s.enemies[0].creature.block, 8);
        for turn in 1..=3 {
            assert!(matches!(s.enemies[0].intent, Intent::Sleep));
            step(&mut s, Action::EndTurn);
            assert_eq!(s.player.creature.hp, 1000);
            assert_eq!(s.enemies[0].creature.block, if turn < 3 { 8 } else { 0 });
        }
        assert_eq!(s.rng.ai.counter, 3); // initial + first two idles
        let attack = if asc >= 3 { 20 } else { 18 };
        let loss = if asc >= 18 { 2 } else { 1 };
        for cycle in 1..=3 {
            for _ in 0..2 {
                assert!(matches!(s.enemies[0].intent, Intent::Attack(n) if n == attack));
                step(&mut s, Action::EndTurn);
            }
            assert!(matches!(s.enemies[0].intent, Intent::Debuff));
            step(&mut s, Action::EndTurn);
            assert_eq!(s.player.creature.power_amount(PowerId::Strength), -cycle * loss);
            assert_eq!(s.player.creature.power_amount(PowerId::Dexterity), -cycle * loss);
            assert!(!s.player.creature.has_power(PowerId::Frail));
            assert!(!s.player.creature.has_power(PowerId::Weak));
        }
        assert_eq!(s.rng.ai.counter, 12);
    }
}

#[test]
fn blocked_damage_does_not_wake_lagavulin_hp_loss_stuns_once() {
    let mut s = state(EnemyId::Lagavulin, 0);
    play(&mut s, CardId::Strike, 0);
    assert!(matches!(s.enemies[0].intent, Intent::Sleep));
    assert_eq!(s.enemies[0].creature.block, 2);
    play(&mut s, CardId::Strike, 0);
    assert!(matches!(s.enemies[0].intent, Intent::Stun));
    assert!(!s.enemies[0].creature.has_power(PowerId::Metalicize));
    let rolls = s.rng.ai.counter;
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 80);
    assert_eq!(s.rng.ai.counter, rolls + 1);
    assert!(matches!(s.enemies[0].intent, Intent::Attack(18)));
    play(&mut s, CardId::Strike, 0);
    assert!(matches!(s.enemies[0].intent, Intent::Attack(18)));
}

#[test]
fn siphon_artifact_blocks_dexterity_before_strength_and_does_not_expire() {
    let mut s = state(EnemyId::Lagavulin, 18);
    s.player.creature.apply_power(PowerId::Artifact, 1);
    s.enemies[0].next_move = 11; // Siphon
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.power_amount(PowerId::Dexterity), 0);
    assert_eq!(s.player.creature.power_amount(PowerId::Strength), -2);
    assert!(!s.player.creature.has_power(PowerId::Artifact));
    s.player.creature.tick_powers_end_of_round();
    assert_eq!(s.player.creature.power_amount(PowerId::Strength), -2);
}

#[test]
fn dexterity_precedes_frail_and_clamps_negative_card_block() {
    let mut c = CreatureState::new(80, 80);
    c.apply_power(PowerId::Frail, 2);
    c.apply_power(PowerId::Dexterity, -2);
    c.add_block(5);
    assert_eq!(c.block, 2); // floor((5 - 2) * .75)
    c.apply_power(PowerId::Dexterity, -10);
    c.add_block(5);
    assert_eq!(c.block, 2);
    c.apply_power(PowerId::Metalicize, 3);
    c.tick_powers_end_of_turn(true);
    assert_eq!(c.block, 5); // power-generated block ignores Dexterity/Frail
}

#[test]
fn guardian_offensive_cycle_and_ascension_boundaries_do_not_reroll() {
    for asc in [0, 3, 4, 8, 9, 18, 19] {
        let mut s = state(EnemyId::TheGuardian, asc);
        s.player.creature.hp = 10_000;
        assert_eq!(s.enemies[0].creature.max_hp, if asc >= 9 { 250 } else { 240 });
        assert_eq!(s.enemies[0].creature.power_amount(PowerId::ModeShift), if asc >= 19 { 40 } else if asc >= 9 { 35 } else { 30 });
        for _ in 0..3 {
            assert!(matches!(s.enemies[0].intent, Intent::Defend));
            step(&mut s, Action::EndTurn);
            assert_eq!(s.enemies[0].creature.block, 9);
            assert!(matches!(s.enemies[0].intent, Intent::Attack(n) if n == if asc >= 4 { 36 } else { 32 }));
            step(&mut s, Action::EndTurn);
            assert!(matches!(s.enemies[0].intent, Intent::Debuff));
            step(&mut s, Action::EndTurn);
            assert_eq!(s.player.creature.power_amount(PowerId::Weak), 2);
            assert_eq!(s.player.creature.power_amount(PowerId::Vulnerable), 2);
            assert!(matches!(s.enemies[0].intent, Intent::MultiAttack { damage: 7, hits: 4 }));
            step(&mut s, Action::EndTurn);
        }
        assert_eq!(s.rng.ai.counter, 1);
    }
}

#[test]
fn mode_shift_uses_hp_loss_and_defers_block_until_after_all_card_hits() {
    let mut s = state(EnemyId::TheGuardian, 0);
    s.enemies[0].creature.block = 5;
    play(&mut s, CardId::TwinStrike, 0);
    assert_eq!(s.enemies[0].creature.power_amount(PowerId::ModeShift), 25);
    s.enemies[0].creature.apply_power(PowerId::ModeShift, -20); // five HP to shift
    let hp = s.enemies[0].creature.hp;
    play(&mut s, CardId::TwinStrike, 0);
    assert_eq!(s.enemies[0].creature.hp, hp - 10); // both hits before queued block
    assert_eq!(s.enemies[0].creature.block, 20);
    assert!(!s.enemies[0].creature.has_power(PowerId::ModeShift));
    assert!(matches!(s.enemies[0].intent, Intent::Buff));
    assert_eq!(s.enemies[0].mode_shift_threshold, 40);
    assert_eq!(s.player.creature.hp, 80); // Close Up has not applied Sharp Hide yet
}

fn close_guardian(s: &mut CombatState) {
    let remaining = s.enemies[0].creature.power_amount(PowerId::ModeShift);
    s.enemies[0].creature.apply_power(PowerId::ModeShift, 1 - remaining);
    s.enemies[0].creature.block = 0;
    play(s, CardId::Strike, 0);
    step(s, Action::EndTurn);
}

#[test]
fn guardian_defensive_cycle_restores_increased_threshold_and_removes_hide() {
    for asc in [0, 4, 19] {
        let mut s = state(EnemyId::TheGuardian, asc);
        s.player.creature.hp = 1000;
        let original_threshold = s.enemies[0].mode_shift_threshold;
        for cycle in 1..=2 {
            close_guardian(&mut s);
            assert_eq!(s.enemies[0].creature.power_amount(PowerId::SharpHide), if asc >= 19 { 4 } else { 3 });
            assert!(matches!(s.enemies[0].intent, Intent::Attack(n) if n == if asc >= 4 { 10 } else { 9 }));
            step(&mut s, Action::EndTurn);
            assert!(matches!(s.enemies[0].intent, Intent::MultiAttack { damage: 8, hits: 2 }));
            step(&mut s, Action::EndTurn);
            assert!(!s.enemies[0].creature.has_power(PowerId::SharpHide));
            assert_eq!(s.enemies[0].creature.power_amount(PowerId::ModeShift), original_threshold + cycle * 10);
            assert!(matches!(s.enemies[0].intent, Intent::MultiAttack { damage: 5, hits: 4 }));
        }
        assert_eq!(s.rng.ai.counter, 1);
    }
}

#[test]
fn sharp_hide_triggers_once_per_attack_card_even_when_targeting_another_enemy() {
    let mut s = CombatState::new(vec![], &[EnemyId::TheGuardian, EnemyId::Cultist], 42, 0, 80);
    close_guardian(&mut s);
    let hp = s.player.creature.hp;
    play(&mut s, CardId::TwinStrike, 1);
    assert_eq!(s.player.creature.hp, hp - 3);
    play(&mut s, CardId::Cleave, 0);
    assert_eq!(s.player.creature.hp, hp - 6);
    play(&mut s, CardId::Defend, 0);
    assert_eq!(s.player.creature.hp, hp - 6);
    play(&mut s, CardId::Strike, 1);
    assert_eq!(s.player.creature.block, 2);
    assert_eq!(s.player.creature.hp, hp - 6);
}

#[test]
fn sharp_hide_resolves_after_card_block_and_survives_its_owners_death() {
    let mut s = state(EnemyId::TheGuardian, 0);
    close_guardian(&mut s);
    s.enemies[0].creature.hp = 1;
    s.player.creature.hp = 1;
    assert_eq!(play(&mut s, CardId::IronWave, 0), Some(CombatResult::Victory));
    assert_eq!(s.player.creature.hp, 1);
    assert_eq!(s.player.creature.block, 2);
    let mut s = state(EnemyId::TheGuardian, 0);
    close_guardian(&mut s);
    s.enemies[0].creature.hp = 1;
    s.player.creature.hp = 1;
    assert_eq!(play(&mut s, CardId::Strike, 0), Some(CombatResult::Defeat));
}

#[test]
fn lethal_followup_hit_cancels_pending_mode_shift() {
    let mut s = state(EnemyId::TheGuardian, 0);
    s.enemies[0].creature.hp = 8;
    s.enemies[0].creature.apply_power(PowerId::ModeShift, -25);
    assert_eq!(play(&mut s, CardId::TwinStrike, 0), Some(CombatResult::Victory));
    assert_eq!(s.enemies[0].creature.block, 0);
    assert!(!s.enemies[0].mode_shift_pending);
}

#[test]
fn interface_exposes_dexterity_mode_shift_hide_and_sleep_stun() {
    let mut s = state(EnemyId::TheGuardian, 0);
    s.player.creature.apply_power(PowerId::Dexterity, -2);
    let obs = encode_obs(&s);
    assert_eq!(POWER_FEATURES, 18);
    assert_eq!(obs[48 + 11], -0.2);
    assert_eq!(obs[ENEMY_OFFSET + 8 + 12], 0.6);
    close_guardian(&mut s);
    assert_eq!(encode_obs(&s)[ENEMY_OFFSET + 8 + 13], 0.75);
    let mut s = state(EnemyId::Lagavulin, 0);
    assert_eq!(encode_obs(&s)[ENEMY_OFFSET + 5], 8.0 / 7.0);
    play(&mut s, CardId::Clothesline, 0);
    assert_eq!(encode_obs(&s)[ENEMY_OFFSET + 5], 9.0 / 7.0);
}
