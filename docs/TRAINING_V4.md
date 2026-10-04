# First multi-seed training comparison, interface v4

Completed 2026-10-04 on simulator revision
`fbc1c70986f91c32bd00378e8ef333c55f6b2217`. All six final PPO checkpoints won
**0/300 held-out fights each**, as did each matched random baseline. No episodes
truncated. This experiment found no win-rate improvement; it does not establish
that these encounters are unwinnable or that PPO cannot learn them.

## Protocol

- Separate policies for sleeping Lagavulin and The Guardian, ascension 0.
- Ironclad starter deck (5 Strike, 4 Defend, 1 Bash), 80 HP, 3 energy; no relics,
  potions, deck additions or upgrades. This is a deliberately restricted combat
  setup, not the deck normally available by an Act 1 boss.
- Training seeds 11, 22, 33; 65,536 steps each, 393,216 steps total. Two CPU
  subprocess environments, 128 steps per rollout worker, batch size 256,
  10 epochs, learning rate 0.0003, gamma 0.99. Default MLP MaskablePPO.
- Reward: zero until victory (`1 + remaining HP / 80`) or defeat (`-1`).
- Evaluate final checkpoints deterministically; do not select using test results.
  Training's periodic validation uses its separate seed stream.
- Each policy uses environment seeds 200000–200299; actual combat seeds are
  recorded per episode. Baselines uniformly sample legal actions, including
  End Turn, using policy seeds 11, 22, 33. All policies share the same initial
  combats. These are 300 distinct test seeds reused across runs, not 900
  independent test seeds per encounter.

## Results

Every row below has zero wins and zero truncations in 300 episodes. Lower
remaining enemy HP means more damage dealt; longer episodes alone are not wins.

| Encounter | Policy | Seed | Mean actions | Mean enemy HP remaining |
|---|---|---:|---:|---:|
| Lagavulin | Random | 11 | 28.99 | 52.55 |
| Lagavulin | Random | 22 | 28.85 | 53.38 |
| Lagavulin | Random | 33 | 28.56 | 54.49 |
| Lagavulin | PPO final | 11 | 52.70 | 57.45 |
| Lagavulin | PPO final | 22 | 52.92 | 57.36 |
| Lagavulin | PPO final | 33 | 52.97 | 57.22 |
| Guardian | Random | 11 | 22.27 | 197.21 |
| Guardian | Random | 22 | 21.99 | 199.10 |
| Guardian | Random | 33 | 21.95 | 197.54 |
| Guardian | PPO final | 11 | 39.36 | 198.01 |
| Guardian | PPO final | 22 | 39.36 | 198.01 |
| Guardian | PPO final | 33 | 39.28 | 197.59 |

The per-policy Wilson 95% win-rate interval for 0/300 is approximately
0–1.26%, conditional on these evaluation seeds representing the target
distribution. It does not measure training-seed uncertainty. All mean episode
rewards are -1; victory HP is undefined. The three runs are reported separately
rather than pooling them as independent evaluation samples.

## Diagnosis and next experiment

Checkpoint reload and replay on the first 20 test seeds reproduced the saved
episode lengths and outcomes. PPO used roughly 26–27 Defends per Lagavulin
fight versus 7–8 for random, and 19–20 versus 5–7 against Guardian. It reached
later turns but left Lagavulin with more HP and Guardian with essentially the
same HP. These trace summaries are exploratory, not a second held-out test.

The result is consistent with learning to postpone defeat: with gamma 0.99,
delaying the same terminal -1 makes its discounted value less negative. This
is an interpretation supported by the reward definition and action traces,
not a causal ablation. Sparse wins and the restricted starter deck may also
contribute. More training at the same settings is not yet justified by these
results.

Next, establish a learning curriculum on ordinary encounters and an explicit
tactical baseline, then compare the existing reward with a separately specified
potential-based shaping variant. Preserve win rate as the evaluation metric,
use fresh test seeds for the next decision, and report all training seeds. Also
continue source/trace validation; simulator success would not establish original
game skill while remaining fidelity gaps are unresolved.

## Reproduction and artifacts

From a clean checkout at the revision above, rebuild the Python extension, then:

```bash
.venv/bin/python tools/benchmark_training.py --output models/benchmark_v4_initial
```

The command refuses an existing output directory to preserve prior artifacts.
For another run, choose a new directory. Exact numerical reproduction also
depends on the recorded package versions and runtime; a fixed seed alone does
not guarantee cross-platform training identity.

[benchmarks/v4_initial.json](benchmarks/v4_initial.json) contains per-run
summaries, protocol, package versions, checkpoint hashes and diagnostic counts.
Full per-episode results, checkpoints, training logs and validation outputs
remain local under `models/benchmark_v4_initial/`; that directory is gitignored.
The summary records the full results file's SHA-256. All six recorded training
revisions were clean and identical, and saved checkpoint hashes were verified.

Validation before training: 80 Rust tests and 16 Python/Gymnasium tests passed;
[GitHub CI](https://github.com/lhy-loveworld/rusted-spire/actions/runs/37180528450)
also passed, including the short PPO smoke test. These checks validate the
software contracts and pipeline, not the learning result or full game parity.
