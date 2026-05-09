use crate::power::{PowerId, PowerState};
use crate::damage::{apply_block_powers, deal_damage, DamageType};

/// Shared state for both players and enemies.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreatureState {
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub powers: Vec<PowerState>,
}

impl CreatureState {
    pub fn new(hp: i32) -> Self {
        CreatureState { hp, max_hp: hp, block: 0, powers: vec![] }
    }

    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }

    /// Adds block after running it through power hooks (Frail etc).
    pub fn add_block(&mut self, amount: i32) {
        let effective = apply_block_powers(amount, &self.powers);
        self.block = (self.block + effective).min(999);
    }

    /// Resets block to zero (start of player turn, or start of enemy turn
    /// for enemies that don't have Barricade).
    pub fn lose_block(&mut self) {
        self.block = 0;
    }

    /// Applies final damage output — reduces block first, then HP.
    /// Returns HP actually lost.
    pub fn receive_damage(&mut self, damage: i32, dtype: DamageType) -> i32 {
        if dtype == DamageType::HpLoss {
            self.hp -= damage;
            return damage;
        }
        deal_damage(damage, &mut self.block, &mut self.hp)
    }

    // --- power helpers ---

    pub fn apply_power(&mut self, id: PowerId, amount: i32) {
        if let Some(existing) = self.powers.iter_mut().find(|p| p.id() == id) {
            existing.stack(amount);
            return;
        }
        let power = match id {
            PowerId::Strength   => PowerState::Strength(crate::power::StrengthPower { stacks: amount }),
            PowerId::Vulnerable => PowerState::Vulnerable(crate::power::VulnerablePower { stacks: amount }),
            PowerId::Weak       => PowerState::Weak(crate::power::WeakPower { stacks: amount }),
            PowerId::Frail      => PowerState::Frail(crate::power::FrailPower { stacks: amount }),
        };
        self.powers.push(power);
    }

    pub fn has_power(&self, id: PowerId) -> bool {
        self.powers.iter().any(|p| p.id() == id)
    }

    pub fn power_amount(&self, id: PowerId) -> i32 {
        self.powers.iter().find(|p| p.id() == id).map(|p| p.amount()).unwrap_or(0)
    }

    /// Tick down turn-based debuffs at end of turn.
    pub fn tick_powers_end_of_turn(&mut self, is_player: bool) {
        for p in &mut self.powers {
            p.at_end_of_turn(is_player);
        }
        self.powers.retain(|p| p.amount() != 0);
    }
}
