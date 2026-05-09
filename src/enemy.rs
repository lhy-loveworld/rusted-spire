use crate::creature::CreatureState;
use crate::damage::{apply_powers, DamageType};
use crate::power::PowerId;
use crate::rng::Rng;

/// All implemented enemies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EnemyId {
    JawWorm,
}

/// What the enemy intends to do this turn (visible to the player).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Intent {
    Attack(i32),          // will deal this much damage
    AttackDefend(i32),    // attack + gain block
    Buff,                 // self-buff (strength, etc.)
    Defend,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnemyState {
    pub id: EnemyId,
    pub creature: CreatureState,
    pub next_move: u8,
    pub move_history: Vec<u8>,
    pub intent: Intent,
}

impl EnemyState {
    pub fn new(id: EnemyId, ascension: u8, hp_rng: &mut Rng, ai_rng: &mut Rng) -> Self {
        let (hp_min, hp_max, _) = hp_range(id, ascension);
        let hp = hp_rng.random_range(hp_min, hp_max);
        let mut enemy = EnemyState {
            id,
            creature: CreatureState::new(hp),
            next_move: 0,
            move_history: vec![],
            intent: Intent::Unknown,
        };
        enemy.roll_move(ai_rng, true);
        enemy
    }

    pub fn is_dead(&self) -> bool {
        self.creature.is_dead()
    }

    /// Roll the next move and update `intent`.
    pub fn roll_move(&mut self, ai_rng: &mut Rng, first_move: bool) {
        let roll = ai_rng.random_int(99);
        let (mv, intent) = get_move(self.id, roll, &self.move_history, first_move, &self.creature);
        self.next_move = mv;
        self.intent = intent;
    }

    /// Execute the queued move, mutating player creature state directly.
    pub fn take_turn(
        &mut self,
        player_hp: &mut i32,
        player_block: &mut i32,
        player_powers: &[crate::power::PowerState],
        ai_rng: &mut Rng,
    ) {
        execute_move(self, player_hp, player_block, player_powers);
        self.move_history.push(self.next_move);
        if self.move_history.len() > 5 {
            self.move_history.remove(0);
        }
        if !self.is_dead() {
            self.roll_move(ai_rng, false);
        }
    }

}

// ---------------------------------------------------------------------------
// Per-enemy move tables
// ---------------------------------------------------------------------------

const JAW_WORM_CHOMP: u8  = 1;
const JAW_WORM_BELLOW: u8 = 2;
const JAW_WORM_THRASH: u8 = 3;

fn hp_range(id: EnemyId, ascension: u8) -> (i32, i32, i32) {
    // returns (hp_min, hp_max, _reserved)
    match id {
        EnemyId::JawWorm => {
            if ascension >= 7 { (42, 46, 0) } else { (40, 44, 0) }
        }
    }
}

fn get_move(
    id: EnemyId,
    roll: i32,
    history: &[u8],
    first_move: bool,
    creature: &CreatureState,
) -> (u8, Intent) {
    match id {
        EnemyId::JawWorm => jaw_worm_get_move(roll, history, first_move, creature),
    }
}

fn execute_move(
    enemy: &mut EnemyState,
    player_hp: &mut i32,
    player_block: &mut i32,
    player_powers: &[crate::power::PowerState],
) {
    match enemy.id {
        EnemyId::JawWorm => jaw_worm_take_turn(enemy, player_hp, player_block, player_powers),
    }
}

fn jaw_worm_get_move(
    roll: i32,
    history: &[u8],
    first_move: bool,
    _creature: &CreatureState,
) -> (u8, Intent) {
    // Source: JawWorm.getMove()
    let last     = |mv| history.last() == Some(&mv);
    let last_two = |mv| history.len() >= 2 && history[history.len()-1] == mv && history[history.len()-2] == mv;

    if first_move {
        return (JAW_WORM_CHOMP, Intent::Attack(11));
    }

    if roll < 25 {
        if last(JAW_WORM_CHOMP) {
            // 56.25% bellow, else thrash
            // simplified: always pick based on roll (deterministic without re-roll)
            return (JAW_WORM_BELLOW, Intent::Buff);
        }
        return (JAW_WORM_CHOMP, Intent::Attack(11));
    }

    if roll < 55 {
        if last_two(JAW_WORM_THRASH) {
            return (JAW_WORM_BELLOW, Intent::Buff);
        }
        return (JAW_WORM_THRASH, Intent::AttackDefend(7));
    }

    // roll >= 55
    if last(JAW_WORM_BELLOW) {
        return (JAW_WORM_CHOMP, Intent::Attack(11));
    }
    (JAW_WORM_BELLOW, Intent::Buff)
}

fn jaw_worm_take_turn(
    enemy: &mut EnemyState,
    player_hp: &mut i32,
    player_block: &mut i32,
    player_powers: &[crate::power::PowerState],
) {
    match enemy.next_move {
        JAW_WORM_CHOMP => {
            let dmg = apply_powers(11, DamageType::Normal, &enemy.creature.powers, player_powers);
            crate::damage::deal_damage(dmg, player_block, player_hp);
        }
        JAW_WORM_BELLOW => {
            enemy.creature.apply_power(PowerId::Strength, 3);
            enemy.creature.add_block(6);
        }
        JAW_WORM_THRASH => {
            let dmg = apply_powers(7, DamageType::Normal, &enemy.creature.powers, player_powers);
            crate::damage::deal_damage(dmg, player_block, player_hp);
            enemy.creature.add_block(5);
        }
        _ => {}
    }
}
