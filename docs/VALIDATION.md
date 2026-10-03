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
