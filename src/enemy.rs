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
    MultiAttack { damage: i32, hits: u8 },
    AttackDebuff(i32),
    AttackDefend(i32),
    Buff,
    Debuff,
    Defend,
    Split,
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


    /// Base damage and hit count for the queued move. Shared by execution and
    /// intent calculation, including ascension-dependent damage.
    pub fn attack_profile(&self) -> Option<(i32, u8)> {
        use EnemyId::*;
        let asc = self.ascension;
        let base = match (self.id, self.next_move) {
            (JawWorm, JAW_WORM_CHOMP) => 11,
            (JawWorm, JAW_WORM_THRASH) => 7,
            (Cultist, CULTIST_DARK_STRIKE) => 6,
            (LouseNormal | LouseDefensive, LOUSE_BITE) => self.var_damage,
            (FungiBeast, FUNGI_BITE) => 6,
            (AcidSlimeSmall, ACID_S_TACKLE) => if asc >= 2 { 4 } else { 3 },
            (AcidSlimeMedium, ACID_M_SPIT) => if asc >= 2 { 8 } else { 7 },
            (AcidSlimeMedium, ACID_M_TACKLE) => if asc >= 2 { 12 } else { 10 },
            (SpikeSlimeSmall, SPIKE_S_TACKLE) => if asc >= 2 { 6 } else { 5 },
            (SpikeSlimeMedium, SPIKE_M_TACKLE) => if asc >= 2 { 10 } else { 8 },
            (MadGremlin, MAD_SCRATCH) => if asc >= 2 { 5 } else { 4 },
            (SneakyGremlin, SNEAKY_PUNCTURE) => if asc >= 2 { 10 } else { 9 },
            (FatGremlin, FAT_SMASH) => if asc >= 2 { 6 } else { 5 },
            (GremlinWizard, WIZARD_ULTIMATE) => 25,
            (GremlinNob, GREMLIN_NOB_SKULL_BASH) => if asc >= 3 { 8 } else { 6 },
            (GremlinNob, GREMLIN_NOB_BULL_RUSH) => if asc >= 3 { 16 } else { 14 },
            (Lagavulin, LAG_MAUL) => if asc >= 8 { 20 } else { 18 },
            (Sentry, SENTRY_BEAM) => if asc >= 3 { 10 } else { 9 },
            (SlimeBoss, SLIME_BOSS_SLAM) => if asc >= 4 { 38 } else { 35 },
            (AcidSlimeLarge, ACID_M_SPIT) => if asc >= 2 { 12 } else { 11 },
            (AcidSlimeLarge, ACID_M_TACKLE) => if asc >= 2 { 18 } else { 16 },
            (SpikeSlimeLarge, SPIKE_M_TACKLE) => if asc >= 2 { 18 } else { 16 },
            (TheGuardian, GUARDIAN_TAIL_WHIP) => if asc >= 3 { 9 } else { 8 },
            (TheGuardian, GUARDIAN_WHIRLWIND) =>
                return Some((if asc >= 3 { 6 } else { 5 }, 4)),
            _ => return None,
        };
        Some((base, 1))
    }

    pub fn attack_damage(&self, player: &CreatureState) -> i32 {
        let (base, _) = self.attack_profile().expect("queued move must be an attack");
        apply_powers(base, DamageType::Normal, &self.creature.powers, &player.powers)
    }

    /// Refresh without rerolling the move or consuming RNG.
    pub fn refresh_intent(&mut self, player: &CreatureState) {
        if let Some((_, hits)) = self.attack_profile() {
            let damage = self.attack_damage(player);
            self.intent = if hits > 1 {
                Intent::MultiAttack { damage, hits }
            } else {
                match self.intent {
                    Intent::AttackDebuff(_) => Intent::AttackDebuff(damage),
                    Intent::AttackDefend(_) => Intent::AttackDefend(damage),
                    _ => Intent::Attack(damage),
                }
            };
        }
    }

    pub fn is_dead(&self) -> bool { self.creature.is_dead() }

    pub fn roll_move(&mut self, ai_rng: &mut Rng, first_move: bool) {
        let roll = ai_rng.random_int(99);
        let (mv, intent) = get_move(self.id, roll, &self.move_history, first_move, &self.creature, self.var_damage, self.ascension, ai_rng);
        self.next_move = mv;
        self.intent = intent;
    }

    pub fn take_turn(&mut self, player_creature: &mut CreatureState, ai_rng: &mut Rng) {
        execute_move(self, player_creature);
        self.move_history.push(self.next_move);
        if self.move_history.len() > 5 { self.move_history.remove(0); }
        if !self.is_dead() {
            match self.id {
                // These Java takeTurn methods set the next move directly.
                EnemyId::AcidSlimeSmall => {
                    (self.next_move, self.intent) = if self.next_move == ACID_S_TACKLE {
                        (ACID_S_LICK, Intent::Debuff)
                    } else {
                        (ACID_S_TACKLE, Intent::Attack(if self.ascension >= 2 { 4 } else { 3 }))
                    };
                }
                EnemyId::SlimeBoss => {
                    (self.next_move, self.intent) = slime_boss_get_move(&self.move_history, false, self.ascension);
                }
                _ => self.roll_move(ai_rng, false),
            }
        }
    }

    /// Power to apply to the player when this enemy dies (e.g. FungiBeast Spore Cloud).
    pub fn on_death_effect(&self) -> Option<(PowerId, i32)> {
        match self.id {
            EnemyId::FungiBeast => Some((PowerId::Vulnerable, 2)),
            _ => None,
        }
    }

    /// Sentries alternate by their original formation position, without an RNG roll.
    pub fn set_formation_position(&mut self, position: usize) {
        if self.id == EnemyId::Sentry {
            (self.next_move, self.intent) = if position % 2 == 0 {
                (SENTRY_BOLT, Intent::Debuff)
            } else {
                (SENTRY_BEAM, Intent::Attack(if self.ascension >= 3 { 10 } else { 9 }))
            };
        }
    }

    /// A surviving large slime interrupts its queued move at half HP.
    pub fn queue_split_if_needed(&mut self) {
        if !self.is_dead() && self.creature.hp <= self.creature.max_hp / 2
            && matches!(self.id, EnemyId::SlimeBoss | EnemyId::AcidSlimeLarge | EnemyId::SpikeSlimeLarge) {
            self.next_move = SLIME_SPLIT;
            self.intent = Intent::Split;
        }
    }

    pub fn is_splitting(&self) -> bool {
        self.next_move == SLIME_SPLIT
            && matches!(self.id, EnemyId::SlimeBoss | EnemyId::AcidSlimeLarge | EnemyId::SpikeSlimeLarge)
    }

    /// Split at execution-time HP. Children have fresh powers and that HP as max HP.
    pub fn split_children(&self, ai_rng: &mut Rng) -> Vec<EnemyState> {
        assert!(self.is_splitting() && !self.is_dead());
        let ids = match self.id {
            EnemyId::AcidSlimeLarge => [EnemyId::AcidSlimeMedium; 2],
            EnemyId::SpikeSlimeLarge => [EnemyId::SpikeSlimeMedium; 2],
            EnemyId::SlimeBoss => [EnemyId::SpikeSlimeLarge, EnemyId::AcidSlimeLarge],
            _ => unreachable!(),
        };
        ids.into_iter().map(|id| {
            let mut child = EnemyState {
                id, creature: CreatureState::new(self.creature.hp, self.creature.hp),
                ascension: self.ascension, var_damage: 0, next_move: 0,
                move_history: vec![], intent: Intent::Unknown, death_processed: false,
            };
            child.roll_move(ai_rng, true);
            child
        }).collect()
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
        EnemyId::AcidSlimeSmall => if asc >= 7 { (9, 13)  } else { (8, 12)  },
        EnemyId::AcidSlimeMedium=> if asc >= 7 { (29, 34) } else { (28, 32) },
        EnemyId::SpikeSlimeSmall=> if asc >= 7 { (11, 15) } else { (10, 14) },
        EnemyId::SpikeSlimeMedium=>if asc >= 7 { (29, 34) } else { (28, 32) },
        EnemyId::MadGremlin     => if asc >= 7 { (21, 25) } else { (20, 24) },
        EnemyId::SneakyGremlin  => if asc >= 7 { (11, 15) } else { (10, 14) },
        EnemyId::FatGremlin     => if asc >= 7 { (14, 18) } else { (13, 17) },
        EnemyId::ShieldGremlin  => if asc >= 7 { (13, 17) } else { (12, 15) },
        EnemyId::GremlinWizard  => if asc >= 7 { (24, 26) } else { (23, 25) },
        EnemyId::GremlinNob     => if asc >= 8 { (85, 90) } else { (82, 86) },
        EnemyId::Lagavulin      => if asc >= 8 { (112, 115) } else { (109, 112) },
        EnemyId::Sentry         => if asc >= 8 { (39, 45) } else { (38, 42) },
        EnemyId::SlimeBoss      => if asc >= 9 { (150, 150) } else { (140, 140) },
        EnemyId::AcidSlimeLarge => if asc >= 7 { (68, 72) } else { (65, 69) },
        EnemyId::SpikeSlimeLarge=> if asc >= 7 { (67, 73) } else { (64, 70) },
        EnemyId::TheGuardian    => if asc >= 3 { (250, 250) } else { (235, 250) },
    }
}

