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
    // Returns an optional (PowerId, amount) to apply to the owner after ticking.
    fn at_start_of_turn(&mut self) -> Option<(PowerId, i32)> { None }
    fn at_end_of_turn(&mut self, _is_player: bool) -> Option<(PowerId, i32)> { None }
    fn at_end_of_round(&mut self) -> Option<(PowerId, i32)> { None }

    // --- card hooks ---
    // Returns a pending (PowerId, amount) to apply to the owner when a Skill is played.
    fn on_play_card(&mut self, _is_skill: bool) -> Option<(PowerId, i32)> { None }
    fn on_card_draw(&mut self) {}
    fn on_exhaust(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PowerId {
    Strength,
    Vulnerable,
    Weak,
    Frail,
    Ritual,
    CurlUp,
    Anger,
    Metalicize,   // gain N block at end of turn
    DemonForm,    // gain N Strength at start of player turn
    StrengthDown, // lose N Strength at end of turn (used by Flex)
}

/// Enum-dispatch wrapper so powers can be stored in a Vec without boxing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PowerState {
    Strength(StrengthPower),
    Vulnerable(VulnerablePower),
    Weak(WeakPower),
    Frail(FrailPower),
    Ritual(RitualPower),
    CurlUp(CurlUpPower),
    Anger(AngerPower),
    Metalicize(MetalicizePower),
    DemonForm(DemonFormPower),
    StrengthDown(StrengthDownPower),
}

impl PowerState {
    pub fn id(&self) -> PowerId {
        match self {
            PowerState::Strength(_)    => PowerId::Strength,
            PowerState::Vulnerable(_)  => PowerId::Vulnerable,
            PowerState::Weak(_)        => PowerId::Weak,
            PowerState::Frail(_)       => PowerId::Frail,
            PowerState::Ritual(_)      => PowerId::Ritual,
            PowerState::CurlUp(_)      => PowerId::CurlUp,
            PowerState::Anger(_)       => PowerId::Anger,
            PowerState::Metalicize(_)  => PowerId::Metalicize,
            PowerState::DemonForm(_)   => PowerId::DemonForm,
            PowerState::StrengthDown(_)=> PowerId::StrengthDown,
        }
    }

    pub fn amount(&self) -> i32 {
        match self {
            PowerState::Strength(p)    => p.amount(),
            PowerState::Vulnerable(p)  => p.amount(),
            PowerState::Weak(p)        => p.amount(),
            PowerState::Frail(p)       => p.amount(),
            PowerState::Ritual(p)      => p.amount(),
            PowerState::CurlUp(p)      => p.amount(),
            PowerState::Anger(p)       => p.amount(),
            PowerState::Metalicize(p)  => p.amount(),
            PowerState::DemonForm(p)   => p.amount(),
            PowerState::StrengthDown(p)=> p.amount(),
        }
    }

    pub fn stack(&mut self, amount: i32) {
        match self {
            PowerState::Strength(p)    => p.stack(amount),
            PowerState::Vulnerable(p)  => p.stack(amount),
            PowerState::Weak(p)        => p.stack(amount),
            PowerState::Frail(p)       => p.stack(amount),
            PowerState::Ritual(p)      => p.stack(amount),
            PowerState::CurlUp(p)      => p.stack(amount),
            PowerState::Anger(p)       => p.stack(amount),
            PowerState::Metalicize(p)  => p.stack(amount),
            PowerState::DemonForm(p)   => p.stack(amount),
            PowerState::StrengthDown(p)=> p.stack(amount),
        }
    }

    pub fn reduce(&mut self, amount: i32) {
        match self {
            PowerState::Strength(p)    => p.reduce(amount),
            PowerState::Vulnerable(p)  => p.reduce(amount),
            PowerState::Weak(p)        => p.reduce(amount),
            PowerState::Frail(p)       => p.reduce(amount),
            PowerState::Ritual(p)      => p.reduce(amount),
            PowerState::CurlUp(p)      => p.reduce(amount),
            PowerState::Anger(p)       => p.reduce(amount),
            PowerState::Metalicize(p)  => p.reduce(amount),
            PowerState::DemonForm(p)   => p.reduce(amount),
            PowerState::StrengthDown(p)=> p.reduce(amount),
        }
    }

