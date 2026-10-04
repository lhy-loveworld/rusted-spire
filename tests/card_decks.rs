//! Preset card expectations transcribed from local Java use/upgrade methods.
use rusted_spire::{
    card::{Card, CardId}, combat::{step, Action, CombatState},
    enemy::EnemyId, power::PowerId,
};

fn state(card: Card) -> CombatState {
    let mut s = CombatState::new(vec![], &[EnemyId::Cultist], 42, 0, 80);
    s.player.hand = vec![card];
    s.enemies[0].creature.hp = 1000;
    s.enemies[0].creature.max_hp = 1000;
    s
}

fn play(s: &mut CombatState) {
    step(s, Action::PlayCard { hand_idx: 0, target_idx: 0 });
}

#[test]
fn preset_attacks_match_base_and_upgrade_values() {
    for (id, base, upgraded) in [
        (CardId::Strike, 6, 9), (CardId::Bash, 8, 10),
        (CardId::TwinStrike, 10, 14), (CardId::Cleave, 8, 11),
        (CardId::HeavyBlade, 14, 14), (CardId::PommelStrike, 9, 10),
    ] {
        for (card, expected) in [(Card::new(id), base), (Card::upgraded(id), upgraded)] {
            let cost = card.cost;
            let upgraded = card.upgraded;
            let mut s = state(card);
            play(&mut s);
            assert_eq!(1000 - s.enemies[0].creature.hp, expected, "{id:?} upgraded={upgraded}");
            assert_eq!(s.player.energy, 3 - cost);
            assert_eq!(s.player.discard_pile.len(), 1);
            if id == CardId::Bash {
                assert_eq!(s.enemies[0].creature.power_amount(PowerId::Vulnerable), if upgraded { 3 } else { 2 });
            }
        }
    }
}

#[test]
fn preset_block_and_power_upgrades_match_java() {
    for (id, base, upgraded) in [(CardId::Defend, 5, 8), (CardId::ShrugItOff, 8, 11)] {
        for (card, expected) in [(Card::new(id), base), (Card::upgraded(id), upgraded)] {
            let mut s = state(card);
            play(&mut s);
            assert_eq!(s.player.creature.block, expected);
        }
    }
    for (id, power, base, upgraded) in [
        (CardId::Inflame, PowerId::Strength, 2, 3),
        (CardId::Metallicize, PowerId::Metalicize, 3, 4),
        (CardId::DemonForm, PowerId::DemonForm, 2, 3),
    ] {
        for (card, expected) in [(Card::new(id), base), (Card::upgraded(id), upgraded)] {
            let mut s = state(card);
            play(&mut s);
            assert_eq!(s.player.creature.power_amount(power), expected);
            assert!(s.player.hand.is_empty());
            assert!(s.player.discard_pile.is_empty());
            assert!(s.player.exhaust_pile.is_empty());
        }
    }
}

#[test]
fn heavy_blade_scales_positive_and_negative_strength_before_multipliers() {
    for upgraded in [false, true] {
        for strength in [-2, 3] {
            let card = if upgraded { Card::upgraded(CardId::HeavyBlade) } else { Card::new(CardId::HeavyBlade) };
            let mut s = state(card);
            s.player.creature.apply_power(PowerId::Strength, strength);
            s.player.creature.apply_power(PowerId::Weak, 1);
            s.enemies[0].creature.apply_power(PowerId::Vulnerable, 1);
            play(&mut s);
            let expected = ((14 + strength * if upgraded { 5 } else { 3 }) as f32 * 0.75 * 1.5).floor() as i32;
            assert_eq!(1000 - s.enemies[0].creature.hp, expected);
        }
    }
}

#[test]
fn drawing_card_cannot_draw_itself_from_empty_piles() {
    for id in [CardId::ShrugItOff, CardId::PommelStrike, CardId::Dropkick] {
        let mut s = state(Card::upgraded(id));
        s.enemies[0].creature.apply_power(PowerId::Vulnerable, 1);
        let counter = s.rng.shuffle.counter;
        play(&mut s);
        assert!(s.player.hand.is_empty(), "{id:?} drew itself");
        assert_eq!(s.player.discard_pile.len(), 1);
        assert_eq!(s.rng.shuffle.counter, counter);
    }
}

#[test]
fn draw_reshuffles_other_cards_before_discarding_the_played_card() {
    let mut s = state(Card::upgraded(CardId::PommelStrike));
    s.player.discard_pile = vec![Card::new(CardId::Defend), Card::upgraded(CardId::Bash)];
    play(&mut s);
    assert_eq!(s.player.hand.len(), 2);
    assert!(s.player.hand.iter().all(|c| c.id != CardId::PommelStrike));
    assert_eq!(s.player.discard_pile.len(), 1);
    assert_eq!(s.player.discard_pile[0].id, CardId::PommelStrike);
}

#[test]
fn drawing_from_full_hand_uses_the_slot_vacated_by_played_card() {
    let mut s = state(Card::new(CardId::ShrugItOff));
    s.player.hand.extend((0..9).map(|_| Card::new(CardId::Defend)));
    s.player.draw_pile = vec![Card::upgraded(CardId::Bash)];
    play(&mut s);
    assert_eq!(s.player.hand.len(), 10);
    assert_eq!(s.player.hand.last().unwrap().id, CardId::Bash);
    assert!(s.player.hand.last().unwrap().upgraded);
}

#[test]
fn exhaust_skills_leave_discard_and_entrench_doubles_block_without_modifiers() {
    for id in [CardId::Intimidate, CardId::Slimed] {
        let mut s = state(Card::new(id));
        play(&mut s);
        assert_eq!(s.player.exhaust_pile[0].id, id);
        assert!(s.player.discard_pile.is_empty());
    }
    let mut s = state(Card::upgraded(CardId::Entrench));
    s.player.creature.block = 7;
    s.player.creature.apply_power(PowerId::Frail, 2);
    s.player.creature.apply_power(PowerId::Dexterity, -3);
    play(&mut s);
    assert_eq!(s.player.creature.block, 14);
    assert_eq!(s.player.energy, 2);
}

#[test]
fn upgraded_sword_boomerang_adds_a_hit_not_damage_per_hit() {
    let mut s = state(Card::upgraded(CardId::SwordBoomerang));
    play(&mut s);
    assert_eq!(s.enemies[0].creature.hp, 988);
}

#[test]
fn true_grit_exhausts_another_card_without_recycling_itself() {
    let mut s = state(Card::new(CardId::TrueGrit));
    s.player.hand.push(Card::new(CardId::Wound));
    play(&mut s);
    assert_eq!(s.player.exhaust_pile[0].id, CardId::Wound);
    assert_eq!(s.player.discard_pile[0].id, CardId::TrueGrit);
    assert!(s.player.hand.is_empty());
}
