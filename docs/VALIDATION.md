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

## Remaining fidelity work

- Sentry Bolt, Lagavulin behavior, Guardian Mode Shift, slime split triggers,
  child HP, and several other enemy effects are still approximations.
- Some cards automatically select a card for upgrade/exhaust/discard instead
  of exposing a choice. Card upgrade costs and other effects need an audit.
- Java RNG sequence parity and live-game trace comparison are not established.
- Existing trained policies need reevaluation after combat correctness changes.

## WSL validation run (2026-10-03)

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
estimate was performed. Reproduction commands are in README.md. Checkpoints
and TensorBoard logs remain local, under the gitignored models/ and runs/ paths.
