use rusted_spire::{
    card::{Card, CardId},
    combat::{available_actions, step, Action, CombatResult, CombatState},
    creature::CreatureState,
    enemy::{EnemyId, Intent},
    obs::{action_mask, encode_obs, enemy_indices, ENEMY_FEATURES, ENEMY_OFFSET},
    power::PowerId,
};

fn state(ids: &[EnemyId], asc: u8) -> CombatState {
    CombatState::new(vec![], ids, 42, asc, 80)
}

fn play(s: &mut CombatState, id: CardId, target: usize) -> Option<CombatResult> {
    s.player.hand = vec![Card::new(id)];
    s.player.energy = 3;
    step(s, Action::PlayCard { hand_idx: 0, target_idx: target })
}

fn status_count(s: &CombatState, id: CardId) -> usize {
    s.player.hand.iter().chain(&s.player.draw_pile)
        .chain(&s.player.discard_pile).chain(&s.player.exhaust_pile)
        .filter(|c| c.id == id).count()
}

#[test]
fn sentry_formation_alternates_damage_and_dazed_beyond_history_limit() {
    for asc in [0, 2, 3, 17, 18, 20] {
        let mut s = state(&[EnemyId::Sentry; 3], asc);
        s.player.creature.hp = 1000;
        let damage = if asc >= 3 { 10 } else { 9 };
        let dazed = if asc >= 18 { 3 } else { 2 };
        for turn in 0..12 {
            let beams = if turn % 2 == 0 { 1 } else { 2 };
            for (i, e) in s.enemies.iter().enumerate() {
                if (i + turn) % 2 == 0 {
                    assert!(matches!(e.intent, Intent::Debuff));
                    assert!(e.attack_profile().is_none());
                } else {
                    assert!(matches!(e.intent, Intent::Attack(d) if d == damage));
                }
            }
            let hp = s.player.creature.hp;
            let statuses = status_count(&s, CardId::Dazed);
            step(&mut s, Action::EndTurn);
            assert_eq!(hp - s.player.creature.hp, beams * damage);
            assert_eq!(status_count(&s, CardId::Dazed) - statuses, (3 - beams) as usize * dazed);
        }
    }
}

#[test]
fn killing_a_sentry_does_not_rephase_its_neighbors() {
    let mut s = state(&[EnemyId::Sentry; 3], 0);
    s.enemies[0].creature.hp = 1;
    play(&mut s, CardId::Strike, 0);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 71);
    assert_eq!(status_count(&s, CardId::Dazed), 2);
    assert!(matches!(s.enemies[1].intent, Intent::Debuff));
    assert!(matches!(s.enemies[2].intent, Intent::Attack(9)));
}

#[test]
fn artifact_blocks_one_debuff_but_not_the_attack_damage() {
    let mut s = state(&[EnemyId::Sentry], 0);
    let hp = s.enemies[0].creature.hp;
    assert_eq!(s.enemies[0].creature.power_amount(PowerId::Artifact), 1);
    assert_eq!(encode_obs(&s)[ENEMY_OFFSET + 18], 1.0 / 3.0);
    play(&mut s, CardId::Bash, 0);
    assert_eq!(s.enemies[0].creature.hp, hp - 8);
    assert!(!s.enemies[0].creature.has_power(PowerId::Artifact));
    assert!(!s.enemies[0].creature.has_power(PowerId::Vulnerable));
    play(&mut s, CardId::Bash, 0);
    assert_eq!(s.enemies[0].creature.hp, hp - 16);
    assert_eq!(s.enemies[0].creature.power_amount(PowerId::Vulnerable), 2);
}

#[test]
fn artifact_consumption_does_not_leave_a_fresh_debuff_flag() {
    let mut c = CreatureState::new(80, 80);
    c.apply_power(PowerId::Artifact, 2);
    c.apply_power(PowerId::Weak, 0);
    c.apply_power(PowerId::Strength, 3);
    assert_eq!(c.power_amount(PowerId::Artifact), 2);
    c.apply_power_from_enemy(PowerId::Weak, 2);
    assert!(c.fresh_debuffs.is_empty());
    assert!(!c.has_power(PowerId::Weak));
    c.apply_power(PowerId::Strength, -1);
    assert_eq!(c.power_amount(PowerId::Strength), 3);
    assert!(!c.has_power(PowerId::Artifact));
    c.apply_power_from_enemy(PowerId::Weak, 2);
    c.tick_powers_end_of_round();
    assert_eq!(c.power_amount(PowerId::Weak), 2);
}

