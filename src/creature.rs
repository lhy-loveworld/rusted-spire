use crate::power::{
    PowerId, PowerState,
    MetalicizePower, DemonFormPower, StrengthDownPower,
};
use crate::damage::{apply_block_powers, deal_damage, DamageType};

/// Shared state for both players and enemies.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreatureState {
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub powers: Vec<PowerState>,
    /// Newly applied enemy-turn debuffs skip their first end-of-round decay.
    #[serde(default)]
    pub fresh_debuffs: Vec<PowerId>,
}

impl CreatureState {
    pub fn new(hp: i32, max_hp: i32) -> Self {
        CreatureState { hp, max_hp, block: 0, powers: vec![], fresh_debuffs: vec![] }
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
        if amount == 0 { return; }
        let debuff = (amount > 0 && matches!(id,
            PowerId::Weak | PowerId::Vulnerable | PowerId::Frail | PowerId::StrengthDown))
            || (matches!(id, PowerId::Strength | PowerId::Dexterity) && amount < 0);
        if debuff && self.power_amount(PowerId::Artifact) > 0 {
            self.apply_power(PowerId::Artifact, -1);
            return;
        }
        if let Some(existing) = self.powers.iter_mut().find(|p| p.id() == id) {
            existing.stack(amount);
            self.powers.retain(|p| p.amount() != 0);
            return;
        }
        let power = match id {
            PowerId::Strength   => PowerState::Strength(crate::power::StrengthPower { stacks: amount }),
            PowerId::Vulnerable => PowerState::Vulnerable(crate::power::VulnerablePower { stacks: amount }),
            PowerId::Weak       => PowerState::Weak(crate::power::WeakPower { stacks: amount }),
            PowerId::Frail      => PowerState::Frail(crate::power::FrailPower { stacks: amount }),
            PowerId::Ritual      => PowerState::Ritual(crate::power::RitualPower { stacks: amount, skip_first: true }),
            PowerId::CurlUp      => PowerState::CurlUp(crate::power::CurlUpPower { block: amount, triggered: false }),
            PowerId::Anger       => PowerState::Anger(crate::power::AngerPower { stacks: amount }),
            PowerId::Metalicize  => PowerState::Metalicize(MetalicizePower { stacks: amount }),
            PowerId::DemonForm   => PowerState::DemonForm(DemonFormPower { stacks: amount }),
            PowerId::StrengthDown=> PowerState::StrengthDown(StrengthDownPower { stacks: amount }),
            PowerId::Artifact => PowerState::Artifact(crate::power::ArtifactPower { stacks: amount }),
            PowerId::Dexterity => PowerState::Dexterity(crate::power::DexterityPower { stacks: amount.clamp(-999, 999) }),
            PowerId::ModeShift => PowerState::ModeShift(crate::power::ModeShiftPower { stacks: amount }),
            PowerId::SharpHide => PowerState::SharpHide(crate::power::SharpHidePower { stacks: amount }),
        };
        self.powers.push(power);
    }

    pub fn apply_power_from_enemy(&mut self, id: PowerId, amount: i32) {
        let fresh = amount > 0 && !self.has_power(id)
            && matches!(id, PowerId::Vulnerable | PowerId::Weak | PowerId::Frail);
        self.apply_power(id, amount);
        if fresh && self.has_power(id) {
            self.fresh_debuffs.push(id);
        }
    }

    pub fn has_power(&self, id: PowerId) -> bool {
        self.powers.iter().any(|p| p.id() == id)
    }

    pub fn power_amount(&self, id: PowerId) -> i32 {
        self.powers.iter().find(|p| p.id() == id).map(|p| p.amount()).unwrap_or(0)
    }

    /// Called at the start of this creature's turn (player or enemy).
    /// Applies effects before the normal draw.
    pub fn trigger_start_of_turn(&mut self) {
        let pending: Vec<(PowerId, i32)> = self.powers.iter_mut()
            .filter_map(|p| p.at_start_of_turn())
            .collect();
        for (id, amt) in pending {
            self.apply_power(id, amt);
        }
    }

    /// Player effects that resolve after the normal turn draw, such as Demon Form.
    pub fn trigger_start_of_turn_post_draw(&mut self) {
        let pending: Vec<_> = self.powers.iter_mut()
            .filter_map(|p| p.at_start_of_turn_post_draw())
            .collect();
        for (id, amount) in pending {
            self.apply_power(id, amount);
        }
    }

    /// Called after this creature takes Normal damage; triggers CurlUp if conditions met.
    pub fn trigger_on_attacked(&mut self, hp_lost: i32, dtype: DamageType) {
        if dtype != DamageType::Normal || hp_lost <= 0 || self.is_dead() {
            return;
        }
        let mut curl_block = 0i32;
        for p in &mut self.powers {
            if p.id() == PowerId::CurlUp && p.amount() > 0 {
                curl_block += p.amount();
                p.reduce(1); // sets triggered=true → amount()=0 → retain removes it
            }
        }
        self.powers.retain(|p| p.amount() != 0);
        if curl_block > 0 {
            self.add_block(curl_block);
        }
    }

    /// Called when the player plays a Skill card; triggers Anger → Strength gain.
    pub fn trigger_on_skill_played(&mut self) {
        let pending: Vec<(PowerId, i32)> = self.powers.iter_mut()
            .filter_map(|p| p.on_play_card(true))
            .collect();
        for (id, amt) in pending {
            self.apply_power(id, amt);
        }
    }

    /// Tick powers at end of turn; apply any pending grants (e.g. Ritual → Strength).
    pub fn tick_powers_end_of_turn(&mut self, is_player: bool) {
        // Metallicize is power-generated block, unaffected by Frail.
        let metal = self.power_amount(PowerId::Metalicize).max(0);
        self.block = (self.block + metal).min(999);
        let pending: Vec<(PowerId, i32)> = self.powers.iter_mut()
            .filter_map(|p| p.at_end_of_turn(is_player))
            .collect();
        self.powers.retain(|p| p.amount() != 0);
        for (id, amt) in pending {
            self.apply_power(id, amt);
        }
    }

    pub fn tick_powers_end_of_round(&mut self) {
        let fresh = std::mem::take(&mut self.fresh_debuffs);
        let pending: Vec<_> = self.powers.iter_mut()
            .filter(|p| !fresh.contains(&p.id()))
            .filter_map(|p| p.at_end_of_round())
            .collect();
        self.powers.retain(|p| p.amount() != 0);
        for (id, amt) in pending {
            self.apply_power(id, amt);
        }
    }
}
