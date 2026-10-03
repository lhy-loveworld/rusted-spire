use crate::creature::CreatureState;
use crate::damage::{apply_powers, DamageType};
use crate::power::PowerId;
use crate::rng::Rng;

/// All implemented enemies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EnemyId {
    // Act 1 normals
    JawWorm,
    Cultist,
    LouseNormal,
    LouseDefensive,
    FungiBeast,
    AcidSlimeSmall,
    AcidSlimeMedium,
    SpikeSlimeSmall,
    SpikeSlimeMedium,
    // Act 1 Gremlin Gang
    MadGremlin,
    SneakyGremlin,
    FatGremlin,
    ShieldGremlin,
    GremlinWizard,
    // Act 1 elites
    GremlinNob,
    Lagavulin,
    Sentry,
    // Act 1 bosses
    SlimeBoss,
    AcidSlimeLarge,
    SpikeSlimeLarge,
    TheGuardian,
}

/// What the enemy intends to do this turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Intent {
    Attack(i32),
    AttackDebuff(i32),
    AttackDefend(i32),
    Buff,
    Debuff,
    Defend,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnemyState {
    pub id: EnemyId,
    pub creature: CreatureState,
    pub ascension: u8,
    pub var_damage: i32,
    pub next_move: u8,
    pub move_history: Vec<u8>,
    pub intent: Intent,
    pub death_processed: bool,
}

impl EnemyState {
    pub fn new(id: EnemyId, ascension: u8, hp_rng: &mut Rng, ai_rng: &mut Rng) -> Self {
        let (hp_min, hp_max) = hp_range(id, ascension);
        let hp = hp_rng.random_range(hp_min, hp_max);
        let var_damage = extra_damage_roll(id, ascension, hp_rng);
        let mut enemy = EnemyState {
            id,
            creature: CreatureState::new(hp, hp),
            ascension,
            var_damage,
            next_move: 0,
            move_history: vec![],
            intent: Intent::Unknown,
            death_processed: false,
        };
        apply_pre_battle(&mut enemy.creature, id, ascension, hp_rng);
        enemy.roll_move(ai_rng, true);
        enemy
    }

    pub fn is_dead(&self) -> bool { self.creature.is_dead() }

    pub fn roll_move(&mut self, ai_rng: &mut Rng, first_move: bool) {
        let roll = ai_rng.random_int(99);
        let (mv, intent) = get_move(self.id, roll, &self.move_history, first_move, &self.creature, self.var_damage, self.ascension);
        self.next_move = mv;
        self.intent = intent;
    }

    pub fn take_turn(&mut self, player_creature: &mut CreatureState, ai_rng: &mut Rng) {
        execute_move(self, player_creature);
        self.move_history.push(self.next_move);
        if self.move_history.len() > 5 { self.move_history.remove(0); }
        if !self.is_dead() {
            self.roll_move(ai_rng, false);
        }
    }

    /// Power to apply to the player when this enemy dies (e.g. FungiBeast Spore Cloud).
    pub fn on_death_effect(&self) -> Option<(PowerId, i32)> {
        match self.id {
            EnemyId::FungiBeast => Some((PowerId::Vulnerable, 2)),
            _ => None,
        }
    }