#[test]
fn dazed_is_unplayable_and_only_exhausts_from_hand_at_turn_end() {
    let mut s = state(&[EnemyId::Cultist], 0);
    s.player.hand = vec![Card::new(CardId::Dazed), Card::new(CardId::Wound), Card::new(CardId::Slimed)];
    s.player.draw_pile = vec![Card::new(CardId::Dazed); 6];
    s.player.energy = 100;
    assert!(!available_actions(&s).iter().any(|a| matches!(a, Action::PlayCard { hand_idx: 0 | 1, .. })));
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.exhaust_pile.len(), 1);
    assert_eq!(s.player.exhaust_pile[0].id, CardId::Dazed);
    assert_eq!(s.player.draw_pile.len(), 1);
    assert!(s.player.hand.iter().all(|c| c.id == CardId::Dazed));
    assert_eq!(action_mask(&s).iter().filter(|&&b| b).count(), 1);
    assert_eq!(s.player.discard_pile.len(), 2);
}

#[test]
fn newly_generated_dazed_survives_until_the_next_player_turn_end() {
    let mut s = state(&[EnemyId::Sentry], 0);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.hand.len(), 2);
    assert!(s.player.exhaust_pile.is_empty());
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.exhaust_pile.len(), 2);
    assert!(s.player.hand.is_empty());
}

#[test]
fn split_uses_execution_hp_and_fresh_children_without_acting() {
    for (parent, children) in [
        (EnemyId::AcidSlimeLarge, [EnemyId::AcidSlimeMedium; 2]),
        (EnemyId::SpikeSlimeLarge, [EnemyId::SpikeSlimeMedium; 2]),
        (EnemyId::SlimeBoss, [EnemyId::SpikeSlimeLarge, EnemyId::AcidSlimeLarge]),
    ] {
        let mut s = state(&[parent], 18);
        s.enemies[0].creature.max_hp = 71;
        s.enemies[0].creature.hp = 41;
        s.enemies[0].creature.apply_power(PowerId::Strength, 7);
        play(&mut s, CardId::Strike, 0); // 35: floor(71 / 2)
        assert!(matches!(s.enemies[0].intent, Intent::Split));
        assert_eq!(encode_obs(&s)[ENEMY_OFFSET + 5..ENEMY_OFFSET + 8], [1.0, 0.0, 0.0]);
        assert_eq!(s.enemies.len(), 1);
        play(&mut s, CardId::Bash, 0); // 27, plus Vulnerable on parent
        step(&mut s, Action::EndTurn);
        assert_eq!(s.player.creature.hp, 80);
        let live = enemy_indices(&s);
        assert_eq!(live.len(), 2);
        for (&idx, id) in live.iter().zip(children) {
            let e = &s.enemies[idx];
            assert_eq!(e.id, id);
            assert_eq!((e.creature.hp, e.creature.max_hp), (27, 27));
            assert_eq!(e.ascension, 18);
            assert!(e.creature.powers.is_empty());
            assert!(e.move_history.is_empty());
        }
        assert_eq!(status_count(&s, CardId::Slimed), 0);
        step(&mut s, Action::EndTurn);
        assert!(live.iter().all(|&i| s.enemies[i].move_history.len() == 1));
    }
}

#[test]
fn one_hp_above_half_does_not_interrupt_and_lethal_damage_never_splits() {
    for id in [EnemyId::SlimeBoss, EnemyId::AcidSlimeLarge, EnemyId::SpikeSlimeLarge,
        EnemyId::AcidSlimeMedium, EnemyId::SpikeSlimeMedium] {
        let mut s = state(&[id], 0);
        s.enemies[0].creature.max_hp = 71;
        s.enemies[0].creature.hp = 42;
        play(&mut s, CardId::Strike, 0);
        assert!(!s.enemies[0].is_splitting());
        s.enemies[0].creature.hp = 7;
        play(&mut s, CardId::Strike, 0); // pending split only for large/boss
        assert_eq!(s.enemies[0].is_splitting(),
            matches!(id, EnemyId::SlimeBoss | EnemyId::AcidSlimeLarge | EnemyId::SpikeSlimeLarge));
        assert_eq!(play(&mut s, CardId::Strike, 0), Some(CombatResult::Victory));
        assert_eq!(s.enemies.len(), 1);
    }
}

#[test]
fn split_preserves_formation_order_and_other_enemies_still_act_once() {
    let mut s = state(&[EnemyId::JawWorm, EnemyId::AcidSlimeLarge, EnemyId::JawWorm], 0);
    s.enemies[1].creature.hp = 20;
    play(&mut s, CardId::Strike, 1);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 58);
    let ids: Vec<_> = enemy_indices(&s).iter().map(|&i| s.enemies[i].id).collect();
    assert_eq!(ids, [EnemyId::JawWorm, EnemyId::AcidSlimeMedium, EnemyId::AcidSlimeMedium, EnemyId::JawWorm]);
    let obs = encode_obs(&s);
    for i in 0..4 { assert_eq!(obs[ENEMY_OFFSET + i * ENEMY_FEATURES], 1.0); }
}

