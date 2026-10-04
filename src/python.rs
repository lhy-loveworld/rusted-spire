use pyo3::prelude::*;

use crate::card::{Card, CardId};
use crate::combat::{step, CombatPhase, CombatResult, CombatState};
use crate::enemy::EnemyId;
use crate::obs::{self, action_mask, decode_action, encode_obs, ACTION_SIZE, MAX_ENEMIES, OBS_SIZE};

/// A single Ironclad combat environment.
///
/// Python usage:
///     env = SlayEnv()                                 # default: Cultist, Asc 7
///     env = SlayEnv(enemies=["Sentry","Sentry","Sentry"], ascension=7)
///     obs, mask = env.reset(seed=42)
///     obs, mask, reward, done = env.step(action)
#[pyclass]
pub struct SlayEnv {
    state: Option<CombatState>,
    enemy_ids: Vec<EnemyId>,
    ascension: u8,
    deck: Vec<Card>,
}

#[pymethods]
impl SlayEnv {
    #[new]
    #[pyo3(signature = (enemy=None, enemies=None, ascension=7, *, deck=None))]
    pub fn new(enemy: Option<&str>, enemies: Option<Vec<String>>, ascension: u8, deck: Option<Vec<String>>) -> PyResult<Self> {
        let enemy_ids = if let Some(list) = enemies {
            list.iter().map(|s| parse_enemy_id(s)).collect::<PyResult<_>>()?
        } else if let Some(e) = enemy {
            vec![parse_enemy_id(e)?]
        } else {
            vec![EnemyId::Cultist]
        };
        if enemy_ids.is_empty() || enemy_ids.len() > MAX_ENEMIES {
            return Err(pyo3::exceptions::PyValueError::new_err(
                format!("expected between 1 and {MAX_ENEMIES} enemies")
            ));
        }
        let deck = match deck {
            None => ironclad_starter(),
            Some(cards) => {
                if cards.is_empty() {
                    return Err(pyo3::exceptions::PyValueError::new_err("deck must contain at least one card"));
                }
                cards.iter().map(|s| Card::from_spec(s).map_err(pyo3::exceptions::PyValueError::new_err))
                    .collect::<PyResult<Vec<_>>>()?
            }
        };
        Ok(SlayEnv { state: None, enemy_ids, ascension, deck })
    }

    /// Returns a copy of the starting deck, preserving order and duplicates.
    #[getter]
    pub fn deck(&self) -> Vec<String> { self.deck.iter().map(Card::spec).collect() }

    /// Reset the environment with the given seed and optional starting HP.
    #[pyo3(signature = (seed, hp=None))]
    pub fn reset(&mut self, seed: u64, hp: Option<i32>) -> (Vec<f32>, Vec<bool>) {
        let starting_hp = hp.unwrap_or(CombatState::MAX_HP).clamp(1, CombatState::MAX_HP);
        let state = CombatState::new(self.deck.clone(), &self.enemy_ids, seed, self.ascension, starting_hp);
        let obs  = encode_obs(&state);
        let mask = action_mask(&state);
        self.state = Some(state);
        (obs, mask)
    }

    /// Step the environment with the given action index.
    pub fn step(&mut self, action_idx: usize) -> PyResult<(Vec<f32>, Vec<bool>, f32, bool)> {
        let state = self.state.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("call reset() before step()")
        })?;

        if matches!(state.phase, CombatPhase::Over(_)) {
            return Err(pyo3::exceptions::PyRuntimeError::new_err("combat is over; call reset() before step()"));
        }
        if !action_mask(state).get(action_idx).copied().unwrap_or(false) {
            return Err(pyo3::exceptions::PyValueError::new_err("illegal action: index is out of range or masked"));
        }
        let action = decode_action(action_idx, state).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("action cannot be decoded")
        })?;
        // Preserve the previous state if a spawning encounter exceeds the
        // fixed interface capacity; never silently hide an enemy from a policy.
        let mut next = state.clone();
        let result = step(&mut next, action);
        if !obs::fits_observation(&next) {
            return Err(pyo3::exceptions::PyValueError::new_err("action exceeds observation capacity; state unchanged"));
        }
        *state = next;

        let done = result.is_some();
        let reward = match result {
            Some(CombatResult::Victory) => {
                1.0 + (state.player.creature.hp as f32 / state.player.creature.max_hp as f32)
            }
            Some(CombatResult::Defeat) => -1.0,
            None => 0.0,
        };

        let obs  = encode_obs(state);
        let mask = if done { vec![false; ACTION_SIZE] } else { action_mask(state) };

        Ok((obs, mask, reward, done))
    }

    #[staticmethod]
    pub fn obs_size() -> usize { OBS_SIZE }

    #[staticmethod]
    pub fn action_size() -> usize { ACTION_SIZE }
}

