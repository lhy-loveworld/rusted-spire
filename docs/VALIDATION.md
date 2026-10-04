# Validation scope

The simulator is an approximation under active development. Passing regression
tests establishes the tested contracts, not full Slay the Spire parity.

The original decompiled Java sources are now accessible in WSL.
The initial static comparison is recorded in [JAVA_AUDIT.md](JAVA_AUDIT.md),
including supported behavior and confirmed remaining differences. The sources
remain gitignored. A limited Java RNG/shuffle harness now generates fixtures
checked by Rust; no live full-game combat differential execution has been performed.

## Power and intent corrections (2026-10-03)

Weak, Vulnerable and Frail decay at the end of the round, after enemy actions.
New enemy-turn debuffs skip their first decay; stacking an existing debuff does
not reset that flag. Metallicize grants block at the end of the owner's turn,
without Frail reduction. Ritual ticks at round end and skips its first tick;
Demon Form remains a start-of-turn effect, and Flex expires at player turn end.

At the time of these changes, the decompiled Java files were absent from WSL. Timing was
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
its ascension-dependent probabilities were not corrected in this commit;
the subsequent Java audit identified discrepancies corrected in the follow-up below.

These contracts were cross-checked against the same independent implementation:

- [Sentry actions](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/MonsterSpecific.cpp#L1035)
- [Slime split threshold](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/Monster.cpp#L499)
- [Spawned child HP and split handling](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/MonsterSpecific.cpp#L3325)
- [Monster HP ranges](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/include/constants/MonsterIds.h#L152)
- [Ethereal hand cleanup](https://github.com/gamerpuppy/sts_lightspeed/blob/7476a81954020087da31d41d16fddf475746ec2d/src/combat/BattleContext.cpp#L2492)

## Java-guided AI and hook-order corrections

The local Java comparison and source paths are recorded in
[JAVA_AUDIT.md](JAVA_AUDIT.md). Acid/Spike slime move probabilities, repeat
restrictions, and fallback RNG decisions now follow those classes. Small Acid
Slime and Slime Boss set their next moves without extra AI rolls. Spike L's
discarded parent roll after splitting is preserved. Demon Form runs after the
normal draw, and all monster end-turn hooks precede player then monster
round-end hooks.

Validation: 63 Rust tests and 15 Python/Gymnasium tests pass, including eight
new AI/power tests and the existing 100 seeded combat episodes. Tests exercise
all primary AI rolls, ascension boundaries, repeat histories, fallback outcomes
and RNG call counts. For these AI/power tests, the Java sources are static references.
Interface v3 dimensions are unchanged; v3 policies need reevaluation because
combat behavior and subsequent seeded outcomes changed.

## Executed Java RNG and shuffle reference

The local `RandomXS128.java` was compiled unchanged and executed alongside
JDK `Collections.shuffle`. All 1,283 fixture cases match Rust: inclusive bounded
integers, a forced rejection/retry, signed seed bit patterns (including zero),
booleans, probability checks, float bits, long bits, and shuffled draw order.
The game wrapper and pile-direction mapping are based on static source inspection.
See [RNG_VALIDATION.md](RNG_VALIDATION.md) for provenance, regeneration and limits.

Corrections include integer sampling, boolean bit selection, identical starting
seeds for independent combat streams, and one game RNG call per shuffle followed
by a fresh Java-compatible 48-bit generator. Both initial decks and discard
reshuffles are tested. Validation: 66 Rust tests and 15 Python/Gymnasium tests
pass, including the existing 100 seeded episodes. No JDK or game installation
is required to run the committed fixture tests.

## Executed original-bytecode action traces

[COMBAT_TRACES.md](COMBAT_TRACES.md) documents 44 matching snapshots across 15
controlled scenarios executed from the local original JAR. The driver preserves
original card/action/damage methods but supplies controlled actors, mock graphics
and an isolated queue scheduler. It does not establish full fight/turn/AI parity.
The comparison found and fixed early Curl Up block and Wound/Dazed cost encoding.
101 Rust tests and 28 Python/Gym tests pass; fixtures include provenance hashes.

## Remaining fidelity work

- Lagavulin and Guardian now have source-derived corrections; see
  [ELITE_BOSS_AUDIT.md](ELITE_BOSS_AUDIT.md). Other enemy AI/effects and unsupported reactions still require audits. Curl Up
  timing now matches the executed action traces described above.
- Armaments, True Grit+, Warcry and Headbutt now expose choices (see below).
  Other card effects and interactions still need audits.
- Full combat RNG call order and live-game trace comparison are not established;
  the primitive/shuffle fixture agreement does not establish whole-combat parity.
- Existing trained policies need reevaluation after combat correctness changes.

## Configurable decks and preset-card audit

Python, Gymnasium, training, evaluation and the benchmark runner now accept
explicit decks and upgraded cards. Early/mid/late Act 1 presets use card effects
checked against their Java classes. Card-in-use cleanup fixes self-reshuffling
draw cards, powers entering exhaust, and Intimidate failing to exhaust. Entrench
upgrade cost/block doubling and Sword Boomerang's upgrade damage were corrected.
This v4 milestone initially rejected selection-dependent cards; v5 now supports
them as described below. See [DECKS.md](DECKS.md) for scope and remaining gaps.

89 Rust tests and 23 Python/Gym tests passed. A custom-deck training/checkpoint
reload smoke run and 20 Guardian evaluation episodes completed. These checks
validate the configuration and tested card effects, not broad policy quality.

## Card selection (interface v5)

Armaments, True Grit+, Headbutt and Warcry now suspend card resolution for a
mandatory choice; Armaments+ upgrades all eligible hand cards. Interface v5 is
242 observations / 73 actions, including paginated discard choices. Source
checks also corrected Anger timing and singleton True Grit RNG consumption.
See [CARD_SELECTION.md](CARD_SELECTION.md) for exact behavior and limitations.

100 Rust tests and 27 Python/Gym tests passed. New coverage includes every choice
type, automatic choices, duplicate cards, paging, draw boundaries, invalid-action
preservation, cleanup/reactions and resets. Thirty seeded selection-deck episodes
are reproducible through Gymnasium. A 256-step PPO smoke run saved/reloaded with
the selection deck and completed 20 Cultist evaluations without truncation.
Original-game combat trace parity and policy improvement remain unestablished.

## Constructor and elite/boss corrections (interface v4)

Constructor HP/damage rolls now precede formation-wide pre-battle rolls,
correcting multi-Louse initialization. Fixed boss HP still consumes a roll;
explicit-HP slime children do not. Lagavulin's waking, sleep duration and
permanent Dexterity/Strength loss are implemented. Guardian now has its
offensive/defensive cycle, Mode Shift thresholds, and Sharp Hide retaliation.
Source paths, ordering decisions and limits are in
[ELITE_BOSS_AUDIT.md](ELITE_BOSS_AUDIT.md).

Validation: 80 Rust tests and 16 Python/Gymnasium tests, including 100 seeded
episodes now covering Lagavulin and Guardian. New observations expose the
relevant powers and Sleep/Stun intents: v4 is 196 / 61. Old checkpoints cannot
load into the changed observation shape and need retraining.

`tools/benchmark_training.py` runs independent training seeds, preserves final
checkpoints/logs, and compares them with uniform legal-action baselines on
matched held-out seeds. It records per-episode outcomes, remaining enemy HP,
Wilson win-rate intervals, package versions, Git revision and checkpoint hashes.
Intervals describe evaluation-seed uncertainty for each policy, not training
variance. Training seeds are reported separately. The completed
[v4 comparison](TRAINING_V4.md) found 0/300 wins for each of six final PPO
checkpoints and each random baseline, with no truncations. Longer episodes
did not translate into wins. Passing tests alone is not evidence of learning.

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
