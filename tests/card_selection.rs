//! Source-derived expectations; see docs/CARD_SELECTION.md for Java references.
use rusted_spire::{
    card::{Card, CardId::*},
    combat::{available_actions, step, Action, CombatPhase, CombatResult, CombatState, SelectionKind},
    enemy::EnemyId, obs::*, power::PowerId,
};

fn state(cards: &[&str]) -> CombatState {
    let mut s = CombatState::new(vec![], &[EnemyId::Cultist], 42, 0, 80);
    s.player.hand = cards.iter().map(|c| Card::from_spec(c).unwrap()).collect();
    s
}
fn play(s: &mut CombatState) -> Option<CombatResult> {
    step(s, Action::PlayCard { hand_idx: 0, target_idx: 0 })
}
fn specs(cards: &[Card]) -> Vec<String> { cards.iter().map(Card::spec).collect() }

#[test]
fn armaments_filters_choices_upgrades_cost_and_defers_cleanup() {
    let mut s = state(&["Armaments", "Wound", "Entrench", "Strike+", "Defend"]);
    play(&mut s);
    assert_eq!(s.phase, CombatPhase::SelectingCard);
    assert_eq!(s.player.creature.block, 5);
    assert_eq!(s.player.energy, 2);
    assert!(s.player.discard_pile.is_empty());
    assert_eq!(s.selection.as_ref().unwrap().indices, vec![1, 3]);
    assert_eq!(available_actions(&s), vec![Action::SelectCard { option_idx: 0 }, Action::SelectCard { option_idx: 1 }]);
    let before = format!("{s:?}");
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| step(&mut s, Action::EndTurn))).is_err());
    assert_eq!(format!("{s:?}"), before);
    step(&mut s, Action::SelectCard { option_idx: 0 });
    assert_eq!(s.phase, CombatPhase::PlayerTurn);
    assert_eq!(specs(&s.player.hand), ["Defend", "Entrench+", "Wound", "Strike+"]);
    assert_eq!(s.player.hand[1].cost, 1);
    assert_eq!(specs(&s.player.discard_pile), ["Armaments"]);
    assert_eq!(s.player.energy, 2);
    assert_eq!(s.turn, 1);
    assert_eq!(s.rng.card.counter, 0);
}

#[test]
fn armaments_plus_and_zero_or_one_eligible_card_need_no_prompt() {
    for (hand, expected) in [
        (vec!["Armaments+", "Strike", "BodySlam", "Wound", "Defend+"], vec!["Strike+", "BodySlam+", "Wound", "Defend+"]),
        (vec!["Armaments", "Wound", "BodySlam"], vec!["Wound", "BodySlam+"]),
        (vec!["Armaments", "Wound", "Defend+"], vec!["Wound", "Defend+"]),
        (vec!["Armaments"], vec![]),
    ] {
        let mut s = state(&hand);
        play(&mut s);
        assert_eq!(s.phase, CombatPhase::PlayerTurn);
        assert!(s.selection.is_none());
        assert_eq!(specs(&s.player.hand), expected);
        assert_eq!(s.player.creature.block, 5);
    }
}

#[test]
fn true_grit_plus_exhausts_chosen_card_without_randomness() {
    let mut s = state(&["TrueGrit+", "Strike", "Wound"]);
    play(&mut s);
    assert_eq!(s.player.creature.block, 9);
    assert!(s.player.exhaust_pile.is_empty());
    assert_eq!(s.selection.as_ref().unwrap().kind, SelectionKind::ExhaustHand);
    step(&mut s, Action::SelectCard { option_idx: 1 });
    assert_eq!(specs(&s.player.hand), ["Strike"]);
    assert_eq!(specs(&s.player.exhaust_pile), ["Wound"]);
    assert_eq!(specs(&s.player.discard_pile), ["TrueGrit+"]);
    assert_eq!(s.rng.card.counter, 0);
}

