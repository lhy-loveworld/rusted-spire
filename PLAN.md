# Rusted Spire — Implementation Plan

Headless Slay the Spire simulator in Rust, exposed to Python via PyO3, for use in Reinforcement Learning.

## Resumption status (2026-10-03)

The original checklist below predates the expanded implementation. Phases 7 and
8 now have code (`src/obs.rs`, `src/python.rs`), and phase 9 has `train.py` plus
`experiments.ipynb`. Phase 10 has partial card, power, enemy, and ascension support.
These implementations still require behavioral validation; unchecked expansion
items should not be interpreted as a complete inventory of missing code.

Completed in the resumption: WSL/Python setup, regression tests, enemy block
lifetime, defeat handling, illegal-action rejection, attack intent refresh,
turn/round power timing, explicit multi-enemy targeting and observations (v3),
Gymnasium seeding/truncation, Sentry Artifact/Dazed, and half-HP slime splits.
The local Java audit also guided slime AI probabilities/repetition, direct
move changes, Demon Form's post-draw timing, and round-end hook ordering.
RNG primitives and deck shuffles now match executed Java fixtures; combat
streams start independently from the same effective combat seed.
Training and seeded evaluation scripts are available, with CI coverage.
Constructor ordering now separates creation, initial moves and pre-battle rolls.
Lagavulin and Guardian have source-derived mechanics tests; interface v4 exposes
their new powers (196 observations / 61 actions). Next priorities are controlled
combat traces, remaining enemy AI/card choice audits, and multi-seed held-out
training comparisons. Full Java combat parity has not been established. See
README.md and docs/VALIDATION.md for setup, evidence, and interface limits.

---

## Status Legend
- [ ] Not started
- [~] In progress
- [x] Done

---

## Phase 1 — Rust Crate Scaffold
- [x] `cargo init --lib` in project root
- [x] Add dependencies to `Cargo.toml`: `pyo3` (optional), `serde`
- [x] Define top-level module layout (`mod` declarations in `lib.rs`)

---

## Phase 2 — RNG
Source: `com/megacrit/cardcrawl/random/Random.java`

- [x] Implement `Rng` struct wrapping xorshift128 (`s0: u64`, `s1: u64`, `counter: u32`)
- [x] Methods: `random_int(range)`, `random_range(lo, hi)`, `random_bool()`, `random_bool_chance(f32)`, `random_float()`, `copy()`
- [x] Implement `RngBundle` holding all named streams: `ai`, `shuffle`, `card`, `monster_hp`, `relic`, `potion`, `misc`
- [x] Seed `RngBundle` from a single `u64` master seed
- [x] Unit tests: determinism, copy independence, range bounds, float unit interval
- [x] Cross-check: 1,283 executed Java cases across seven seeds, boundary values, bounded retries, and deck shuffles
- [ ] Validate full combat RNG consumption against original-game traces

---

## Phase 3 — Core Data Structures
Source: `AbstractCreature`, `AbstractPlayer`, `AbstractMonster`, `AbstractCard`, `CardGroup`, `DamageInfo`

### 3a — Cards
- [x] `CardId` enum: `Strike`, `Defend`, `Bash`
- [x] `Card` struct: `id: CardId`, `cost: i32`, `upgraded: bool`
- [x] `requires_target(id) -> bool`

### 3b — Powers
- [x] `Power` trait with default no-op hooks (damage give/receive/final, modify_block, turn hooks, card hooks)
- [x] Concrete powers: `Strength`, `Vulnerable`, `Weak`, `Frail`
- [x] `PowerState` enum dispatch (no boxing)

### 3c — Creatures
- [x] `CreatureState`: `hp`, `max_hp`, `block`, `powers`
- [x] `add_block()` — power hooks, cap 999
- [x] `lose_block()`, `receive_damage()`, `is_dead()`
- [x] `apply_power()`, `has_power()`, `power_amount()`, `tick_powers_end_of_turn()`

### 3d — Player
- [x] `PlayerState`: `CreatureState` + energy + hand/draw/discard/exhaust piles
- [x] `draw(n)` — shuffles discard into draw when empty
- [x] `start_turn()` — reset block, restore energy, draw 5; `end_turn()` exhausts Ethereal and discards the rest
- [x] `discard_from_hand()`, `exhaust_from_hand()`

### 3e — Enemy
- [x] `EnemyId` enum: `JawWorm`
- [x] `EnemyState`: `CreatureState` + `next_move`, `move_history`, `intent`
- [x] `Intent` enum: `Attack(i32)`, `AttackDefend(i32)`, `Buff`, `Defend`, `Unknown`
- [x] `roll_move()` / `take_turn()` for JawWorm
- [x] HP randomization at construction using `monster_hp` rng stream

---

## Phase 4 — Damage Pipeline
Source: `DamageInfo.applyPowers()`

- [x] `DamageType` enum: `Normal`, `Thorns`, `HpLoss`
- [x] `apply_powers(base, dtype, owner_powers, target_powers) -> i32`
- [x] `deal_damage(damage, block, hp) -> i32`
- [x] `apply_block_powers(base, owner_powers) -> i32`
- [x] Unit tests: Strength, Vulnerable, Weak, combinations, block absorption

---

## Phase 5 — Card Effects
Source: `cards/red/Strike_Red.java`, `Defend_Red.java`, `Bash.java`

- [x] `play_card()` in `combat.rs`: deduct energy, dispatch on `CardId`, move to discard
- [x] Strike: 6 (9★) damage
- [x] Defend: 5 (8★) block
- [x] Bash: 8 (10★) damage + 2 (3★) Vulnerable

---