    pub fn at_damage_give(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)    => p.at_damage_give(damage, dtype),
            PowerState::Vulnerable(p)  => p.at_damage_give(damage, dtype),
            PowerState::Weak(p)        => p.at_damage_give(damage, dtype),
            PowerState::Frail(p)       => p.at_damage_give(damage, dtype),
            PowerState::Ritual(p)      => p.at_damage_give(damage, dtype),
            PowerState::CurlUp(p)      => p.at_damage_give(damage, dtype),
            PowerState::Anger(p)       => p.at_damage_give(damage, dtype),
            PowerState::Metalicize(p)  => p.at_damage_give(damage, dtype),
            PowerState::DemonForm(p)   => p.at_damage_give(damage, dtype),
            PowerState::StrengthDown(p)=> p.at_damage_give(damage, dtype),
        }
    }

    pub fn at_damage_receive(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)    => p.at_damage_receive(damage, dtype),
            PowerState::Vulnerable(p)  => p.at_damage_receive(damage, dtype),
            PowerState::Weak(p)        => p.at_damage_receive(damage, dtype),
            PowerState::Frail(p)       => p.at_damage_receive(damage, dtype),
            PowerState::Ritual(p)      => p.at_damage_receive(damage, dtype),
            PowerState::CurlUp(p)      => p.at_damage_receive(damage, dtype),
            PowerState::Anger(p)       => p.at_damage_receive(damage, dtype),
            PowerState::Metalicize(p)  => p.at_damage_receive(damage, dtype),
            PowerState::DemonForm(p)   => p.at_damage_receive(damage, dtype),
            PowerState::StrengthDown(p)=> p.at_damage_receive(damage, dtype),
        }
    }

    pub fn at_damage_final_give(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)    => p.at_damage_final_give(damage, dtype),
            PowerState::Vulnerable(p)  => p.at_damage_final_give(damage, dtype),
            PowerState::Weak(p)        => p.at_damage_final_give(damage, dtype),
            PowerState::Frail(p)       => p.at_damage_final_give(damage, dtype),
            PowerState::Ritual(p)      => p.at_damage_final_give(damage, dtype),
            PowerState::CurlUp(p)      => p.at_damage_final_give(damage, dtype),
            PowerState::Anger(p)       => p.at_damage_final_give(damage, dtype),
            PowerState::Metalicize(p)  => p.at_damage_final_give(damage, dtype),
            PowerState::DemonForm(p)   => p.at_damage_final_give(damage, dtype),
            PowerState::StrengthDown(p)=> p.at_damage_final_give(damage, dtype),
        }
    }

    pub fn at_damage_final_receive(&self, damage: f32, dtype: DamageType) -> f32 {
        match self {
            PowerState::Strength(p)    => p.at_damage_final_receive(damage, dtype),
            PowerState::Vulnerable(p)  => p.at_damage_final_receive(damage, dtype),
            PowerState::Weak(p)        => p.at_damage_final_receive(damage, dtype),
            PowerState::Frail(p)       => p.at_damage_final_receive(damage, dtype),
            PowerState::Ritual(p)      => p.at_damage_final_receive(damage, dtype),
            PowerState::CurlUp(p)      => p.at_damage_final_receive(damage, dtype),
            PowerState::Anger(p)       => p.at_damage_final_receive(damage, dtype),
            PowerState::Metalicize(p)  => p.at_damage_final_receive(damage, dtype),
            PowerState::DemonForm(p)   => p.at_damage_final_receive(damage, dtype),
            PowerState::StrengthDown(p)=> p.at_damage_final_receive(damage, dtype),
        }
    }

    pub fn modify_block(&self, block: f32) -> f32 {
        match self {
            PowerState::Strength(p)    => p.modify_block(block),
            PowerState::Vulnerable(p)  => p.modify_block(block),
            PowerState::Weak(p)        => p.modify_block(block),
            PowerState::Frail(p)       => p.modify_block(block),
            PowerState::Ritual(p)      => p.modify_block(block),
            PowerState::CurlUp(p)      => p.modify_block(block),
            PowerState::Anger(p)       => p.modify_block(block),
            PowerState::Metalicize(p)  => p.modify_block(block),
            PowerState::DemonForm(p)   => p.modify_block(block),
            PowerState::StrengthDown(p)=> p.modify_block(block),
        }
    }

    pub fn at_start_of_turn(&mut self) -> Option<(PowerId, i32)> {
        match self {
            PowerState::DemonForm(p)   => p.at_start_of_turn(),
            _                          => None,
        }
    }

    pub fn at_end_of_turn(&mut self, is_player: bool) -> Option<(PowerId, i32)> {
        match self {
            PowerState::Strength(p)    => p.at_end_of_turn(is_player),
            PowerState::Vulnerable(p)  => p.at_end_of_turn(is_player),
            PowerState::Weak(p)        => p.at_end_of_turn(is_player),
            PowerState::Frail(p)       => p.at_end_of_turn(is_player),
            PowerState::Ritual(p)      => p.at_end_of_turn(is_player),
            PowerState::CurlUp(p)      => p.at_end_of_turn(is_player),
            PowerState::Anger(p)       => p.at_end_of_turn(is_player),
            PowerState::Metalicize(p)  => p.at_end_of_turn(is_player),
            PowerState::DemonForm(p)   => p.at_end_of_turn(is_player),
            PowerState::StrengthDown(p)=> p.at_end_of_turn(is_player),
        }
    }

    pub fn on_play_card(&mut self, is_skill: bool) -> Option<(PowerId, i32)> {
        match self {
            PowerState::Anger(p) => p.on_play_card(is_skill),
            _                    => None,
        }
    }

    pub fn at_end_of_round(&mut self) -> Option<(PowerId, i32)> {
        match self {
            PowerState::Vulnerable(p) => p.at_end_of_round(),
            PowerState::Weak(p) => p.at_end_of_round(),
            PowerState::Frail(p) => p.at_end_of_round(),
            PowerState::Ritual(p) => p.at_end_of_round(),
            _ => None,
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

    fn at_end_of_round(&mut self) -> Option<(PowerId, i32)> {
        self.stacks -= 1;
        None
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

    fn at_end_of_round(&mut self) -> Option<(PowerId, i32)> {
        self.stacks -= 1;
        None
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

    fn at_end_of_round(&mut self) -> Option<(PowerId, i32)> {
        self.stacks -= 1;
        None
    }
}

// --- CurlUp: gain block once on first Normal hit that doesn't kill ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CurlUpPower { pub block: i32, pub triggered: bool }

impl Power for CurlUpPower {
    fn power_id(&self) -> PowerId { PowerId::CurlUp }
    // amount() returns 0 once triggered so `retain` removes it automatically
    fn amount(&self) -> i32 { if self.triggered { 0 } else { self.block } }
    fn stack(&mut self, n: i32) { self.block += n; }
    fn reduce(&mut self, _n: i32) { self.triggered = true; }
}

// --- Anger: gain Strength each time the player plays a Skill ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AngerPower { pub stacks: i32 }

impl Power for AngerPower {
    fn power_id(&self) -> PowerId { PowerId::Anger }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn on_play_card(&mut self, is_skill: bool) -> Option<(PowerId, i32)> {
        if is_skill { Some((PowerId::Strength, self.stacks)) } else { None }
    }
}

// --- Metalicize: gain N block at end of turn (handled by CreatureState) ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetalicizePower { pub stacks: i32 }

impl Power for MetalicizePower {
    fn power_id(&self) -> PowerId { PowerId::Metalicize }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }
}

// --- DemonForm: gain N Strength at start of each player turn ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DemonFormPower { pub stacks: i32 }

impl Power for DemonFormPower {
    fn power_id(&self) -> PowerId { PowerId::DemonForm }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn at_start_of_turn(&mut self) -> Option<(PowerId, i32)> {
        Some((PowerId::Strength, self.stacks))
    }
}

// --- StrengthDown: lose N Strength at end of turn (Flex's temporary buff) ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StrengthDownPower { pub stacks: i32 }

impl Power for StrengthDownPower {
    fn power_id(&self) -> PowerId { PowerId::StrengthDown }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn at_end_of_turn(&mut self, _is_player: bool) -> Option<(PowerId, i32)> {
        let n = self.stacks;
        self.stacks = 0; // zeroed → retain removes this power
        Some((PowerId::Strength, -n))
    }
}

// --- Ritual: gain Strength each round (skips first tick) ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RitualPower { pub stacks: i32, pub skip_first: bool }

impl Power for RitualPower {
    fn power_id(&self) -> PowerId { PowerId::Ritual }
    fn amount(&self) -> i32 { self.stacks }
    fn stack(&mut self, n: i32) { self.stacks += n; }
    fn reduce(&mut self, n: i32) { self.stacks -= n; }

    fn at_end_of_round(&mut self) -> Option<(PowerId, i32)> {
        if self.skip_first {
            self.skip_first = false;
            None
        } else {
            Some((PowerId::Strength, self.stacks))
        }
    }
}
