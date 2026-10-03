# RL interface v2

`rusted_spire.INTERFACE_VERSION == 2`. Old 83-observation / 11-action policies
are incompatible and must be retrained. Current dimensions are 172 / 61.
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
| 8–47 | Ten hand slots: card ID, cost, upgraded, playable | 28, 3, boolean, boolean |
| 48–61 | Player powers and timing flags | As below |
| 62–171 | Five enemy slots, 22 values each | As below |

Each enemy slot has: alive flag; enemy ID / 21; HP / max HP; max HP / 300;
block / 100; intent type / 6; damage per hit / 20; hit count / 4; then powers.
Missing slots are all zero. Non-attacks have zero damage and hit count.

Power entries are Strength / 10, Vulnerable / 5, Weak / 5, Frail / 5,
Ritual / 5, Curl Up / 12, Anger / 5, Metallicize / 10, Demon Form / 5,
Strength Down / 10, then three fresh-debuff flags (Vulnerable, Weak, Frail)
and Ritual's skip-first-tick flag.

Enemy ID order: JawWorm, Cultist, LouseNormal, LouseDefensive, FungiBeast,
AcidSlimeSmall, AcidSlimeMedium, SpikeSlimeSmall, SpikeSlimeMedium, MadGremlin,
SneakyGremlin, FatGremlin, ShieldGremlin, GremlinWizard, GremlinNob, Lagavulin,
Sentry, SlimeBoss, AcidSlimeLarge, SpikeSlimeLarge, TheGuardian (1–21).
Intent types: unknown=0, attack=1, attack+debuff=2, attack+block=3, buff=4,
debuff=5, defend=6. Multi-hit attacks use type 1 and a hit count above one.

This remains a partial observation: draw order, pile composition and enemy move
history are not exposed. The policy does not receive omniscient simulator state.

## Gymnasium and evaluation

`spire_env.SpireEnv` is shared by the CLI and notebook. `reset(seed=...)` seeds
Gymnasium's RNG, which supplies a reproducible stream of combat seeds across
subsequent unseeded resets. Reset info includes the actual `combat_seed`.
For an exact Rust seed, use `SlayEnv.reset(seed)` directly.

`max_steps` defaults to 1000; exceeding it produces `truncated=True`, distinct
from victory/defeat (`terminated=True`). Reset is required after either. Invalid
actions raise exceptions rather than consuming a step or mutating combat.

`evaluate.py` uses explicit environment and policy seeds and reports win rate,
reward, episode length, victory HP, and truncations. Compare policies using the
same enemy configuration and seed range; training alone is not evidence of an
improvement over the random baseline.