#[test]
fn true_grit_automatic_and_random_branches_consume_correct_rng() {
    for name in ["TrueGrit", "TrueGrit+"] {
        for other in [vec![], vec!["Wound"]] {
            let mut cards = vec![name]; cards.extend(other.iter());
            let mut s = state(&cards);
            play(&mut s);
            assert_eq!(s.phase, CombatPhase::PlayerTurn);
            assert_eq!(s.player.exhaust_pile.len(), other.len());
            assert_eq!(s.rng.card.counter, 0);
        }
    }
    let mut s = state(&["TrueGrit", "Wound", "Strike"]);
    let mut rng = s.rng.card.copy();
    let index = rng.random_int(1) as usize;
    play(&mut s);
    assert_eq!(s.player.exhaust_pile[0].id, [Wound, Strike][index]);
    assert_eq!(s.rng.card.counter, 1);
}

#[test]
fn warcry_draws_then_can_topdeck_an_old_card_and_exhausts_after_choice() {
    for (name, draws) in [("Warcry", 1), ("Warcry+", 2)] {
        let mut s = state(&[name, "Defend"]);
        s.player.draw_pile = vec![Card::new(Strike), Card::new(Bash)];
        play(&mut s);
        assert_eq!(s.player.hand.len(), 1 + draws);
        assert!(s.player.exhaust_pile.is_empty());
        step(&mut s, Action::SelectCard { option_idx: 0 });
        assert_eq!(s.player.draw_pile[0].id, Defend);
        assert_eq!(specs(&s.player.exhaust_pile), [name]);
        assert_eq!(s.rng.card.counter, 0);
        s.player.draw(1, &mut s.rng.shuffle);
        assert_eq!(s.player.hand.last().unwrap().id, Defend);
    }
}

#[test]
fn warcry_empty_piles_still_topdecks_from_existing_hand() {
    let mut s = state(&["Warcry", "Strike", "Defend"]);
    play(&mut s);
    assert_eq!(s.phase, CombatPhase::SelectingCard);
    step(&mut s, Action::SelectCard { option_idx: 1 });
    assert_eq!(s.player.draw_pile[0].id, Defend);
    let mut s = state(&["Warcry", "Strike"]);
    play(&mut s);
    assert_eq!(s.phase, CombatPhase::PlayerTurn);
    assert_eq!(s.player.draw_pile[0].id, Strike);
    assert_eq!(s.rng.card.counter, 1); // Java PutOnDeckAction singleton path
    let mut s = state(&["Warcry"]);
    play(&mut s);
    assert!(s.player.draw_pile.is_empty());
    assert_eq!(specs(&s.player.exhaust_pile), ["Warcry"]);
    assert_eq!(s.rng.card.counter, 0);
}

#[test]
fn headbutt_choices_follow_damage_and_exclude_card_in_use() {
    for (name, damage) in [("Headbutt", 9), ("Headbutt+", 12)] {
        let mut s = state(&[name]);
        s.player.discard_pile = vec![Card::new(Strike), Card::upgraded(Defend)];
        let hp = s.enemies[0].creature.hp;
        play(&mut s);
        assert_eq!(s.enemies[0].creature.hp, hp - damage);
        assert_eq!(s.selection.as_ref().unwrap().indices.len(), 2);
        step(&mut s, Action::SelectCard { option_idx: 1 });
        assert_eq!(specs(&s.player.draw_pile), ["Defend+"]);
        assert_eq!(specs(&s.player.discard_pile), ["Strike", name]);
        assert_eq!(s.enemies[0].creature.hp, hp - damage);
    }
}

#[test]
fn headbutt_empty_singleton_and_lethal_hits_do_not_prompt() {
    for size in 0..=1 {
        let mut s = state(&["Headbutt"]);
        s.player.discard_pile = vec![Card::new(Defend); size];
        play(&mut s);
        assert_eq!(s.phase, CombatPhase::PlayerTurn);
        assert_eq!(s.player.draw_pile.len(), size);
        assert_eq!(specs(&s.player.discard_pile), ["Headbutt"]);
    }
    let mut s = state(&["Headbutt"]);
    s.player.discard_pile = vec![Card::new(Defend); 2];
    s.enemies[0].creature.hp = 1;
    assert_eq!(play(&mut s), Some(CombatResult::Victory));
    assert!(s.selection.is_none());
    assert!(s.player.draw_pile.is_empty());
    assert!(!action_mask(&s).iter().any(|&v| v));
}

