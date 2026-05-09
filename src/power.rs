use crate::damage::DamageType;

/// Hook interface matching AbstractPower — all methods have default no-ops.
pub trait Power: std::fmt::Debug {
    fn power_id(&self) -> PowerId;
    fn amount(&self) -> i32;
    fn stack(&mut self, amount: i32);
    fn reduce(&mut self, amount: i32);

    // --- damage hooks (called by DamageInfo.applyPowers) ---
    fn at_damage_give(&self, damage: f32, _dtype: DamageType) -> f32 { damage }
    fn at_damage_final_give(&self, damage: f32, _dtype: DamageType) -> f32 { damage }
    fn at_damage_receive(&self, damage: f32, _dtype: DamageType) -> f32 { damage }
    fn at_damage_final_receive(&self, damage: f32, _dtype: DamageType) -> f32 { damage }

    // --- block hooks ---
    fn modify_block(&self, block: f32) -> f32 { block }

    // --- turn hooks ---
    fn at_start_of_turn(&mut self) {}
    fn at_end_of_turn(&mut self, _is_player: bool) {}
    fn at_end_of_round(&mut self) {}

    // --- card hooks ---
    fn on_play_card(&mut self) {}
    fn on_card_draw(&mut self) {}
    fn on_exhaust(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PowerId {
    Strength,
    Vulnerable,
    Weak,
    Frail,
}

/// Enum-dispatch wrapper so powers can be stored in a Vec without boxing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PowerState {
    Strength(StrengthPower),
    Vulnerable(VulnerablePower),
    Weak(WeakPower),
    Frail(FrailPower),
}

impl PowerState {
    pub fn id(&self) -> PowerId {
        match self {
            PowerState::Strength(_)   => PowerId::Strength,
            PowerState::Vulnerable(_) => PowerId::Vulnerable,
            PowerState::Weak(_)       => PowerId::Weak,
            PowerState::Frail(_)      => PowerId::Frail,
        }
    }

    pub fn amount(&self) -> i32 {
        match self {
            PowerState::Strength(p)   => p.amount(),
            PowerState::Vulnerable(p) => p.amount(),
            PowerState::Weak(p)       => p.amount(),
            PowerState::Frail(p)      => p.amount(),
        }
    }

    pub fn stack(&mut self, amount: i32) {
        match self {
            PowerState::Strength(p)   => p.stack(amount),
            PowerState::Vulnerable(p) => p.stack(amount),
            PowerState::Weak(p)       => p.stack(amount),
            PowerState::Frail(p)      => p.stack(amount),
        }
    }

    pub fn reduce(&mut self, amount: i32) {
        match self {
            PowerState::Strength(p)   => p.reduce(amount),
            PowerState::Vulnerable(p) => p.reduce(amount),
            PowerState::Weak(p)       => p.reduce(amount),
            PowerState::Frail(p)      => p.reduce(amount),
        }
    }

    pub fn at_damage_give(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)   => p.at_damage_give(damage, dtype),
            PowerState::Vulnerable(p) => p.at_damage_give(damage, dtype),
            PowerState::Weak(p)       => p.at_damage_give(damage, dtype),
            PowerState::Frail(p)      => p.at_damage_give(damage, dtype),
        }
    }

    pub fn at_damage_receive(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)   => p.at_damage_receive(damage, dtype),
            PowerState::Vulnerable(p) => p.at_damage_receive(damage, dtype),
            PowerState::Weak(p)       => p.at_damage_receive(damage, dtype),
            PowerState::Frail(p)      => p.at_damage_receive(damage, dtype),
        }
    }

    pub fn at_damage_final_give(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)   => p.at_damage_final_give(damage, dtype),
            PowerState::Vulnerable(p) => p.at_damage_final_give(damage, dtype),
            PowerState::Weak(p)       => p.at_damage_final_give(damage, dtype),
            PowerState::Frail(p)      => p.at_damage_final_give(damage, dtype),
        }
    }

    pub fn at_damage_final_receive(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)   => p.at_damage_final_receive(damage, dtype),
            PowerState::Vulnerable(p) => p.at_damage_final_receive(damage, dtype),
            PowerState::Weak(p)       => p.at_damage_final_receive(damage, dtype),
            PowerState::Frail(p)      => p.at_damage_final_receive(damage, dtype),
        }
    }

    pub fn modify_block(&self, block: f32) -> f32 {
        match self {
            PowerState::Strength(p)   => p.modify_block(block),
            PowerState::Vulnerable(p) => p.modify_block(block),
            PowerState::Weak(p)       => p.modify_block(block),
            PowerState::Frail(p)      => p.modify_block(block),
        }
    }

    pub fn at_end_of_turn(&mut self, is_player: bool) {
        match self {
            PowerState::Strength(p)   => p.at_end_of_turn(is_player),
            PowerState::Vulnerable(p) => p.at_end_of_turn(is_player),
            PowerState::Weak(p)       => p.at_end_of_turn(is_player),
            PowerState::Frail(p)      => p.at_end_of_turn(is_player),
        }
    }
}

// --- Strength ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StrengthPower { pub stacks: i32 }

impl Power for StrengthPower {
    fn power_id(&self) -> PowerId { PowerId::Strength }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn at_damage_give(&self, damage: f32, dtype: DamageType) -> f32 {
        if dtype == DamageType::Normal {
            damage + self.stacks as f32
        } else {
            damage
        }
    }
}

// --- Vulnerable: target takes 50% more damage ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VulnerablePower { pub stacks: i32 }

impl Power for VulnerablePower {
    fn power_id(&self) -> PowerId { PowerId::Vulnerable }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn at_damage_receive(&self, damage: f32, dtype: DamageType) -> f32 {
        if dtype == DamageType::Normal {
            damage * 1.5
        } else {
            damage
        }
    }

    fn at_end_of_turn(&mut self, _is_player: bool) {
        self.stacks -= 1;
    }
}

// --- Weak: owner deals 25% less damage ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeakPower { pub stacks: i32 }

impl Power for WeakPower {
    fn power_id(&self) -> PowerId { PowerId::Weak }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn at_damage_give(&self, damage: f32, dtype: DamageType) -> f32 {
        if dtype == DamageType::Normal {
            damage * 0.75
        } else {
            damage
        }
    }

    fn at_end_of_turn(&mut self, _is_player: bool) {
        self.stacks -= 1;
    }
}

// --- Frail: owner gains 25% less block ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrailPower { pub stacks: i32 }

impl Power for FrailPower {
    fn power_id(&self) -> PowerId { PowerId::Frail }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn modify_block(&self, block: f32) -> f32 {
        block * 0.75
    }

    fn at_end_of_turn(&mut self, _is_player: bool) {
        self.stacks -= 1;
    }
}
