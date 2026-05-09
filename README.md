# Rusted Spire

A headless [Slay the Spire](https://store.steampowered.com/app/646570/Slay_the_Spire/) simulator written in Rust, designed for Reinforcement Learning research.

---

## Motivation

Most existing Slay the Spire AI projects hook into the live Java game process via the [CommunicationMod](https://github.com/ForgottenArbiter/CommunicationMod). That works well for rule-based agents but is too slow for RL training, which needs thousands of rollouts per second. This project reimplements the core game logic from scratch in Rust — no JVM, no renderer, no frame budget — so training can run as fast as the CPU allows.

---

## Status

Early development. Currently implements:

| Component | Status |
|---|---|
| RNG (xorshift128, seeded, multiple streams) | ✅ |
| Damage pipeline (Strength, Vulnerable, Weak, Frail) | ✅ |
| Combat loop (player turn, enemy turn, win/lose) | ✅ |
| Cards | Strike, Defend, Bash |
| Enemies | Jaw Worm |
| Python bindings (PyO3) | Planned |
| RL training loop | Planned |

See [`PLAN.md`](PLAN.md) for the full roadmap and per-item progress.

---

## Requirements

- Rust 1.75+ (`rustup` recommended)
- Python 3.12+ with `uv` (for RL training, when bindings are ready)

---

## Build

```bash
cargo build
cargo test
```

---

## Play Interactively

```bash
cargo run --example play           # fixed seed 42
cargo run --example play -- 1234   # custom seed
```

This drops you into a terminal combat — Ironclad vs Jaw Worm. Type the action number and press Enter.

```
─────────────────────────────────────
Turn 1   Player  HP 80/80  Block 0  Energy 3/3
  Draw 5 | Discard 0
  Hand:
    [0] Strike  cost 1
    [1] Defend  cost 1
    [2] Strike  cost 1
    [3] Bash    cost 2
    [4] Defend  cost 1
  Enemies:
    [0] Jaw Worm  HP 42/42  Block 0  Intent: Attack 11

  Actions:
    [0] Play Strike → Jaw Worm
    [1] Play Defend
    [2] Play Strike → Jaw Worm
    [3] Play Bash   → Jaw Worm
    [4] Play Defend
    [5] End Turn
> _
```

---

## Architecture

The game's original code uses global statics and an animation-driven action queue. This simulator eliminates both:

- **No globals** — all mutable state lives in a single owned `CombatState` struct, making it trivial to clone for tree search or parallel rollouts.
- **No action queue** — card and enemy effects execute immediately rather than being deferred for rendering.

Key modules:

| Module | Responsibility |
|---|---|
| `rng` | xorshift128 matching libGDX `RandomXS128`; multiple named streams |
| `damage` | Power hook chain (`atDamageGive` → `atDamageReceive` → floor → clamp) |
| `power` | `Power` trait with default no-ops; enum dispatch to avoid boxing |
| `creature` | Shared HP / block / power state for players and enemies |
| `player` | Hand, draw/discard piles, energy, shuffle |
| `enemy` | Intent system, move history, per-enemy AI tables |
| `combat` | `CombatState`, `available_actions()`, `step()` |

---

## Prior Work

- **[gamerpuppy/sts_lightspeed](https://github.com/gamerpuppy/sts_lightspeed)** — a C++ simulator and tree-search engine for Slay the Spire, designed for fast random playouts and claimed to be 100% RNG-accurate. The closest prior work to this project in spirit.
- **[zhiyue/sts2-rl-agent](https://github.com/zhiyue/sts2-rl-agent)** — a MaskablePPO RL agent for Slay the Spire 2, combining a headless Python combat simulator (~1,200 combats/sec) with a C# mod that bridges the trained model back into the live game. Directly inspired the approach taken here.

---

## License

This project is an independent reimplementation. No game assets or original source code are included. Slay the Spire is the property of [Mega Crit](https://www.megacrit.com/).
