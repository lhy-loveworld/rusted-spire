use rusted_spire::{
    card::{Card, CardId},
    combat::{available_actions, step, Action, CombatPhase, CombatResult, CombatState},
    enemy::{EnemyId, Intent},
};
use std::io::{self, Write};

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    let deck = ironclad_starter();
    let mut state = CombatState::new(deck, &[EnemyId::JawWorm], seed);

    println!("=== Rusted Spire ===  (seed {})\n", seed);

    loop {
        print_state(&state);

        if let CombatPhase::Over(ref result) = state.phase {
            match result {
                CombatResult::Victory => println!("\n*** VICTORY ***"),
                CombatResult::Defeat  => println!("\n*** DEFEAT ***"),
            }
            break;
        }

        let actions = available_actions(&state);
        print_actions(&actions, &state);

        let choice = prompt(&actions);
        println!();
        step(&mut state, choice);
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

fn print_state(state: &CombatState) {
    let p = &state.player;
    println!("─────────────────────────────────────");
    println!(
        "Turn {}   Player  HP {}/{}  Block {}  Energy {}/{}",
        state.turn,
        p.creature.hp,
        p.creature.max_hp,
        p.creature.block,
        p.energy,
        p.energy_master,
    );

    if !p.creature.powers.is_empty() {
        let powers: Vec<String> = p.creature.powers.iter()
            .map(|pw| format!("{:?} {}", pw.id(), pw.amount()))
            .collect();
        println!("  Powers: {}", powers.join(", "));
    }

    println!("  Draw {} | Discard {}", p.draw_pile.len(), p.discard_pile.len());
    println!("  Hand:");
    for (i, card) in p.hand.iter().enumerate() {
        let name = card_name(card.id, card.upgraded);
        let affordable = if card.cost <= p.energy { "" } else { " (too costly)" };
        println!("    [{}] {}  cost {}{}", i, name, card.cost, affordable);
    }

    println!("  Enemies:");
    for (i, enemy) in state.enemies.iter().enumerate() {
        if enemy.is_dead() {
            println!("    [{}] {} — DEAD", i, enemy_name(enemy.id));
            continue;
        }
        let intent_str = intent_string(&enemy.intent);
        println!(
            "    [{}] {}  HP {}/{}  Block {}  Intent: {}",
            i,
            enemy_name(enemy.id),
            enemy.creature.hp,
            enemy.creature.max_hp,
            enemy.creature.block,
            intent_str,
        );
        if !enemy.creature.powers.is_empty() {
            let powers: Vec<String> = enemy.creature.powers.iter()
                .map(|pw| format!("{:?} {}", pw.id(), pw.amount()))
                .collect();
            println!("         Powers: {}", powers.join(", "));
        }
    }
    println!();
}

fn print_actions(actions: &[Action], state: &CombatState) {
    println!("  Actions:");
    for (i, action) in actions.iter().enumerate() {
        let desc = action_description(action, state);
        println!("    [{}] {}", i, desc);
    }
}

fn action_description(action: &Action, state: &CombatState) -> String {
    match action {
        Action::EndTurn => "End Turn".to_string(),
        Action::PlayCard { hand_idx, target_idx } => {
            let card = &state.player.hand[*hand_idx];
            let name = card_name(card.id, card.upgraded);
            if rusted_spire::card::requires_target(card.id) {
                let enemy = &state.enemies[*target_idx];
                format!("Play {} → {}", name, enemy_name(enemy.id))
            } else {
                format!("Play {}", name)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

fn prompt(actions: &[Action]) -> Action {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        let trimmed = line.trim();

        match trimmed.parse::<usize>() {
            Ok(n) if n < actions.len() => return actions[n].clone(),
            _ => println!("  Enter a number between 0 and {}", actions.len() - 1),
        }
    }
}

// ---------------------------------------------------------------------------
// Name helpers
// ---------------------------------------------------------------------------

fn card_name(id: CardId, upgraded: bool) -> String {
    let base = match id {
        CardId::Strike => "Strike",
        CardId::Defend => "Defend",
        CardId::Bash   => "Bash",
    };
    if upgraded { format!("{}+", base) } else { base.to_string() }
}

fn enemy_name(id: EnemyId) -> &'static str {
    match id {
        EnemyId::JawWorm => "Jaw Worm",
    }
}

fn intent_string(intent: &Intent) -> String {
    match intent {
        Intent::Attack(dmg)       => format!("Attack {}", dmg),
        Intent::AttackDefend(dmg) => format!("Attack {} + Block", dmg),
        Intent::Buff              => "Buff".to_string(),
        Intent::Defend            => "Defend".to_string(),
        Intent::Unknown           => "Unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Starter deck
// ---------------------------------------------------------------------------

fn ironclad_starter() -> Vec<Card> {
    let mut deck = vec![];
    for _ in 0..5 { deck.push(Card::new(CardId::Strike)); }
    for _ in 0..4 { deck.push(Card::new(CardId::Defend)); }
    deck.push(Card::new(CardId::Bash));
    deck
}