#[test]
fn two_pending_splits_resolve_in_one_phase_without_child_actions() {
    let mut s = state(&[EnemyId::AcidSlimeLarge, EnemyId::SpikeSlimeLarge], 0);
    for e in &mut s.enemies { e.creature.hp = 20; }
    play(&mut s, CardId::Cleave, 0);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 80);
    let live = enemy_indices(&s);
    assert_eq!(live.len(), 4);
    assert!(live.iter().all(|&i| s.enemies[i].creature.hp == 12 && s.enemies[i].move_history.is_empty()));
}

#[test]
fn slime_boss_cycle_and_ascension_thresholds() {
    for asc in [0, 3, 4, 8, 9, 18, 19, 20] {
        let mut s = state(&[EnemyId::SlimeBoss], asc);
        assert_eq!(s.enemies[0].creature.max_hp, if asc >= 9 { 150 } else { 140 });
        s.player.creature.hp = 1000;
        for turn in 0..12 {
            let hp = s.player.creature.hp;
            let statuses = status_count(&s, CardId::Slimed);
            step(&mut s, Action::EndTurn);
            assert_eq!(hp - s.player.creature.hp, if turn % 3 == 2 { if asc >= 4 { 38 } else { 35 } } else { 0 });
            assert_eq!(status_count(&s, CardId::Slimed) - statuses,
                if turn % 3 == 0 { if asc >= 19 { 5 } else { 3 } } else { 0 });
            assert_eq!(s.enemies[0].creature.block, 0);
        }
    }
}

#[test]
fn slime_attacks_add_correct_status_counts_without_lick_debuffs() {
    for (id, low_damage, high_damage, count) in [
        (EnemyId::AcidSlimeMedium, 7, 8, 1), (EnemyId::AcidSlimeLarge, 11, 12, 2),
        (EnemyId::SpikeSlimeSmall, 5, 6, 0), (EnemyId::SpikeSlimeMedium, 8, 10, 1),
        (EnemyId::SpikeSlimeLarge, 16, 18, 2),
    ] {
        for asc in [0, 1, 2, 17] {
            let mut s = state(&[id], asc);
            s.enemies[0].next_move = 1;
            step(&mut s, Action::EndTurn);
            assert_eq!(s.player.creature.hp, 80 - if asc >= 2 { high_damage } else { low_damage });
            assert_eq!(status_count(&s, CardId::Slimed), count);
            assert!(!s.player.creature.has_power(PowerId::Weak));
            assert!(!s.player.creature.has_power(PowerId::Frail));
        }
    }
}

#[test]
fn slime_licks_apply_debuffs_without_damage_or_status_cards() {
    for asc in [0, 16, 17] {
        for (id, power, amount) in [
            (EnemyId::AcidSlimeMedium, PowerId::Weak, 1),
            (EnemyId::AcidSlimeLarge, PowerId::Weak, 2),
            (EnemyId::SpikeSlimeMedium, PowerId::Frail, 1),
            (EnemyId::SpikeSlimeLarge, PowerId::Frail, if asc >= 17 { 3 } else { 2 }),
        ] {
            let mut s = state(&[id], asc);
            s.enemies[0].next_move = 2;
            step(&mut s, Action::EndTurn);
            assert_eq!(s.player.creature.hp, 80);
            assert_eq!(s.player.creature.power_amount(power), amount);
            assert_eq!(status_count(&s, CardId::Slimed), 0);
        }
    }
}

#[test]
fn large_acid_tackle_damage_changes_at_ascension_two() {
    for asc in [0, 1, 2, 20] {
        let mut s = state(&[EnemyId::AcidSlimeLarge], asc);
        s.enemies[0].next_move = 3;
        s.refresh_intents();
        let damage = if asc >= 2 { 18 } else { 16 };
        assert!(matches!(s.enemies[0].intent, Intent::Attack(d) | Intent::AttackDebuff(d) if d == damage));
        step(&mut s, Action::EndTurn);
        assert_eq!(s.player.creature.hp, 80 - damage);
        assert_eq!(status_count(&s, CardId::Slimed), 0);
    }
}

#[test]
fn slime_hp_ranges_change_at_ascension_seven() {
    for (id, low, high) in [
        (EnemyId::AcidSlimeSmall, (8, 12), (9, 13)),
        (EnemyId::AcidSlimeMedium, (28, 32), (29, 34)),
        (EnemyId::AcidSlimeLarge, (65, 69), (68, 72)),
        (EnemyId::SpikeSlimeSmall, (10, 14), (11, 15)),
        (EnemyId::SpikeSlimeMedium, (28, 32), (29, 34)),
        (EnemyId::SpikeSlimeLarge, (64, 70), (67, 73)),
    ] {
        for asc in [0, 6, 7, 20] {
            let expected = if asc >= 7 { high } else { low };
            let hp: Vec<_> = (0..128).map(|seed|
                CombatState::new(vec![], &[id], seed, asc, 80).enemies[0].creature.hp).collect();
            assert_eq!((*hp.iter().min().unwrap(), *hp.iter().max().unwrap()), expected, "{id:?} A{asc}");
        }
    }
}