    /// Spawn child enemies when this enemy dies (slime splits, boss splits).
    pub fn spawn_on_death(&self, hp_rng: &mut Rng, ai_rng: &mut Rng) -> Vec<EnemyState> {
        match self.id {
            EnemyId::AcidSlimeMedium =>
                vec![EnemyState::new(EnemyId::AcidSlimeSmall, self.ascension, hp_rng, ai_rng),
                     EnemyState::new(EnemyId::AcidSlimeSmall, self.ascension, hp_rng, ai_rng)],
            EnemyId::SpikeSlimeMedium =>
                vec![EnemyState::new(EnemyId::SpikeSlimeSmall, self.ascension, hp_rng, ai_rng),
                     EnemyState::new(EnemyId::SpikeSlimeSmall, self.ascension, hp_rng, ai_rng)],
            EnemyId::AcidSlimeLarge =>
                vec![EnemyState::new(EnemyId::AcidSlimeMedium, self.ascension, hp_rng, ai_rng),
                     EnemyState::new(EnemyId::AcidSlimeMedium, self.ascension, hp_rng, ai_rng)],
            EnemyId::SpikeSlimeLarge =>
                vec![EnemyState::new(EnemyId::SpikeSlimeMedium, self.ascension, hp_rng, ai_rng),
                     EnemyState::new(EnemyId::SpikeSlimeMedium, self.ascension, hp_rng, ai_rng)],
            EnemyId::SlimeBoss =>
                vec![EnemyState::new(EnemyId::AcidSlimeLarge, self.ascension, hp_rng, ai_rng),
                     EnemyState::new(EnemyId::SpikeSlimeLarge, self.ascension, hp_rng, ai_rng)],
            _ => vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// HP ranges, damage rolls, pre-battle powers
// ---------------------------------------------------------------------------

fn hp_range(id: EnemyId, asc: u8) -> (i32, i32) {
    match id {
        EnemyId::JawWorm        => if asc >= 7 { (42, 46) } else { (40, 44) },
        EnemyId::Cultist        => if asc >= 7 { (50, 56) } else { (48, 54) },
        EnemyId::LouseNormal    => if asc >= 7 { (11, 16) } else { (10, 15) },
        EnemyId::LouseDefensive => if asc >= 7 { (12, 18) } else { (11, 17) },
        EnemyId::FungiBeast     => if asc >= 7 { (24, 28) } else { (22, 28) },
        EnemyId::AcidSlimeSmall => if asc >= 7 { (9, 12)  } else { (8, 12)  },
        EnemyId::AcidSlimeMedium=> if asc >= 7 { (32, 38) } else { (28, 32) },
        EnemyId::SpikeSlimeSmall=> if asc >= 7 { (11, 14) } else { (10, 14) },
        EnemyId::SpikeSlimeMedium=>if asc >= 7 { (32, 38) } else { (28, 32) },
        EnemyId::MadGremlin     => if asc >= 7 { (21, 25) } else { (20, 24) },
        EnemyId::SneakyGremlin  => if asc >= 7 { (11, 15) } else { (10, 14) },
        EnemyId::FatGremlin     => if asc >= 7 { (14, 18) } else { (13, 17) },
        EnemyId::ShieldGremlin  => if asc >= 7 { (13, 17) } else { (12, 15) },
        EnemyId::GremlinWizard  => if asc >= 7 { (24, 26) } else { (23, 25) },
        EnemyId::GremlinNob     => if asc >= 8 { (85, 90) } else { (82, 86) },
        EnemyId::Lagavulin      => if asc >= 8 { (112, 115) } else { (109, 112) },
        EnemyId::Sentry         => if asc >= 8 { (39, 45) } else { (38, 42) },
        EnemyId::SlimeBoss      => if asc >= 3 { (150, 150) } else { (140, 140) },
        EnemyId::AcidSlimeLarge => if asc >= 8 { (68, 72) } else { (65, 70) },
        EnemyId::SpikeSlimeLarge=> if asc >= 8 { (68, 72) } else { (65, 70) },
        EnemyId::TheGuardian    => if asc >= 3 { (250, 250) } else { (235, 250) },
    }
}

fn extra_damage_roll(id: EnemyId, asc: u8, rng: &mut Rng) -> i32 {
    match id {
        EnemyId::LouseNormal | EnemyId::LouseDefensive =>
            if asc >= 2 { rng.random_range(6, 8) } else { rng.random_range(5, 7) },
        EnemyId::Sentry => rng.random_int(1), // 0 or 1: stagger offset for beam/bolt cycle
        _ => 0,
    }
}

fn apply_pre_battle(creature: &mut CreatureState, id: EnemyId, asc: u8, rng: &mut Rng) {
    match id {
        EnemyId::LouseNormal | EnemyId::LouseDefensive => {
            let curl = if asc >= 17 { rng.random_range(9, 12) }
                       else if asc >= 7 { rng.random_range(4, 8) }
                       else { rng.random_range(3, 7) };
            creature.apply_power(PowerId::CurlUp, curl);
        }
        EnemyId::Lagavulin => {
            let metal = if asc >= 15 { 10 } else { 8 };
            creature.apply_power(PowerId::Metalicize, metal);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Per-enemy move constants
// ---------------------------------------------------------------------------

const JAW_WORM_CHOMP:  u8 = 1;
const JAW_WORM_BELLOW: u8 = 2;
const JAW_WORM_THRASH: u8 = 3;

const CULTIST_INCANTATION: u8 = 3;
const CULTIST_DARK_STRIKE:  u8 = 1;

const LOUSE_BITE:        u8 = 3;
const LOUSE_BUFF_DEBUFF: u8 = 4;

const FUNGI_BITE: u8 = 1;
const FUNGI_GROW: u8 = 2;

const ACID_S_TACKLE: u8 = 1;
const ACID_S_LICK:   u8 = 2;
const ACID_M_SPIT:   u8 = 1;
const ACID_M_LICK:   u8 = 2;
const ACID_M_TACKLE: u8 = 3;
const SPIKE_S_TACKLE: u8 = 1;
const SPIKE_S_LICK:   u8 = 2;
const SPIKE_M_TACKLE: u8 = 1;
const SPIKE_M_LICK:   u8 = 2;

const GREMLIN_NOB_BULL_RUSH:  u8 = 1;
const GREMLIN_NOB_SKULL_BASH: u8 = 2;
const GREMLIN_NOB_BELLOW:     u8 = 3;

const MAD_SCRATCH:      u8 = 1;
const SNEAKY_PUNCTURE:  u8 = 1;
const FAT_SMASH:        u8 = 1;
const SHIELD_PROTECT:   u8 = 1;
const WIZARD_CHARGING:  u8 = 1;
const WIZARD_ULTIMATE:  u8 = 2;

const LAG_SLEEPING:   u8 = 10;
const LAG_DEBILITATE: u8 = 11;
const LAG_MAUL:       u8 = 12;
const LAG_SIPHON:     u8 = 13;

const SENTRY_BEAM: u8 = 1;
const SENTRY_BOLT: u8 = 2;

const SLIME_BOSS_GOOP:      u8 = 1;
const SLIME_BOSS_PREPARING: u8 = 2;
const SLIME_BOSS_SLAM:      u8 = 3;

const GUARDIAN_TAIL_WHIP:  u8 = 1;
const GUARDIAN_CHARGE_UP:  u8 = 2;
const GUARDIAN_WHIRLWIND:  u8 = 3;

// ---------------------------------------------------------------------------
// Move routing
// ---------------------------------------------------------------------------

fn get_move(id: EnemyId, roll: i32, history: &[u8], first_move: bool,
            creature: &CreatureState, var_damage: i32, asc: u8) -> (u8, Intent) {
    match id {
        EnemyId::JawWorm        => jaw_worm_get_move(roll, history, first_move),
        EnemyId::Cultist        => cultist_get_move(first_move),
        EnemyId::LouseNormal    => louse_get_move(roll, history, var_damage, false),
        EnemyId::LouseDefensive => louse_get_move(roll, history, var_damage, true),
        EnemyId::FungiBeast     => fungi_get_move(roll, history),
        EnemyId::AcidSlimeSmall => acid_small_get_move(roll, history),
        EnemyId::AcidSlimeMedium=> acid_medium_get_move(roll, history),
        EnemyId::SpikeSlimeSmall=> spike_small_get_move(roll, history),
        EnemyId::SpikeSlimeMedium=>spike_medium_get_move(roll, history),
        EnemyId::MadGremlin     => (MAD_SCRATCH,     Intent::Attack(4 + if asc >= 2 { 1 } else { 0 })),
        EnemyId::SneakyGremlin  => (SNEAKY_PUNCTURE, Intent::Attack(9 + if asc >= 2 { 1 } else { 0 })),
        EnemyId::FatGremlin     => (FAT_SMASH,       Intent::AttackDebuff(5 + if asc >= 2 { 1 } else { 0 })),
        EnemyId::ShieldGremlin  => (SHIELD_PROTECT,  Intent::Defend),
        EnemyId::GremlinWizard  => wizard_get_move(history),
        EnemyId::GremlinNob     => gremlin_nob_get_move(roll, history, first_move, creature, asc),
        EnemyId::Lagavulin      => lagavulin_get_move(history, first_move, creature, asc),
        EnemyId::Sentry         => sentry_get_move(history, var_damage, asc),
        EnemyId::SlimeBoss      => slime_boss_get_move(history, first_move, asc),
        EnemyId::AcidSlimeLarge => acid_large_get_move(roll, history),
        EnemyId::SpikeSlimeLarge=> spike_large_get_move(roll, history),
        EnemyId::TheGuardian    => guardian_get_move(history, first_move, creature, asc),
    }
}

fn execute_move(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.id {
        EnemyId::JawWorm        => jaw_worm_take_turn(enemy, player),
        EnemyId::Cultist        => cultist_take_turn(enemy, player),
        EnemyId::LouseNormal    => louse_take_turn(enemy, player, false),
        EnemyId::LouseDefensive => louse_take_turn(enemy, player, true),
        EnemyId::FungiBeast     => fungi_take_turn(enemy, player),
        EnemyId::AcidSlimeSmall => acid_small_take_turn(enemy, player),
        EnemyId::AcidSlimeMedium=> acid_medium_take_turn(enemy, player),
        EnemyId::SpikeSlimeSmall=> spike_small_take_turn(enemy, player),
        EnemyId::SpikeSlimeMedium=>spike_medium_take_turn(enemy, player),
        EnemyId::MadGremlin     => { mad_gremlin_take_turn(enemy, player); }
        EnemyId::SneakyGremlin  => { sneaky_gremlin_take_turn(enemy, player); }
        EnemyId::FatGremlin     => { fat_gremlin_take_turn(enemy, player); }
        EnemyId::ShieldGremlin  => { shield_gremlin_take_turn(enemy); }
        EnemyId::GremlinWizard  => { wizard_take_turn(enemy, player); }
        EnemyId::GremlinNob     => gremlin_nob_take_turn(enemy, player),
        EnemyId::Lagavulin      => lagavulin_take_turn(enemy, player),
        EnemyId::Sentry         => sentry_take_turn(enemy, player),
        EnemyId::SlimeBoss      => slime_boss_take_turn(enemy, player),
        EnemyId::AcidSlimeLarge => acid_large_take_turn(enemy, player),
        EnemyId::SpikeSlimeLarge=> spike_large_take_turn(enemy, player),
        EnemyId::TheGuardian    => guardian_take_turn(enemy, player),
    }
}

// ---------------------------------------------------------------------------
// Jaw Worm
// ---------------------------------------------------------------------------

fn jaw_worm_get_move(roll: i32, history: &[u8], first_move: bool) -> (u8, Intent) {
    let last     = |mv| history.last() == Some(&mv);
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;

    if first_move { return (JAW_WORM_CHOMP, Intent::Attack(11)); }

    if roll < 25 {
        if last(JAW_WORM_CHOMP) { return (JAW_WORM_BELLOW, Intent::Buff); }
        return (JAW_WORM_CHOMP, Intent::Attack(11));
    }
    if roll < 55 {
        if last_two(JAW_WORM_THRASH) { return (JAW_WORM_BELLOW, Intent::Buff); }
        return (JAW_WORM_THRASH, Intent::AttackDefend(7));
    }
    if last(JAW_WORM_BELLOW) { return (JAW_WORM_CHOMP, Intent::Attack(11)); }
    (JAW_WORM_BELLOW, Intent::Buff)
}

fn jaw_worm_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        JAW_WORM_CHOMP => {
            let dmg = apply_powers(11, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        JAW_WORM_BELLOW => {
            enemy.creature.apply_power(PowerId::Strength, 3);
            enemy.creature.add_block(6);
        }
        JAW_WORM_THRASH => {
            let dmg = apply_powers(7, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            enemy.creature.add_block(5);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Cultist
// ---------------------------------------------------------------------------

fn cultist_get_move(first_move: bool) -> (u8, Intent) {
    if first_move { (CULTIST_INCANTATION, Intent::Buff) } else { (CULTIST_DARK_STRIKE, Intent::Attack(6)) }
}

fn cultist_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        CULTIST_INCANTATION => {
            let ritual = if enemy.ascension >= 2 { 4 } else { 3 };
            enemy.creature.apply_power(PowerId::Ritual, ritual);
        }
        CULTIST_DARK_STRIKE => {
            let dmg = apply_powers(6, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Louse
// ---------------------------------------------------------------------------

fn louse_get_move(roll: i32, history: &[u8], bite_dmg: i32, is_defensive: bool) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    let buff_intent = if is_defensive { Intent::Debuff } else { Intent::Buff };
    if roll < 25 {
        if last_two(LOUSE_BUFF_DEBUFF) { return (LOUSE_BITE, Intent::Attack(bite_dmg)); }
        return (LOUSE_BUFF_DEBUFF, buff_intent);
    }
    if last_two(LOUSE_BITE) { return (LOUSE_BUFF_DEBUFF, buff_intent); }
    (LOUSE_BITE, Intent::Attack(bite_dmg))
}

fn louse_take_turn(enemy: &mut EnemyState, player: &mut CreatureState, is_defensive: bool) {
    match enemy.next_move {
        LOUSE_BITE => {
            let dmg = apply_powers(enemy.var_damage, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        LOUSE_BUFF_DEBUFF => {
            if is_defensive {
                player.apply_power(PowerId::Weak, 2);
            } else {
                let str_amt = if enemy.ascension >= 17 { 4 } else { 3 };
                enemy.creature.apply_power(PowerId::Strength, str_amt);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// FungiBeast
// ---------------------------------------------------------------------------

fn fungi_get_move(roll: i32, history: &[u8]) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    if roll < 30 {
        if last_two(FUNGI_GROW) { return (FUNGI_BITE, Intent::Attack(6)); }
        return (FUNGI_GROW, Intent::Buff);
    }
    if last_two(FUNGI_BITE) { return (FUNGI_GROW, Intent::Buff); }
    (FUNGI_BITE, Intent::Attack(6))
}

fn fungi_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        FUNGI_BITE => {
            let dmg = apply_powers(6, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        FUNGI_GROW => {
            let str_amt = if enemy.ascension >= 2 { 4 } else { 3 };
            enemy.creature.apply_power(PowerId::Strength, str_amt);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Acid Slime (Small)
// ---------------------------------------------------------------------------

fn acid_small_get_move(roll: i32, history: &[u8]) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    if roll < 50 {
        if last_two(ACID_S_LICK) { return (ACID_S_TACKLE, Intent::Attack(3)); }
        return (ACID_S_LICK, Intent::Debuff);
    }
    if last_two(ACID_S_TACKLE) { return (ACID_S_LICK, Intent::Debuff); }
    (ACID_S_TACKLE, Intent::Attack(3))
}

fn acid_small_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        ACID_S_TACKLE => {
            let base = if enemy.ascension >= 2 { 4 } else { 3 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        ACID_S_LICK => {
            player.apply_power(PowerId::Weak, 1);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Acid Slime (Medium) — splits into 2× Small on death
// ---------------------------------------------------------------------------

fn acid_medium_get_move(roll: i32, history: &[u8]) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    if roll < 40 {
        if last_two(ACID_M_SPIT) { return (ACID_M_LICK, Intent::Debuff); }
        return (ACID_M_SPIT, Intent::AttackDebuff(7));
    }
    if roll < 70 {
        if last_two(ACID_M_LICK) { return (ACID_M_TACKLE, Intent::Attack(10)); }
        return (ACID_M_LICK, Intent::Debuff);
    }
    if last_two(ACID_M_TACKLE) { return (ACID_M_SPIT, Intent::AttackDebuff(7)); }
    (ACID_M_TACKLE, Intent::Attack(10))
}

fn acid_medium_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    // Note: slimed cards are added in combat.rs via take_turn context;
    // here we apply the debuff/damage effects only.
    match enemy.next_move {
        ACID_M_SPIT => {
            let base = if enemy.ascension >= 2 { 8 } else { 7 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            player.apply_power(PowerId::Weak, 1);
        }
        ACID_M_LICK => {
            player.apply_power(PowerId::Weak, 2);
        }
        ACID_M_TACKLE => {
            let base = if enemy.ascension >= 2 { 12 } else { 10 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Spike Slime (Small)
// ---------------------------------------------------------------------------

fn spike_small_get_move(roll: i32, history: &[u8]) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    if roll < 30 {
        if last_two(SPIKE_S_LICK) { return (SPIKE_S_TACKLE, Intent::AttackDebuff(5)); }
        return (SPIKE_S_LICK, Intent::Debuff);
    }
    if last_two(SPIKE_S_TACKLE) { return (SPIKE_S_LICK, Intent::Debuff); }
    (SPIKE_S_TACKLE, Intent::AttackDebuff(5))
}

fn spike_small_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SPIKE_S_TACKLE => {
            let base = if enemy.ascension >= 2 { 6 } else { 5 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            // Add Slimed to player discard
            player.apply_power(PowerId::Weak, 0); // placeholder — Slimed added in combat.rs
        }
        SPIKE_S_LICK => {
            // Add Slimed to player discard — handled in combat.rs via slimed_move check
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Spike Slime (Medium) — splits into 2× Small on death
// ---------------------------------------------------------------------------

fn spike_medium_get_move(roll: i32, history: &[u8]) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    if roll < 30 {
        if last_two(SPIKE_M_LICK) { return (SPIKE_M_TACKLE, Intent::AttackDebuff(8)); }
        return (SPIKE_M_LICK, Intent::Debuff);
    }
    if last_two(SPIKE_M_TACKLE) { return (SPIKE_M_LICK, Intent::Debuff); }
    (SPIKE_M_TACKLE, Intent::AttackDebuff(8))
}

fn spike_medium_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SPIKE_M_TACKLE => {
            let base = if enemy.ascension >= 2 { 10 } else { 8 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        SPIKE_M_LICK => { /* Slimed handled in combat.rs */ }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Gremlin (individual types)
// ---------------------------------------------------------------------------

fn mad_gremlin_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    let base = if enemy.ascension >= 2 { 5 } else { 4 };
    let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
    crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
}

fn sneaky_gremlin_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    let base = if enemy.ascension >= 2 { 10 } else { 9 };
    let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
    crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
}

fn fat_gremlin_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    let base = if enemy.ascension >= 2 { 6 } else { 5 };
    let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
    crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
    player.apply_power(PowerId::Weak, 1);
}

fn shield_gremlin_take_turn(enemy: &mut EnemyState) {
    enemy.creature.add_block(if enemy.ascension >= 7 { 12 } else { 10 });
}

fn wizard_get_move(history: &[u8]) -> (u8, Intent) {
    let charge_count = history.iter().filter(|&&m| m == WIZARD_CHARGING).count();
    if charge_count < 3 {
        (WIZARD_CHARGING, Intent::Buff) // charging up
    } else {
        (WIZARD_ULTIMATE, Intent::Attack(25))
    }
}

fn wizard_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        WIZARD_CHARGING => { /* charging, no effect */ }
        WIZARD_ULTIMATE => {
            let dmg = apply_powers(25, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            // After ultimate, reset charge history by clearing relevant entries
            enemy.move_history.retain(|&m| m != WIZARD_CHARGING);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Gremlin Nob (Elite)
// ---------------------------------------------------------------------------

fn gremlin_nob_get_move(roll: i32, history: &[u8], first_move: bool,
                        creature: &CreatureState, asc: u8) -> (u8, Intent) {
    let str_bonus = creature.power_amount(PowerId::Strength);
    let rush_dmg  = (if asc >= 3 { 16 } else { 14 }) + str_bonus;
    let bash_dmg  = (if asc >= 3 { 8  } else { 6  }) + str_bonus;
    let last_two  = |mv: u8| history.len() >= 2
        && history[history.len()-1] == mv && history[history.len()-2] == mv;

    if first_move { return (GREMLIN_NOB_BELLOW, Intent::Buff); }
    if roll < 33 || last_two(GREMLIN_NOB_BULL_RUSH) {
        return (GREMLIN_NOB_SKULL_BASH, Intent::AttackDebuff(bash_dmg));
    }
    (GREMLIN_NOB_BULL_RUSH, Intent::Attack(rush_dmg))
}

fn gremlin_nob_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        GREMLIN_NOB_BELLOW => {
            let anger = if enemy.ascension >= 18 { 3 } else { 2 };
            enemy.creature.apply_power(PowerId::Anger, anger);
        }
        GREMLIN_NOB_SKULL_BASH => {
            let base = if enemy.ascension >= 3 { 8 } else { 6 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            player.apply_power(PowerId::Vulnerable, 2);
        }
        GREMLIN_NOB_BULL_RUSH => {
            let base = if enemy.ascension >= 3 { 16 } else { 14 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Lagavulin (Elite) — Metalicize 8, sleeps 2 turns then attacks
// ---------------------------------------------------------------------------

fn lagavulin_get_move(history: &[u8], first_move: bool,
                      creature: &CreatureState, asc: u8) -> (u8, Intent) {
    let sleep_count = history.iter().filter(|&&m| m == LAG_SLEEPING).count();
    if first_move || sleep_count < 2 {
        return (LAG_SLEEPING, Intent::Unknown);
    }
    let str_bonus = creature.power_amount(PowerId::Strength);
    let maul_dmg = (if asc >= 8 { 20 } else { 18 }) + str_bonus;
    let last = history.last().copied().unwrap_or(LAG_SLEEPING);
    if last == LAG_SLEEPING { return (LAG_DEBILITATE, Intent::Debuff); }
    if last == LAG_DEBILITATE || last == LAG_SIPHON { return (LAG_MAUL, Intent::Attack(maul_dmg)); }
    (LAG_SIPHON, Intent::Debuff)
}

fn lagavulin_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        LAG_SLEEPING => { /* sleeping, no effect */ }
        LAG_DEBILITATE => {
            // Apply Weak 2 (simplified from -Str/-Dex which we don't have)
            player.apply_power(PowerId::Weak, 2);
            player.apply_power(PowerId::Frail, 2);
        }
        LAG_MAUL => {
            let base = if enemy.ascension >= 8 { 20 } else { 18 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        LAG_SIPHON => {
            player.apply_power(PowerId::Weak, 1);
            player.apply_power(PowerId::Frail, 1);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Sentry (Elite — encountered as group of 3)
// ---------------------------------------------------------------------------

fn sentry_get_move(history: &[u8], var_damage: i32, asc: u8) -> (u8, Intent) {
    let beam_dmg = if asc >= 8 { 10 } else { 9 };
    let bolt_dmg = if asc >= 8 { 30 } else { 25 };
    // Alternate Beam/Bolt; var_damage (0 or 1) staggers phase offset
    let pos = (history.len() + var_damage as usize) % 2;
    if pos == 0 {
        (SENTRY_BEAM, Intent::Attack(beam_dmg))
    } else {
        (SENTRY_BOLT, Intent::Attack(bolt_dmg))
    }
}

fn sentry_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SENTRY_BEAM => {
            let base = if enemy.ascension >= 8 { 10 } else { 9 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        SENTRY_BOLT => {
            let base = if enemy.ascension >= 8 { 30 } else { 25 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Slime Boss — splits into AcidSlimeLarge + SpikeSlimeLarge on death
// ---------------------------------------------------------------------------

fn slime_boss_get_move(history: &[u8], first_move: bool, _asc: u8) -> (u8, Intent) {
    if first_move { return (SLIME_BOSS_GOOP, Intent::Debuff); }
    let cycle = history.len() % 3;
    match cycle {
        0 => (SLIME_BOSS_GOOP,      Intent::Debuff),
        1 => (SLIME_BOSS_PREPARING, Intent::Buff),
        _ => (SLIME_BOSS_SLAM,      Intent::Attack(38)),
    }
}

fn slime_boss_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SLIME_BOSS_GOOP => {
            // Slimed cards added in combat.rs via slimed_move() hook — here just debuff intent
            let _ = player; // player receives Slimed cards, handled externally
        }
        SLIME_BOSS_PREPARING => {
            enemy.creature.add_block(if enemy.ascension >= 3 { 15 } else { 12 });
        }
        SLIME_BOSS_SLAM => {
            let base = if enemy.ascension >= 3 { 42 } else { 38 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Acid Slime Large / Spike Slime Large (from boss split)
// ---------------------------------------------------------------------------

fn acid_large_get_move(roll: i32, history: &[u8]) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    if roll < 40 {
        if last_two(ACID_M_SPIT) { return (ACID_M_LICK, Intent::Debuff); }
        return (ACID_M_SPIT, Intent::AttackDebuff(11));
    }
    if roll < 70 {
        if last_two(ACID_M_LICK) { return (ACID_M_TACKLE, Intent::Attack(14)); }
        return (ACID_M_LICK, Intent::Debuff);
    }
    if last_two(ACID_M_TACKLE) { return (ACID_M_SPIT, Intent::AttackDebuff(11)); }
    (ACID_M_TACKLE, Intent::Attack(14))
}

fn acid_large_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        ACID_M_SPIT => {
            let base = if enemy.ascension >= 2 { 12 } else { 11 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            player.apply_power(PowerId::Weak, 2);
        }
        ACID_M_LICK => { player.apply_power(PowerId::Weak, 2); }
        ACID_M_TACKLE => {
            let base = if enemy.ascension >= 2 { 16 } else { 14 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

fn spike_large_get_move(roll: i32, history: &[u8]) -> (u8, Intent) {
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;
    if roll < 30 {
        if last_two(SPIKE_M_LICK) { return (SPIKE_M_TACKLE, Intent::AttackDebuff(16)); }
        return (SPIKE_M_LICK, Intent::Debuff);
    }
    if last_two(SPIKE_M_TACKLE) { return (SPIKE_M_LICK, Intent::Debuff); }
    (SPIKE_M_TACKLE, Intent::AttackDebuff(16))
}

fn spike_large_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SPIKE_M_TACKLE => {
            let base = if enemy.ascension >= 2 { 18 } else { 16 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        SPIKE_M_LICK => { /* Slimed handled externally */ }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// The Guardian (Boss — simplified, no Mode Shift)
// ---------------------------------------------------------------------------

fn guardian_get_move(history: &[u8], first_move: bool,
                     creature: &CreatureState, asc: u8) -> (u8, Intent) {
    let str_bonus  = creature.power_amount(PowerId::Strength);
    let whirl_dmg  = (if asc >= 3 { 6 } else { 5 }) + str_bonus;
    let twip_dmg   = (if asc >= 3 { 9 } else { 8 }) + str_bonus;
    if first_move { return (GUARDIAN_TAIL_WHIP, Intent::AttackDebuff(twip_dmg)); }
    let cycle = history.len() % 3;
    match cycle {
        0 => (GUARDIAN_TAIL_WHIP, Intent::AttackDebuff(twip_dmg)),
        1 => (GUARDIAN_CHARGE_UP, Intent::Defend),
        _ => (GUARDIAN_WHIRLWIND, Intent::Attack(whirl_dmg * 4)),
    }
}

fn guardian_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        GUARDIAN_TAIL_WHIP => {
            let base = if enemy.ascension >= 3 { 9 } else { 8 };
            let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            player.apply_power(PowerId::Weak, 2);
        }
        GUARDIAN_CHARGE_UP => {
            let block = if enemy.ascension >= 3 { 20 } else { 18 };
            enemy.creature.add_block(block);
        }
        GUARDIAN_WHIRLWIND => {
            let base = if enemy.ascension >= 3 { 6 } else { 5 };
            for _ in 0..4 {
                let dmg = apply_powers(base, DamageType::Normal, &enemy.creature.powers, &player.powers);
                crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
                if player.hp <= 0 { break; }
            }
        }
        _ => {}
    }
}
