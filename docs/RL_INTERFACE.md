# RL interface v4

`rusted_spire.INTERFACE_VERSION == 4`. Older v1 (83/11), v2 (172/61) and
v3 (178/61) policies are incompatible and must be retrained.
Current dimensions are 196 / 61. Version 4 adds Dexterity, Mode Shift and
Sharp Hide, plus distinct Sleep/Stun intents. Dazed remains card ordinal 29.
Training writes the version, dimensions, and command arguments to
`interface.json` beside the checkpoint. Evaluation passes the environment to
SB3 on load so incompatible observation/action dimensions are rejected.

## Actions

For hand slot `h` (0–9), action `6*h+t` chooses target slot `t` (0–4).
`6*h+5` plays an untargeted card. Action 60 ends the turn. The legal-action mask
enables exactly one representation for untargeted cards and one per living
target for targeted cards. Empty slots, unaffordable and unplayable cards, and
all terminal-state actions are masked out.

Enemy slots enumerate **living enemies in vector order** in the current
observation. Slots compact after a death; use the newly returned observation
and mask on every decision. Dead entries in the simulator's vector never hide
newly spawned enemies. At most five living enemies can be represented. A Python
step that would exceed capacity raises `ValueError` and leaves state unchanged;
such encounters need a larger interface before they can be used for training.

## Observations

Normalization divisors below define units, not bounds. Strength and growing
card piles can exceed their scales. The Gymnasium space permits all finite
float32 values; consumers should not assume the old [-1, 2] interval.

| Offset | Features | Normalization |
|---|---|---|
| 0–7 | Player HP, block, energy, draw/discard/exhaust counts, hand size, turn | max HP, 100, max energy, 10/10/10, 10, 100 |
| 8–47 | Ten hand slots: card ID, cost, upgraded, playable | 29, 3, boolean, boolean |
| 48–65 | Player powers and timing flags | As below |
| 66–195 | Five enemy slots, 26 values each | As below |

Each enemy slot has: alive flag; enemy ID / 21; HP / max HP; max HP / 300;
block / 100; intent type / 7; damage per hit / 20; hit count / 4; then powers.
Missing slots are all zero. Non-attacks have zero damage and hit count.

Power entries are Strength / 10, Vulnerable / 5, Weak / 5, Frail / 5,
Ritual / 5, Curl Up / 12, Anger / 5, Metallicize / 10, Demon Form / 5,
Strength Down / 10, Artifact / 3, Dexterity / 10, Mode Shift remaining HP / 50,
Sharp Hide / 4, then three fresh-debuff flags (Vulnerable, Weak, Frail)
and Ritual's skip-first-tick flag.

Enemy ID order: JawWorm, Cultist, LouseNormal, LouseDefensive, FungiBeast,
AcidSlimeSmall, AcidSlimeMedium, SpikeSlimeSmall, SpikeSlimeMedium, MadGremlin,
SneakyGremlin, FatGremlin, ShieldGremlin, GremlinWizard, GremlinNob, Lagavulin,
Sentry, SlimeBoss, AcidSlimeLarge, SpikeSlimeLarge, TheGuardian (1–21).
Intent types: unknown=0, attack=1, attack+debuff=2, attack+block=3, buff=4,
debuff=5, defend=6, split=7, sleep=8, stun=9. The divisor remains 7, so Sleep/Stun
encode above 1. Multi-hit attacks use type 1 and a hit count above one; Guardian's
Twin Slam also restores offensive mode, but its intent uses that attack encoding.

Slimes announce Split after surviving damage at or below half HP; the split
executes on their next action. Children occupy the parent's place in formation
order, have its remaining HP as both current and max HP, and do not act in the
phase in which they spawn. A lethal hit prevents splitting. Use the returned
mask after every transition, since splitting changes the target slots.

This remains a partial observation: draw order, pile composition and enemy move
history are not exposed. The policy does not receive omniscient simulator state.

## Gymnasium and evaluation

`spire_env.SpireEnv` is shared by the CLI and notebook. `reset(seed=...)` seeds
Gymnasium's RNG, which supplies a reproducible stream of combat seeds across
subsequent unseeded resets. Reset info includes the actual `combat_seed`.
For an exact Rust seed, use `SlayEnv.reset(seed)` directly.

The Rust seed is an **effective combat seed**, shared by independent AI,
shuffle, HP, card and misc streams. The game reseeds these streams with run
seed + floor; this simulator has no floor/run model. RNG primitives and deck
shuffles now follow the [Java reference fixtures](RNG_VALIDATION.md), replacing
the earlier stream offsets and shuffle algorithm. Identical numeric seeds
therefore produce different episodes than earlier revisions. Interface v3
was subsequently superseded by v4's new powers and dimensions. Matching a
whole original-game combat still requires validating random-call order.

`max_steps` defaults to 1000; exceeding it produces `truncated=True`, distinct
from victory/defeat (`terminated=True`). Reset is required after either. Invalid
actions raise exceptions rather than consuming a step or mutating combat.

`evaluate.py` uses explicit environment and policy seeds and reports win rate,
reward, episode length, victory HP, and truncations. Compare policies using the
same enemy configuration and seed range; training alone is not evidence of an
improvement over the random baseline.
