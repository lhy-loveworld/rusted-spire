# Validation scope

The simulator is an approximation under active development. Passing regression
tests establishes the tested contracts, not full Slay the Spire parity.

## Power and intent corrections (2026-10-03)

Weak, Vulnerable and Frail decay at the end of the round, after enemy actions.
New enemy-turn debuffs skip their first decay; stacking an existing debuff does
not reset that flag. Metallicize grants block at the end of the owner's turn,
without Frail reduction. Ritual ticks at round end and skips its first tick;
Demon Form remains a start-of-turn effect, and Flex expires at player turn end.

The original decompiled Java files are absent from the WSL checkout. Timing was
cross-checked against the independent `sts_lightspeed` implementation at commit
`7476a81954020087da31d41d16fddf475746ec2d`:

- [Player round-end debuff decay](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/Player.cpp#L442)
- [Monster turn/round hooks](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/Monster.cpp#L41)
- [Player Metallicize and round sequencing](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/BattleContext.cpp#L2062)

Attack intents now use the same damage definitions as execution and refresh
after state changes without consuming RNG. Multi-hit damage is rounded per hit.
Tests compare announced damage with execution across all supported enemy IDs;
they do not validate every enemy's base damage or AI against the original game.

## Sentries and slime corrections (2026-10-03, interface v3)

Sentries begin with one Artifact charge, which consumes a debuff application
without preventing attack damage. Formation positions determine their first
move: Bolt / Beam / Bolt. They continue alternating after the five-entry move
history fills. Bolt adds two Dazed (three at A18) without damage; Beam deals nine
(ten at A3). Dazed is unplayable and exhausts from hand at player turn end.

Surviving large slimes and Slime Boss interrupt their move at half HP, then split
on their enemy action. Children use the parent's execution-time HP as current
and max HP, have fresh powers, and wait until the next phase to act. Killing the
parent prevents splitting; medium slimes never split. Formation order and
Python rollback on exceeding five living enemies have regression coverage.

Slime Boss now cycles Goop / Prepare / Slam beyond five turns. Prepare grants
no block; Slam is 35 (38 at A4); Goop adds three Slimed (five at A19); boss HP is
140 (150 at A9). Slime HP ranges, attack status counts, Lick debuffs, and large
Acid Slime Tackle damage were also corrected. Random slime move selection and
its ascension-dependent probabilities remain unaudited.

These contracts were cross-checked against the same independent implementation:

- [Sentry actions](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/MonsterSpecific.cpp#L1035)
- [Slime split threshold](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/Monster.cpp#L499)
- [Spawned child HP and split handling](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/MonsterSpecific.cpp#L3325)
- [Monster HP ranges](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/include/constants/MonsterIds.h#L152)
- [Ethereal hand cleanup](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/BattleContext.cpp#L2492)

## Remaining fidelity work

- Slime AI probabilities, Lagavulin behavior, Guardian Mode Shift, and several
  other enemy effects are still approximations.
- Some cards automatically select a card for upgrade/exhaust/discard instead
  of exposing a choice. Card upgrade costs and other effects need an audit.
- Java RNG sequence parity and live-game trace comparison are not established.
- Existing trained policies need reevaluation after combat correctness changes.

## WSL validation run (2026-10-03, interface v3)

- `cargo test --locked`: 55 Rust tests passed, including 15 new Sentry/slime tests.
- `python -m unittest discover -s tests -v`: 15 Python/Gymnasium tests passed,
  including 100 seeded episodes across six encounter configurations.
- Release Python extension rebuilt; interface is 178 observations / 61 actions.
- A 256-step MaskablePPO smoke run with three Sentries at A7, seed 42,
  two subprocess environments, rollout length 32 and four evaluation episodes
  every 128 steps completed and saved a checkpoint in `models/sentry_smoke_v3`.
  Reloaded evaluation on seeds 100000–100019 completed with zero truncations
  and zero wins (mean 24.05 actions). This validates the pipeline, not policy quality.

## Historical WSL training run (2026-10-03, interface v2)

- `cargo test --locked`: 40 Rust tests passed.
- `python -m unittest discover -s tests -v`: 15 Python/Gymnasium tests passed,
  including 100 seeded random-policy episodes and notebook wrapper checks.
- Release extension built with Python 3.12; CPU PyTorch 2.14.1+cpu and
  stable-baselines3 / sb3-contrib 2.9.0 were used for the training smoke test.
- MaskablePPO trained for 2,048 steps with seed 42, two subprocess environments,
  rollout length 64, and LouseNormal + LouseDefensive at ascension 7. Checkpoint
  saving/loading and both scheduled evaluations succeeded.

Held-out evaluation used environment seeds 100000–100099 and random-policy seed
1234, with the same encounter and ascension for both policies:

| Policy | Wins | Mean surviving HP | Mean actions | Truncations |
|---|---:|---:|---:|---:|
| Random legal actions | 100/100 | 57.45 | 13.98 | 0 |
| 2,048-step PPO checkpoint | 100/100 | 64.32 | 10.00 | 0 |

This easy encounter is a pipeline smoke test, not evidence of broad policy
quality or game parity. No multi-seed training comparison or uncertainty
estimate was performed. README.md now uses interface v3; these historical
numbers and v2 checkpoints do not carry forward to the corrected simulator. Checkpoints
and TensorBoard logs remain local, under the gitignored models/ and runs/ paths.
