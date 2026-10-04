//! Recorded original-JAR action traces. No game installation is needed for CI.
use rusted_spire::{
    card::Card,
    combat::{step, selection_card, Action, CombatPhase, CombatState, SelectionKind},
    creature::CreatureState, enemy::EnemyId, power::PowerId, rng::RngBundle,
};

fn cards(specs: &str) -> Vec<Card> {
    if specs == "-" { vec![] } else { specs.split(',').map(|s| Card::from_spec(s).unwrap()).collect() }
}
fn power_id(name: &str) -> PowerId {
    match name {
        "Strength" => PowerId::Strength, "Dexterity" => PowerId::Dexterity,
        "Weak" => PowerId::Weak, "Vulnerable" => PowerId::Vulnerable,
        "Frail" => PowerId::Frail, "CurlUp" => PowerId::CurlUp,
        "Anger" => PowerId::Anger, "SharpHide" => PowerId::SharpHide,
        _ => panic!("unmapped fixture power {name}"),
    }
}
fn powers(creature: &mut CreatureState, specs: &str) {
    if specs != "-" {
        for spec in specs.split(',') {
            let (name, amount) = spec.split_once(':').unwrap();
            creature.apply_power(power_id(name), amount.parse().unwrap());
        }
    }
}
fn joined(values: Vec<String>) -> String {
    if values.is_empty() { "-".into() } else { values.join(",") }
}
fn card_list(cards: &[Card]) -> String {
    joined(cards.iter().map(|c| format!("{}:{}", c.spec(), c.cost)).collect())
}
fn power_list(creature: &CreatureState) -> String {
    let mut result: Vec<_> = creature.powers.iter().map(|p| format!("{:?}:{}", p.id(), p.amount())).collect();
    result.sort(); joined(result)
}
fn snapshot(s: &CombatState, case: &str, index: usize) -> Vec<String> {
    let phase = match &s.phase {
        CombatPhase::PlayerTurn => "play",
        CombatPhase::SelectingCard => match s.selection.as_ref().unwrap().kind {
            SelectionKind::UpgradeHand => "upgrade", SelectionKind::ExhaustHand => "exhaust",
            SelectionKind::TopdeckHand => "topdeck_hand", SelectionKind::TopdeckDiscard => "topdeck_discard",
        },
        CombatPhase::Over(_) => panic!("fixture unexpectedly ended combat"),
    };
    let p = &s.player;
    let choices = s.selection.as_ref().map(|c| card_list(&(0..c.indices.len())
        .map(|i| selection_card(s, i).unwrap().clone()).collect::<Vec<_>>())).unwrap_or("-".into());
    vec![case.into(), index.to_string(), phase.into(), p.creature.hp.to_string(),
        p.creature.block.to_string(), p.energy.to_string(), card_list(&p.hand),
        card_list(&p.draw_pile), card_list(&p.discard_pile), card_list(&p.exhaust_pile),
        s.enemies[0].creature.hp.to_string(), s.enemies[0].creature.block.to_string(),
        power_list(&p.creature), power_list(&s.enemies[0].creature),
        s.rng.card.counter.to_string(), s.rng.shuffle.counter.to_string(), choices]
}

#[test]
fn controlled_actions_match_original_jar_snapshots() {
    let rows = |text: &'static str| text.lines().filter(|l| !l.starts_with('#') && !l.is_empty());
    let expected: Vec<_> = rows(include_str!("fixtures/java_combat.tsv")).collect();
    let directives: Vec<_> = rows(include_str!("fixtures/combat_scenarios.tsv")).collect();
    assert_eq!(directives.len(), expected.len(), "every directive needs an oracle snapshot");
    let fields = ["case", "step", "phase", "hp", "block", "energy", "hand", "draw", "discard", "exhaust",
        "enemy_hp", "enemy_block", "player_powers", "enemy_powers", "card_rng", "shuffle_rng", "choices"];
    let mut state = None;
    let mut case = "";
    let mut index = 0;
    let mut mismatches = vec![];
    for (directive, expected) in directives.iter().zip(expected) {
        let row: Vec<_> = directive.split('\t').collect();
        match row[0] {
            "case" => {
                case = row[1]; index = 0;
                let seed = row[2].parse().unwrap();
                let mut s = CombatState::new(vec![], &[EnemyId::Cultist], seed, 0, 80);
                // Controlled setup bypasses both games' constructors/opening draw.
                s.rng = RngBundle::new(seed);
                s.player.hand = cards(row[3]); s.player.draw_pile = cards(row[4]);
                s.player.discard_pile = cards(row[5]);
                powers(&mut s.player.creature, row[6]); powers(&mut s.enemies[0].creature, row[7]);
                s.player.creature.block = row[8].parse().unwrap();
                s.enemies[0].creature.block = row[9].parse().unwrap();
                s.enemies[0].creature.hp = 200; s.enemies[0].creature.max_hp = 200;
                state = Some(s);
            }
            "play" | "choose" => {
                let action = if row[0] == "play" { Action::PlayCard { hand_idx: row[1].parse().unwrap(), target_idx: 0 } }
                    else { Action::SelectCard { option_idx: row[1].parse().unwrap() } };
                step(state.as_mut().unwrap(), action); index += 1;
            }
            other => panic!("unknown directive {other}"),
        }
        let actual = snapshot(state.as_ref().unwrap(), case, index);
        let expected: Vec<_> = expected.split('\t').collect();
        assert_eq!(expected.len(), fields.len());
        for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            if actual != expected { mismatches.push(format!("{case} step {index} {}: Rust={actual}, Java={expected}", fields[i])); }
        }
    }
    assert!(mismatches.is_empty(), "original bytecode divergences:\n{}", mismatches.join("\n"));
}