fn extra_damage_roll(id: EnemyId, asc: u8, rng: &mut Rng) -> i32 {
    match id {
        EnemyId::LouseNormal | EnemyId::LouseDefensive =>
            if asc >= 2 { rng.random_range(6, 8) } else { rng.random_range(5, 7) },
        _ => 0,
    }
}

fn apply_pre_battle(creature: &mut CreatureState, id: EnemyId, asc: u8, rng: &mut Rng) {
    match id {
        EnemyId::Sentry => creature.apply_power(PowerId::Artifact, 1),
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
const SLIME_SPLIT:          u8 = 4;

const GUARDIAN_TAIL_WHIP:  u8 = 1;
const GUARDIAN_CHARGE_UP:  u8 = 2;
const GUARDIAN_WHIRLWIND:  u8 = 3;

// ---------------------------------------------------------------------------
// Move routing
// ---------------------------------------------------------------------------

fn get_move(id: EnemyId, roll: i32, history: &[u8], first_move: bool,
            creature: &CreatureState, var_damage: i32, asc: u8, ai_rng: &mut Rng) -> (u8, Intent) {
    match id {
        EnemyId::JawWorm        => jaw_worm_get_move(roll, history, first_move),
        EnemyId::Cultist        => cultist_get_move(first_move),
        EnemyId::LouseNormal    => louse_get_move(roll, history, var_damage, false),
        EnemyId::LouseDefensive => louse_get_move(roll, history, var_damage, true),
        EnemyId::FungiBeast     => fungi_get_move(roll, history),
        EnemyId::AcidSlimeSmall => acid_small_get_move(history, asc, ai_rng),
        EnemyId::AcidSlimeMedium=> acid_get_move(roll, history, asc, false, ai_rng),
        EnemyId::SpikeSlimeSmall=> spike_small_get_move(roll, history),
        EnemyId::SpikeSlimeMedium=>spike_get_move(roll, history, asc, false),
        EnemyId::MadGremlin     => (MAD_SCRATCH,     Intent::Attack(4 + if asc >= 2 { 1 } else { 0 })),
        EnemyId::SneakyGremlin  => (SNEAKY_PUNCTURE, Intent::Attack(9 + if asc >= 2 { 1 } else { 0 })),
        EnemyId::FatGremlin     => (FAT_SMASH,       Intent::AttackDebuff(5 + if asc >= 2 { 1 } else { 0 })),
        EnemyId::ShieldGremlin  => (SHIELD_PROTECT,  Intent::Defend),
        EnemyId::GremlinWizard  => wizard_get_move(history),
        EnemyId::GremlinNob     => gremlin_nob_get_move(roll, history, first_move, creature, asc),
        EnemyId::Lagavulin      => lagavulin_get_move(history, first_move, creature, asc),
        EnemyId::Sentry         => sentry_get_move(history, asc),
        EnemyId::SlimeBoss      => slime_boss_get_move(history, first_move, asc),
        EnemyId::AcidSlimeLarge => acid_get_move(roll, history, asc, true, ai_rng),
        EnemyId::SpikeSlimeLarge=> spike_get_move(roll, history, asc, true),
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
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        JAW_WORM_BELLOW => {
            enemy.creature.apply_power(PowerId::Strength, 3);
            enemy.creature.add_block(6);
        }
        JAW_WORM_THRASH => {
            let dmg = enemy.attack_damage(player);
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
            let dmg = enemy.attack_damage(player);
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
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        LOUSE_BUFF_DEBUFF => {
            if is_defensive {
                player.apply_power_from_enemy(PowerId::Weak, 2);
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
            let dmg = enemy.attack_damage(player);
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

fn acid_small_get_move(history: &[u8], asc: u8, ai_rng: &mut Rng) -> (u8, Intent) {
    let tackle = if asc >= 17 {
        history.ends_with(&[ACID_S_TACKLE, ACID_S_TACKLE])
    } else {
        ai_rng.random_bool()
    };
    if tackle {
        (ACID_S_TACKLE, Intent::Attack(if asc >= 2 { 4 } else { 3 }))
    } else {
        (ACID_S_LICK, Intent::Debuff)
    }
}

fn acid_small_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        ACID_S_TACKLE => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        ACID_S_LICK => {
            player.apply_power_from_enemy(PowerId::Weak, 1);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Acid Slime (Medium)
// ---------------------------------------------------------------------------

/// AcidSlime_M/L.getMove: branch thresholds and fallback coin flips differ at A17.
fn acid_get_move(roll: i32, history: &[u8], asc: u8, large: bool, ai_rng: &mut Rng) -> (u8, Intent) {
    let hard = asc >= 17;
    let last = |mv| history.last() == Some(&mv);
    let twice = |mv| history.ends_with(&[mv, mv]);
    let spit_threshold = if hard { 40 } else { 30 };
    let tackle_threshold = if hard && !large { 80 } else { 70 };
    let mv = if roll < spit_threshold {
        if twice(ACID_M_SPIT) {
            let tackle = if hard && large { ai_rng.random_bool_chance(0.6) }
                         else { ai_rng.random_bool() };
            if tackle { ACID_M_TACKLE } else { ACID_M_LICK }
        } else { ACID_M_SPIT }
    } else if roll < tackle_threshold {
        if if hard { twice(ACID_M_TACKLE) } else { last(ACID_M_TACKLE) } {
            let chance = if !hard { 0.4 } else if large { 0.6 } else { 0.5 };
            if ai_rng.random_bool_chance(chance) { ACID_M_SPIT } else { ACID_M_LICK }
        } else { ACID_M_TACKLE }
    } else if if hard { last(ACID_M_LICK) } else { twice(ACID_M_LICK) } {
        if ai_rng.random_bool_chance(0.4) { ACID_M_SPIT } else { ACID_M_TACKLE }
    } else { ACID_M_LICK };
    let (spit, tackle) = match (large, asc >= 2) {
        (false, false) => (7, 10), (false, true) => (8, 12),
        (true, false) => (11, 16), (true, true) => (12, 18),
    };
    match mv {
        ACID_M_SPIT => (mv, Intent::AttackDebuff(spit)),
        ACID_M_TACKLE => (mv, Intent::Attack(tackle)),
        _ => (mv, Intent::Debuff),
    }
}

fn acid_medium_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    // Note: slimed cards are added in combat.rs via take_turn context;
    // here we apply the debuff/damage effects only.
    match enemy.next_move {
        ACID_M_SPIT => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        ACID_M_LICK => {
            player.apply_power_from_enemy(PowerId::Weak, 1);
        }
        ACID_M_TACKLE => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Spike Slime (Small)
// ---------------------------------------------------------------------------

fn spike_small_get_move(_roll: i32, _history: &[u8]) -> (u8, Intent) {
    (SPIKE_S_TACKLE, Intent::Attack(5))
}

fn spike_small_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SPIKE_S_TACKLE => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Spike Slime (Medium)
// ---------------------------------------------------------------------------

/// SpikeSlime_M/L.getMove: 30% Tackle, 70% Lick before repeat restrictions.
fn spike_get_move(roll: i32, history: &[u8], asc: u8, large: bool) -> (u8, Intent) {
    let tackle = if roll < 30 {
        !history.ends_with(&[SPIKE_M_TACKLE, SPIKE_M_TACKLE])
    } else if asc >= 17 {
        history.last() == Some(&SPIKE_M_LICK)
    } else {
        history.ends_with(&[SPIKE_M_LICK, SPIKE_M_LICK])
    };
    if tackle {
        let damage = match (large, asc >= 2) {
            (false, false) => 8, (false, true) => 10,
            (true, false) => 16, (true, true) => 18,
        };
        (SPIKE_M_TACKLE, Intent::AttackDebuff(damage))
    } else {
        (SPIKE_M_LICK, Intent::Debuff)
    }
}

fn spike_medium_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SPIKE_M_TACKLE => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        SPIKE_M_LICK => { player.apply_power_from_enemy(PowerId::Frail, 1); }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Gremlin (individual types)
// ---------------------------------------------------------------------------

fn mad_gremlin_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    let dmg = enemy.attack_damage(player);
    crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
}

fn sneaky_gremlin_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    let dmg = enemy.attack_damage(player);
    crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
}

fn fat_gremlin_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    let dmg = enemy.attack_damage(player);
    crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
    player.apply_power_from_enemy(PowerId::Weak, 1);
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
            let dmg = enemy.attack_damage(player);
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
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            player.apply_power_from_enemy(PowerId::Vulnerable, 2);
        }
        GREMLIN_NOB_BULL_RUSH => {
            let dmg = enemy.attack_damage(player);
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
            player.apply_power_from_enemy(PowerId::Weak, 2);
            player.apply_power_from_enemy(PowerId::Frail, 2);
        }
        LAG_MAUL => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        LAG_SIPHON => {
            player.apply_power_from_enemy(PowerId::Weak, 1);
            player.apply_power_from_enemy(PowerId::Frail, 1);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Sentry (Elite — encountered as group of 3)
// ---------------------------------------------------------------------------

fn sentry_get_move(history: &[u8], asc: u8) -> (u8, Intent) {
    if history.last() == Some(&SENTRY_BOLT) {
        (SENTRY_BEAM, Intent::Attack(if asc >= 3 { 10 } else { 9 }))
    } else {
        (SENTRY_BOLT, Intent::Debuff)
    }
}

fn sentry_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SENTRY_BEAM => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        SENTRY_BOLT => { /* Dazed cards added by combat.rs */ }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Slime Boss — prepares a split at half HP
// ---------------------------------------------------------------------------

fn slime_boss_get_move(history: &[u8], first_move: bool, asc: u8) -> (u8, Intent) {
    if first_move { return (SLIME_BOSS_GOOP, Intent::Debuff); }
    match history.last().copied() {
        Some(SLIME_BOSS_GOOP) => (SLIME_BOSS_PREPARING, Intent::Unknown),
        Some(SLIME_BOSS_PREPARING) => (SLIME_BOSS_SLAM, Intent::Attack(if asc >= 4 { 38 } else { 35 })),
        _ => (SLIME_BOSS_GOOP, Intent::Debuff),
    }
}

fn slime_boss_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SLIME_BOSS_GOOP => {
            // Slimed cards added in combat.rs.
            let _ = player; // player receives Slimed cards, handled externally
        }
        SLIME_BOSS_PREPARING => {}
        SLIME_BOSS_SLAM => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Acid Slime Large / Spike Slime Large (from boss split)
// ---------------------------------------------------------------------------



fn acid_large_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        ACID_M_SPIT => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        ACID_M_LICK => { player.apply_power_from_enemy(PowerId::Weak, 2); }
        ACID_M_TACKLE => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        _ => {}
    }
}



fn spike_large_take_turn(enemy: &mut EnemyState, player: &mut CreatureState) {
    match enemy.next_move {
        SPIKE_M_TACKLE => {
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
        }
        SPIKE_M_LICK => { player.apply_power_from_enemy(PowerId::Frail, if enemy.ascension >= 17 { 3 } else { 2 }); }
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
            let dmg = enemy.attack_damage(player);
            crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
            player.apply_power_from_enemy(PowerId::Weak, 2);
        }
        GUARDIAN_CHARGE_UP => {
            let block = if enemy.ascension >= 3 { 20 } else { 18 };
            enemy.creature.add_block(block);
        }
        GUARDIAN_WHIRLWIND => {
            for _ in 0..4 {
                let dmg = enemy.attack_damage(player);
                crate::damage::deal_damage(dmg, &mut player.block, &mut player.hp);
                if player.hp <= 0 { break; }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod slime_ai_tests {
    use super::*;

    // Expected ranges transcribed from the Java getMove branches. These test
    // the rules for supplied rolls; they do not claim Java/Rust RNG parity.
    #[test]
    fn acid_first_move_ranges_cover_every_roll_and_ascension_boundary() {
        for (asc, large, spit_end, tackle_end) in [
            (0, false, 30, 70), (16, false, 30, 70),
            (17, false, 40, 80), (20, false, 40, 80),
            (0, true, 30, 70), (16, true, 30, 70),
            (17, true, 40, 70), (20, true, 40, 70),
        ] {
            for roll in 0..100 {
                let expected = if roll < spit_end { ACID_M_SPIT }
                    else if roll < tackle_end { ACID_M_TACKLE } else { ACID_M_LICK };
                let mut rng = Rng::new(42);
                let (mv, _) = acid_get_move(roll, &[], asc, large, &mut rng);
                assert_eq!(mv, expected, "A{asc} large={large} roll={roll}");
                assert_eq!(rng.counter, 0, "normal branches must not consume a fallback roll");
            }
        }
    }

    #[test]
    fn acid_repeat_limits_change_at_a17() {
        // One Tackle is forbidden below A17; two are allowed starting at A17.
        // One Lick may repeat below A17; it cannot repeat starting at A17.
        for large in [false, true] {
            let mut rng = Rng::new(42);
            assert_eq!(acid_get_move(40, &[ACID_M_TACKLE], 17, large, &mut rng).0, ACID_M_TACKLE);
            assert_eq!(acid_get_move(99, &[ACID_M_LICK], 16, large, &mut rng).0, ACID_M_LICK);
            assert_eq!(rng.counter, 0);
            acid_get_move(40, &[ACID_M_TACKLE], 16, large, &mut rng);
            assert_eq!(rng.counter, 1);
            acid_get_move(99, &[ACID_M_LICK], 17, large, &mut rng);
            assert_eq!(rng.counter, 2);
        }
    }

    #[derive(Clone, Copy)]
    enum Coin { Boolean, Chance(f32) }

    #[test]
    fn acid_fallbacks_use_the_java_coin_type_and_probability() {
        use Coin::*;
        // asc, large, roll, prior move, repeat count, coin, true move, false move
        let cases = [
            (16, false, 0, ACID_M_SPIT, 2, Boolean, ACID_M_TACKLE, ACID_M_LICK),
            (16, true, 29, ACID_M_SPIT, 2, Boolean, ACID_M_TACKLE, ACID_M_LICK),
            (17, false, 39, ACID_M_SPIT, 2, Boolean, ACID_M_TACKLE, ACID_M_LICK),
            (17, true, 39, ACID_M_SPIT, 2, Chance(0.6), ACID_M_TACKLE, ACID_M_LICK),
            (16, false, 30, ACID_M_TACKLE, 1, Chance(0.4), ACID_M_SPIT, ACID_M_LICK),
            (16, true, 69, ACID_M_TACKLE, 1, Chance(0.4), ACID_M_SPIT, ACID_M_LICK),
            (17, false, 79, ACID_M_TACKLE, 2, Chance(0.5), ACID_M_SPIT, ACID_M_LICK),
            (17, true, 69, ACID_M_TACKLE, 2, Chance(0.6), ACID_M_SPIT, ACID_M_LICK),
            (16, false, 70, ACID_M_LICK, 2, Chance(0.4), ACID_M_SPIT, ACID_M_TACKLE),
            (16, true, 99, ACID_M_LICK, 2, Chance(0.4), ACID_M_SPIT, ACID_M_TACKLE),
            (17, false, 80, ACID_M_LICK, 1, Chance(0.4), ACID_M_SPIT, ACID_M_TACKLE),
            (17, true, 70, ACID_M_LICK, 1, Chance(0.4), ACID_M_SPIT, ACID_M_TACKLE),
        ];
        for (asc, large, roll, previous, repeats, coin, yes, no) in cases {
            let mut seen = [false; 2];
            for seed in 0..128 {
                let mut rng = Rng::new(seed);
                let mut expected_rng = rng.clone();
                let outcome = match coin {
                    Boolean => expected_rng.random_bool(),
                    Chance(chance) => expected_rng.random_bool_chance(chance),
                };
                let history = vec![previous; repeats];
                assert_eq!(acid_get_move(roll, &history, asc, large, &mut rng).0,
                    if outcome { yes } else { no });
                seen[usize::from(outcome)] = true;
                assert_eq!(rng.counter, 1);
                assert_eq!(rng.random_int(99), expected_rng.random_int(99));
            }
            assert_eq!(seen, [true, true]);
        }
    }

    #[test]
    fn spike_move_ranges_and_repeat_restrictions() {
        for large in [false, true] {
            for asc in [0, 16, 17, 20] {
                for roll in 0..100 {
                    let expected = if roll < 30 { SPIKE_M_TACKLE } else { SPIKE_M_LICK };
                    assert_eq!(spike_get_move(roll, &[], asc, large).0, expected);
                    assert_eq!(spike_get_move(roll, &[SPIKE_M_TACKLE; 2], asc, large).0, SPIKE_M_LICK);
                    assert_eq!(spike_get_move(roll, &[SPIKE_M_LICK; 2], asc, large).0, SPIKE_M_TACKLE);
                    let after_lick = if asc >= 17 { SPIKE_M_TACKLE } else { expected };
                    assert_eq!(spike_get_move(roll, &[SPIKE_M_LICK], asc, large).0, after_lick);
                }
            }
        }
    }

    #[test]
    fn small_acid_initial_choice_and_direct_alternation() {
        for asc in [0, 16, 17, 20] {
            for seed in 0..16 {
                let mut ai = Rng::new(seed);
                let mut expected_rng = ai.clone();
                expected_rng.random_int(99); // AbstractMonster.rollMove
                let first = if asc < 17 && expected_rng.random_bool() { ACID_S_TACKLE } else { ACID_S_LICK };
                let mut hp = Rng::new(42);
                let mut e = EnemyState::new(EnemyId::AcidSlimeSmall, asc, &mut hp, &mut ai);
                assert_eq!(e.next_move, first);
                let counter = ai.counter;
                let mut player = CreatureState::new(1000, 1000);
                for turn in 0..12 {
                    let expected = if turn % 2 == 0 { first }
                        else if first == ACID_S_TACKLE { ACID_S_LICK } else { ACID_S_TACKLE };
                    assert_eq!(e.next_move, expected);
                    e.take_turn(&mut player, &mut ai);
                    assert_eq!(ai.counter, counter, "direct moves must not consume RNG");
                }
                assert_eq!(ai.random_int(99), expected_rng.random_int(99));
            }
        }
    }

    #[test]
    fn boss_direct_cycle_does_not_consume_ai_rolls() {
        let mut ai = Rng::new(42);
        let mut hp = Rng::new(42);
        let mut e = EnemyState::new(EnemyId::SlimeBoss, 20, &mut hp, &mut ai);
        let mut player = CreatureState::new(1000, 1000);
        let initial = ai.counter;
        for turn in 0..12 {
            assert_eq!(e.next_move, [SLIME_BOSS_GOOP, SLIME_BOSS_PREPARING, SLIME_BOSS_SLAM][turn % 3]);
            e.take_turn(&mut player, &mut ai);
            assert_eq!(ai.counter, initial);
        }
    }

    #[test]
    fn rolled_slime_moves_consume_primary_roll_and_only_required_fallback() {
        for id in [EnemyId::AcidSlimeMedium, EnemyId::AcidSlimeLarge,
            EnemyId::SpikeSlimeMedium, EnemyId::SpikeSlimeLarge, EnemyId::SpikeSlimeSmall] {
            for seed in 0..128 {
                let mut hp = Rng::new(42);
                let mut ai = Rng::new(seed);
                let mut e = EnemyState::new(id, 17, &mut hp, &mut ai);
                e.move_history = vec![1, 1];
                let mut expected_rng = ai.clone();
                let roll = expected_rng.random_int(99);
                if roll < 40 {
                    match id {
                        EnemyId::AcidSlimeMedium => { expected_rng.random_bool(); }
                        EnemyId::AcidSlimeLarge => { expected_rng.random_bool_chance(0.6); }
                        _ => {}
                    }
                }
                e.roll_move(&mut ai, false);
                assert_eq!(ai.counter, expected_rng.counter);
                assert_eq!(ai.random_int(99), expected_rng.random_int(99));
            }
        }
    }
}
