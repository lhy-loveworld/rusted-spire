# rusted-spire — Agent Instructions

Headless Slay the Spire combat simulator in Rust, exposed to Python via PyO3, used for RL training with MaskablePPO.

## Decompiled source index

The original game JAR has been decompiled to `decompiled/sources/com/megacrit/cardcrawl/` (~2008 Java files).  
**Before exploring the decompiled tree, read `decompiled/INDEX.md` first** — it maps every package and key class to its purpose and lists the exact files to check for common topics (damage calc, monster AI, RNG, power hooks, card mechanics, etc.).

The decompiled source is gitignored (copyrighted). `decompiled/INDEX.md` is tracked.

## Project layout

```
src/          Rust simulator (card, combat, creature, damage, enemy, obs, player, power, rng)
src/python.rs PyO3 bindings — SlayEnv class
examples/     play.rs — interactive terminal test
train.py      Headless MaskablePPO training script
experiments.ipynb  Jupyter notebook for RL experiments
models/       Saved model checkpoints (gitignored)
runs/         TensorBoard logs (gitignored)
```

## Python tooling

Always use `uv` inside `.venv` — never `pip` or `pip3` directly.

```bash
source .venv/bin/activate
uv pip install <package>
```

## Building the extension

```bash
maturin develop --features python   # debug build into .venv
maturin develop --release --features python  # optimised
```

## Running tests

```bash
cargo test
```