#[test]
fn discard_pagination_reaches_every_duplicate_and_preserves_combat_and_rng() {
    let mut s = state(&["Headbutt"]);
    s.player.discard_pile = vec![Card::new(Strike); 23];
    s.player.discard_pile[22] = Card::upgraded(Defend);
    play(&mut s);
    let player = format!("{:?}", s.player);
    let rng = format!("{:?}", s.rng);
    for page in 0..3 {
        let obs = encode_obs(&s);
        let mask = action_mask(&s);
        assert_eq!(obs[SELECTION_OFFSET + 3], 1.0);
        assert_eq!(obs[SELECTION_OFFSET + 4], page as f32 / 10.0);
        assert!(!mask[END_TURN_ACTION]);
        assert_eq!(mask[PREVIOUS_PAGE_ACTION], page > 0);
        assert_eq!(mask[NEXT_PAGE_ACTION], page < 2);
        for action in available_actions(&s) {
            let encoded = encode_action(&action, &s).unwrap();
            assert_eq!(decode_action(encoded, &s), Some(action));
        }
        if page < 2 { let a = decode_action(NEXT_PAGE_ACTION, &s).unwrap(); step(&mut s, a); }
    }
    assert_eq!(format!("{:?}", s.player), player);
    assert_eq!(format!("{:?}", s.rng), rng);
    assert_eq!(s.turn, 1);
    assert!(decode_action(SELECT_CARD_ACTION + 3, &s).is_none());
    let back = decode_action(PREVIOUS_PAGE_ACTION, &s).unwrap(); step(&mut s, back);
    let next = decode_action(NEXT_PAGE_ACTION, &s).unwrap(); step(&mut s, next);
    let pick = decode_action(SELECT_CARD_ACTION + 2, &s).unwrap(); step(&mut s, pick);
    assert_eq!(specs(&s.player.draw_pile), ["Defend+"]);
    assert!(encode_obs(&s)[SELECTION_OFFSET..].iter().all(|&x| x == 0.0));
}

#[test]
fn selection_completion_resolves_reactions_once_including_defeat() {
    let mut s = state(&["TrueGrit+", "Strike", "Defend"]);
    s.enemies[0].creature.apply_power(PowerId::Anger, 2);
    play(&mut s);
    assert_eq!(s.enemies[0].creature.power_amount(PowerId::Strength), 2);
    step(&mut s, Action::SelectCard { option_idx: 0 });
    assert_eq!(s.enemies[0].creature.power_amount(PowerId::Strength), 2);
    let mut s = state(&["Headbutt"]);
    s.enemies[0].creature.apply_power(PowerId::SharpHide, 3);
    s.player.creature.hp = 1;
    s.player.discard_pile = vec![Card::new(Strike); 2];
    play(&mut s);
    assert_eq!(step(&mut s, Action::SelectCard { option_idx: 0 }), Some(CombatResult::Defeat));
    assert!(s.selection.is_none());
    assert!(!action_mask(&s).iter().any(|&x| x));
}

#[test]
fn warcry_respects_full_hand_and_reshuffles_without_drawing_itself() {
    let mut s = state(&["Warcry+", "Strike", "Strike", "Strike", "Strike",
        "Defend", "Defend", "Defend", "Defend", "Defend"]);
    s.player.draw_pile = vec![Card::new(Bash), Card::new(Inflame)];
    play(&mut s);
    assert_eq!(s.player.hand.len(), 10);
    assert_eq!(specs(&s.player.draw_pile), ["Inflame"]);
    step(&mut s, Action::SelectCard { option_idx: 9 });
    assert_eq!(specs(&s.player.draw_pile), ["Bash", "Inflame"]);
    let mut s = state(&["Warcry+", "Defend"]);
    s.player.discard_pile = vec![Card::new(Strike)];
    play(&mut s);
    assert_eq!(specs(&s.player.hand), ["Defend", "Strike"]);
    assert!(s.player.discard_pile.is_empty());
    assert_eq!(s.rng.shuffle.counter, 2); // initial shuffle and reshuffle
    step(&mut s, Action::SelectCard { option_idx: 0 });
    assert_eq!(specs(&s.player.exhaust_pile), ["Warcry+"]);
}
