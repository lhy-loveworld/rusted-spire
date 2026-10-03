use rusted_spire::{
    card::{Card, CardId},
    combat::{step, Action, CombatState},
    creature::CreatureState,
    damage::{apply_powers, DamageType},
    enemy::{EnemyId, Intent},
    power::PowerId,
};

fn state(cards: &[CardId], enemies: &[EnemyId]) -> CombatState {
    CombatState::new(cards.iter().copied().map(Card::new).collect(), enemies, 42, 0, 80)
}

#[test]
fn vulnerable_lasts_through_enemy_attack_then_expires() {
    let mut s = state(&[], &[EnemyId::JawWorm]);
    s.player.creature.apply_power(PowerId::Vulnerable, 1);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 64); // floor(11 * 1.5)
    assert_eq!(s.player.creature.power_amount(PowerId::Vulnerable), 0);
}

#[test]
fn new_enemy_turn_debuff_survives_its_first_round() {
    let mut s = state(&[], &[EnemyId::GremlinNob, EnemyId::JawWorm]);
    s.enemies[0].next_move = 2; // Skull Bash: 6 damage then Vulnerable 2
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 58); // 6 + floor(11 * 1.5)
    assert_eq!(s.player.creature.power_amount(PowerId::Vulnerable), 2);
    s.enemies[0].next_move = 1;
    s.enemies[1].next_move = 1;
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.power_amount(PowerId::Vulnerable), 1);
}

#[test]
fn stacking_existing_debuff_does_not_reset_first_round_flag() {
    let mut c = CreatureState::new(80, 80);
    c.apply_power(PowerId::Weak, 1);
    c.apply_power_from_enemy(PowerId::Weak, 2);
    c.tick_powers_end_of_round();
    assert_eq!(c.power_amount(PowerId::Weak), 2);
}

#[test]
fn enemy_weak_affects_current_attack_then_expires() {
    let mut s = state(&[CardId::Intimidate], &[EnemyId::JawWorm]);
    step(&mut s, Action::PlayCard { hand_idx: 0, target_idx: 0 });
    assert!(matches!(s.enemies[0].intent, Intent::Attack(8)));
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 72);
    assert_eq!(s.enemies[0].creature.power_amount(PowerId::Weak), 0);
}

#[test]
fn metallicize_protects_same_turn_without_frail_reduction() {
    let mut s = state(&[CardId::Metallicize], &[EnemyId::JawWorm]);
    s.player.creature.apply_power(PowerId::Frail, 2);
    step(&mut s, Action::PlayCard { hand_idx: 0, target_idx: 0 });
    assert_eq!(s.player.creature.block, 0);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 72); // 11 - 3 block
    assert_eq!(s.player.creature.block, 0); // next player's turn
}

#[test]
fn enemy_metallicize_applies_at_end_of_turn() {
    let mut s = state(&[], &[EnemyId::Lagavulin]);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.enemies[0].creature.block, 8);
    step(&mut s, Action::EndTurn);
    assert_eq!(s.enemies[0].creature.block, 8);
}

#[test]
fn strength_precedes_weak_regardless_of_application_order() {
    let mut c = CreatureState::new(80, 80);
    c.apply_power(PowerId::Weak, 1);
    c.apply_power(PowerId::Strength, 4);
    assert_eq!(apply_powers(6, DamageType::Normal, &c.powers, &[]), 7);
}

#[test]
fn zero_amount_debuff_has_no_effect_or_negative_duration() {
    let mut c = CreatureState::new(80, 80);
    c.apply_power(PowerId::Weak, 0);
    c.tick_powers_end_of_round();
    assert!(!c.has_power(PowerId::Weak));
}

#[test]
fn ritual_and_skill_reactions_refresh_intents_without_rng() {
    let mut s = state(&[], &[EnemyId::Cultist]);
    step(&mut s, Action::EndTurn);
    assert!(matches!(s.enemies[0].intent, Intent::Attack(6)));
    step(&mut s, Action::EndTurn);
    assert!(matches!(s.enemies[0].intent, Intent::Attack(9)));
    let counter = s.rng.ai.counter;
    s.refresh_intents();
    s.refresh_intents();
    assert_eq!(s.rng.ai.counter, counter);

    let mut s = state(&[CardId::Defend], &[EnemyId::GremlinNob]);
    step(&mut s, Action::EndTurn); // Bellow
    let before = s.enemies[0].attack_damage(&s.player.creature);
    step(&mut s, Action::PlayCard { hand_idx: 0, target_idx: 0 });
    let after = match s.enemies[0].intent {
        Intent::Attack(d) | Intent::AttackDebuff(d) => d,
        _ => panic!("expected attack"),
    };
    assert_eq!(after, before + 2);
}

#[test]
fn multihit_intent_rounds_each_hit_before_multiplying() {
    let mut s = state(&[], &[EnemyId::TheGuardian]);
    s.enemies[0].next_move = 3; // four hits of 5
    s.enemies[0].creature.apply_power(PowerId::Strength, 2);
    s.enemies[0].creature.apply_power(PowerId::Weak, 1);
    s.player.creature.apply_power(PowerId::Vulnerable, 1);
    s.refresh_intents();
    assert!(matches!(s.enemies[0].intent, Intent::MultiAttack { damage: 7, hits: 4 }));
    step(&mut s, Action::EndTurn);
    assert_eq!(s.player.creature.hp, 52);
}

#[test]
fn announced_attacks_match_execution_across_supported_enemies() {
    use EnemyId::*;
    let ids = [JawWorm, Cultist, LouseNormal, LouseDefensive, FungiBeast,
        AcidSlimeSmall, AcidSlimeMedium, SpikeSlimeSmall, SpikeSlimeMedium,
        MadGremlin, SneakyGremlin, FatGremlin, ShieldGremlin, GremlinWizard,
        GremlinNob, Lagavulin, Sentry, SlimeBoss, AcidSlimeLarge, SpikeSlimeLarge, TheGuardian];
    for ascension in [0, 7, 18] {
        for id in ids {
            let mut s = CombatState::new(vec![], &[id], 42, ascension, 80);
            s.player.creature.hp = 10_000;
            s.player.creature.max_hp = 10_000;
            for _ in 0..12 {
                s.enemies[0].creature.apply_power(PowerId::Strength, 2);
                s.enemies[0].creature.apply_power(PowerId::Weak, 2);
                s.player.creature.apply_power(PowerId::Vulnerable, 2);
                s.refresh_intents();
                let announced = match s.enemies[0].intent {
                    Intent::Attack(d) | Intent::AttackDebuff(d) | Intent::AttackDefend(d) => d,
                    Intent::MultiAttack { damage, hits } => damage * i32::from(hits),
                    _ => 0,
                };
                let hp = s.player.creature.hp;
                step(&mut s, Action::EndTurn);
                assert_eq!(hp - s.player.creature.hp, announced, "{id:?} ascension {ascension}");
            }
        }
    }
}
