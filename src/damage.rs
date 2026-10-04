use crate::power::{PowerId, PowerState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DamageType {
    Normal,
    Thorns,
    HpLoss,
}

/// Mirrors DamageInfo.applyPowers() — walks the power hook chain and returns
/// the final integer damage output.
pub fn apply_powers(
    base: i32,
    dtype: DamageType,
    owner_powers: &[PowerState],
    target_powers: &[PowerState],
) -> i32 {
    let mut tmp = base as f32;

    // Strength precedes multiplicative outgoing modifiers regardless of the
    // order in which powers were applied to the creature.
    for p in owner_powers.iter().filter(|p| p.id() == PowerId::Strength) {
        tmp = p.at_damage_give(tmp, dtype);
    }
    for p in owner_powers.iter().filter(|p| p.id() != PowerId::Strength) {
        tmp = p.at_damage_give(tmp, dtype);
    }
    for p in target_powers {
        tmp = p.at_damage_receive(tmp, dtype);
    }
    for p in owner_powers {
        tmp = p.at_damage_final_give(tmp, dtype);
    }
    for p in target_powers {
        tmp = p.at_damage_final_receive(tmp, dtype);
    }

    (tmp.floor() as i32).max(0)
}

/// Applies block reduction first, then HP damage. Returns actual HP lost.
pub fn deal_damage(damage: i32, block: &mut i32, hp: &mut i32) -> i32 {
    let absorbed = damage.min(*block);
    *block -= absorbed;
    let hp_damage = damage - absorbed;
    *hp -= hp_damage;
    hp_damage
}

/// Mirrors the block calculation chain (Frail etc).
pub fn apply_block_powers(base: i32, owner_powers: &[PowerState]) -> i32 {
    let mut tmp = base as f32;
    for p in owner_powers.iter().filter(|p| p.id() == PowerId::Dexterity) {
        tmp = p.modify_block(tmp);
    }
    for p in owner_powers.iter().filter(|p| p.id() != PowerId::Dexterity) {
        tmp = p.modify_block(tmp);
    }
    (tmp.floor() as i32).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::{PowerState, StrengthPower, VulnerablePower, WeakPower};

    #[test]
    fn strength_adds_to_damage() {
        let owner = vec![PowerState::Strength(StrengthPower { stacks: 3 })];
        let target = vec![];
        assert_eq!(apply_powers(6, DamageType::Normal, &owner, &target), 9);
    }

    #[test]
    fn vulnerable_multiplies_received() {
        let owner = vec![];
        let target = vec![PowerState::Vulnerable(VulnerablePower { stacks: 1 })];
        // 6 * 1.5 = 9.0 → floor → 9
        assert_eq!(apply_powers(6, DamageType::Normal, &owner, &target), 9);
    }

    #[test]
    fn weak_reduces_outgoing() {
        let owner = vec![PowerState::Weak(WeakPower { stacks: 1 })];
        let target = vec![];
        // 6 * 0.75 = 4.5 → floor → 4
        assert_eq!(apply_powers(6, DamageType::Normal, &owner, &target), 4);
    }

    #[test]
    fn strength_plus_vulnerable() {
        let owner = vec![PowerState::Strength(StrengthPower { stacks: 2 })];
        let target = vec![PowerState::Vulnerable(VulnerablePower { stacks: 1 })];
        // (6+2) * 1.5 = 12.0 → 12
        assert_eq!(apply_powers(6, DamageType::Normal, &owner, &target), 12);
    }

    #[test]
    fn damage_blocks_first() {
        let mut block = 4;
        let mut hp = 80;
        let hp_lost = deal_damage(10, &mut block, &mut hp);
        assert_eq!(block, 0);
        assert_eq!(hp, 74);
        assert_eq!(hp_lost, 6);
    }

    #[test]
    fn damage_fully_blocked() {
        let mut block = 20;
        let mut hp = 80;
        let hp_lost = deal_damage(10, &mut block, &mut hp);
        assert_eq!(block, 10);
        assert_eq!(hp, 80);
        assert_eq!(hp_lost, 0);
    }
}