fn parse_enemy_id(s: &str) -> PyResult<EnemyId> {
    match s {
        "JawWorm"         => Ok(EnemyId::JawWorm),
        "Cultist"         => Ok(EnemyId::Cultist),
        "LouseNormal"     => Ok(EnemyId::LouseNormal),
        "LouseDefensive"  => Ok(EnemyId::LouseDefensive),
        "FungiBeast"      => Ok(EnemyId::FungiBeast),
        "AcidSlimeSmall"  => Ok(EnemyId::AcidSlimeSmall),
        "AcidSlimeMedium" => Ok(EnemyId::AcidSlimeMedium),
        "SpikeSlimeSmall" => Ok(EnemyId::SpikeSlimeSmall),
        "SpikeSlimeMedium"=> Ok(EnemyId::SpikeSlimeMedium),
        "MadGremlin"      => Ok(EnemyId::MadGremlin),
        "SneakyGremlin"   => Ok(EnemyId::SneakyGremlin),
        "FatGremlin"      => Ok(EnemyId::FatGremlin),
        "ShieldGremlin"   => Ok(EnemyId::ShieldGremlin),
        "GremlinWizard"   => Ok(EnemyId::GremlinWizard),
        "GremlinNob"      => Ok(EnemyId::GremlinNob),
        "Lagavulin"       => Ok(EnemyId::Lagavulin),
        "Sentry"          => Ok(EnemyId::Sentry),
        "SlimeBoss"       => Ok(EnemyId::SlimeBoss),
        "AcidSlimeLarge"  => Ok(EnemyId::AcidSlimeLarge),
        "SpikeSlimeLarge" => Ok(EnemyId::SpikeSlimeLarge),
        "TheGuardian"     => Ok(EnemyId::TheGuardian),
        other => Err(pyo3::exceptions::PyValueError::new_err(
            format!("unknown enemy: {other}")
        )),
    }
}

fn ironclad_starter() -> Vec<Card> {
    let mut deck = vec![];
    for _ in 0..5 { deck.push(Card::new(CardId::Strike)); }
    for _ in 0..4 { deck.push(Card::new(CardId::Defend)); }
    deck.push(Card::new(CardId::Bash));
    deck
}

/// Register classes and constants into the Python module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SlayEnv>()?;
    m.add("OBS_SIZE",    OBS_SIZE)?;
    m.add("ACTION_SIZE", ACTION_SIZE)?;
    m.add("MAX_HP",      CombatState::MAX_HP)?;
    m.add("INTERFACE_VERSION", obs::INTERFACE_VERSION)?;
    m.add("MAX_HAND", obs::MAX_HAND)?;
    m.add("MAX_ENEMIES", obs::MAX_ENEMIES)?;
    m.add("TARGETS_PER_CARD", obs::TARGETS_PER_CARD)?;
    m.add("UNTARGETED_SLOT", obs::UNTARGETED_SLOT)?;
    m.add("END_TURN_ACTION", obs::END_TURN_ACTION)?;
    m.add("SELECT_CARD_ACTION", obs::SELECT_CARD_ACTION)?;
    m.add("PREVIOUS_PAGE_ACTION", obs::PREVIOUS_PAGE_ACTION)?;
    m.add("NEXT_PAGE_ACTION", obs::NEXT_PAGE_ACTION)?;
    m.add("SELECTION_OFFSET", obs::SELECTION_OFFSET)?;
    m.add("CHOICE_OFFSET", obs::CHOICE_OFFSET)?;
    m.add("CHOICE_FEATURES", obs::CHOICE_FEATURES)?;
    m.add("SELECTION_PAGE_SIZE", crate::combat::SELECTION_PAGE_SIZE)?;
    m.add("PLAYER_FEATURES", obs::PLAYER_FEATURES)?;
    m.add("HAND_FEATURES", obs::HAND_FEATURES)?;
    m.add("ENEMY_OFFSET", obs::ENEMY_OFFSET)?;
    m.add("ENEMY_FEATURES", obs::ENEMY_FEATURES)?;
    m.add("CARD_COUNT", crate::card::CARD_COUNT)?;
    m.add("CARD_NAMES", crate::card::ALL_CARDS.iter().map(|id| format!("{id:?}")).collect::<Vec<_>>())?;
    Ok(())
}