## Phase 6 — Combat Loop
Source: `GameActionManager`, `AbstractDungeon`, `AbstractPlayer`, `AbstractMonster`

- [x] `CombatState`: owns `PlayerState`, `Vec<EnemyState>`, `RngBundle`, `turn`, `phase`
- [x] `CombatPhase` / `CombatResult` enums
- [x] `CombatState::new(deck, enemies, seed)`
- [x] `available_actions(state) -> Vec<Action>`
- [x] `step(state, action) -> Option<CombatResult>`
- [x] End-of-turn: power ticks, enemy turns, block resets, new player turn
- [x] Victory / defeat detection
- [x] Tests: init, actions, turn counter, combat-can-be-won

---

## Phase 7 — Observation & Action Encoding (RL Interface)
- [x] Flat `Vec<f32>` encoding, 196 values in interface v4 (see docs/RL_INTERFACE.md)
- [x] Player, hand, five living enemy slots, per-creature powers and timing flags
- [x] `Vec<bool>` mask aligned to 61 actions, with explicit target selection
- [x] Exported `ACTION_SIZE`, `OBS_SIZE`, and `INTERFACE_VERSION`
- [x] Fixed hand/enemy capacities with explicit Python overflow rejection

---

## Phase 8 — Python Bindings (PyO3)
- [x] Add `python` feature to `Cargo.toml`, configure `cdylib` crate type
- [x] `#[pyclass] SlayEnv`: wraps `CombatState`
- [x] `#[pymethods]`:
  - `reset(seed: u64) -> (obs, mask)`
  - `step(action: usize) -> (obs, mask, reward, done)`
  - `action_size() -> usize`
  - `obs_size() -> usize`
- [x] Reward: `1 + hp_remaining/max_hp` on combat victory; `-1` on defeat; zero otherwise
- [x] Build with `maturin develop` into the `.venv`
- [x] Python smoke test: 100 seeded random-action episodes plus API regression tests

---

## Phase 9 — First Training Run
- [x] Write `train.py`: shared Gymnasium wrapper, seeded subprocess training
- [x] Enable action masking (`MaskablePPO` from `sb3-contrib`)
- [ ] Baseline: random policy win rate on JawWorm
- [ ] Train 1M steps, log win rate and avg HP remaining
- [ ] Save checkpoint, plot learning curve

---

## Phase 10 — Incremental Expansion (post-MVP)

### 10a — More Ironclad Cards (Act 1 pool)
- [ ] Anger, Armaments, Body Slam, Clash, Cleave, Clothesline, Flex, Havoc, Headbutt, Heavy Blade, Iron Wave, Perfected Strike, Pommel Strike, Shrug It Off, Sword Boomerang, Thunderclap, True Grit, Twin Strike, Warcry, Wild Strike
- [ ] Uncommons: Battling, Battle Trance, Blood for Blood, Bloodletting, Burning Pact, Carnage, Combust, Dark Embrace, Disarm, Dropkick, Dual Wield, Entrench, Evolve, Feel No Pain, Fire Breathing, Flame Barrier, Ghostly Armor, Hemokinesis, Inflame, Intimidate, Metallicize, Power Through, Pummel, Rage, Rampage, Reckless Charge, Rupture, Searing Blow, Second Wind, Seeing Red, Sentinel, Sever Soul, Shockwave, Spot Weakness

### 10b — More Enemies
- [ ] All Act 1 normals: Cultist, Jaw Worm, 2× Louse (Red/Green), Small Slimes, Spike Slime (S/M/L), Acid Slime (S/M/L), Fungi Beast, Gremlin (all types), Looter, Mugger, Exordium Thugs/Wildlife
- [ ] Act 1 elites: Gremlin Nob, Lagavulin, 3× Sentries
- [ ] Act 1 boss: Hexaghost, Slime Boss, The Guardian

### 10c — More Powers
- [ ] Poison, Thorns, Metallicize, Ritual, Curiosity, Anger, Evolve, Feel No Pain, Flame Barrier, Rage, Rupture
- [ ] Enemy-specific: Curl Up, Spore Cloud, Split

### 10d — Run Structure
- [ ] Map generation (15×7, density 6) matching `MapGenerator.java`
- [ ] Room types: Monster, Elite, Rest, Event, Shop, Treasure, Boss
- [ ] Card rewards after combat (offer 3 cards, pick 1 or skip)
- [ ] Rest site: heal 30% or upgrade a card
- [ ] Gold drops and shop (buy cards / relics / potions / remove)

### 10e — Relics (starter set)
- [ ] Burning Blood (Ironclad starter): heal 6 HP after combat
- [ ] Common combat relics: Akabeko, Anchor, Art of War, Bag of Marbles, Bag of Preparation, Blood Vial, Bronze Scales, Centennial Puzzle, Cloak Clasp, Dream Catcher, Happy Flower, Lantern, Nunchaku, Odd Mushroom, Orichalcum, Ornamental Fan, Vajra

### 10f — Potions
- [ ] Potion slots, Fire Potion, Block Potion, Strength Potion, Dexterity Potion, Fear Potion, Explosive Potion

### 10g — Full Run RL
- [ ] Observation space extended to cover map state, gold, relics, deck composition
- [ ] Action space extended: map navigation, card reward choice, rest choice, shop actions
- [ ] Reward shaping for full run: floor depth, boss kills, final score

---

## Deferred / Out of Scope for Now
- The Silent, Defect, Watcher characters
- Ascension levels
- Daily challenges / modifiers
- Multiplayer / co-op (Spire with Friends mod)
- Full-run RNG replay across maps, events and rewards (combat RNG validation remains in scope)
