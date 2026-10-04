# Configurable Ironclad combat decks

Both Python environments accept an ordered list of card names. Duplicates are
separate cards; a single `+` suffix applies that card's upgrade. Names are case
sensitive. Omitting `deck` preserves the original 10-card starter deck.

```python
import rusted_spire
from spire_env import SpireEnv
from deck_config import PRESETS

env = rusted_spire.SlayEnv(
    enemy="Lagavulin", ascension=0,
    deck=["Strike"] * 5 + ["Defend"] * 4 + ["Bash+", "TwinStrike", "Inflame"],
)
obs, mask = env.reset(42)

gym_env = SpireEnv(enemy="TheGuardian", ascension=0,
                   deck=list(PRESETS["act1_late"]))
obs, info = gym_env.reset(seed=42)
```

Construction validates the complete deck. An empty list, unknown card name,
malformed upgrade, or upgraded status card raises an error. `env.deck` returns
a copy of the original configuration. Every reset recreates it, so exhausted
cards, powers played, and generated statuses do not leak into the next fight.
The original ordering is preserved before the seeded opening shuffle.

## Example decks

These are illustrative snapshots assembled from audited card effects, not
optimal decks or a simulation of rewards earned during a run. They retain all
starter cards and omit relics/potions. Every fight still starts independently.

| Preset | Cards | Changes from the preceding preset |
|---|---:|---|
| `starter` | 10 | 5 Strike, 4 Defend, Bash |
| `act1_early` | 12 | Add Twin Strike and Shrug It Off |
| `act1_mid` | 14 | Upgrade Bash; add Cleave and Inflame |
| `act1_late` | 16 | Upgrade Twin Strike and Shrug It Off; add Heavy Blade and Metallicize |

The underlying names are `TwinStrike`, `ShrugItOff`, and `HeavyBlade`.
The late preset has three upgrades total. Use explicit lists for other card
rewards, removals or upgrade choices.

## Training, evaluation and files

```bash
.venv/bin/python train.py --deck-preset act1_late \
  --enemies TheGuardian --ascension 0 --timesteps 10000 --n-envs 2 \
  --save-path models/guardian_custom
.venv/bin/python evaluate.py --model models/guardian_custom/final \
  --enemies TheGuardian --ascension 0 --episodes 100
.venv/bin/python evaluate.py --deck-preset act1_late \
  --enemies TheGuardian --ascension 0 --episodes 100
```

Choose exactly one of `--deck-preset`, `--deck-file`, or `--deck`. A deck file is
a JSON array such as `["Strike", "Strike", "Defend+", "Bash+", "Inflame"]`.
`--deck Strike Strike Defend+ Bash+ Inflame` supplies the same format inline.

Training saves the resolved ordered list in `interface.json`, not just a preset
name or a file path. Model evaluation uses that saved deck unless a deck option
explicitly overrides it. Old metadata without a deck field implies the legacy
starter deck; absent metadata requires an explicit deck. Enemy configuration
and ascension still come from the evaluation command. Evaluation JSON includes
the actual deck used. The benchmark runner accepts the same deck options and
uses the resolved deck for every random baseline, worker and evaluation.

## Audited mechanics and corrections

The local Java references are under
`decompiled/sources/com/megacrit/cardcrawl/`. Expectations are source-derived;
the card tests do not execute the original game's combat engine.

| Cards | Java reference and checked behavior |
|---|---|
| Strike, Defend, Bash | `cards/red/Strike_Red.java`, `Defend_Red.java`, `Bash.java`: costs, upgraded damage/block, Bash's Vulnerable after damage |
| Twin Strike, Cleave | Corresponding `cards/red/` classes: 5/7 per hit twice; 8/11 AoE damage |
| Shrug It Off | `ShrugItOff.java`: 8/11 block then draw one |
| Inflame, Metallicize | Corresponding classes: 2/3 Strength, 3/4 Metallicize, cost one |
| Heavy Blade | `HeavyBlade.java`: base 14, Strength multiplier 3/5 before Weak/Vulnerable, including negative Strength |

`AbstractPlayer.useCard`, `DrawCardAction` and `UseCardAction` establish that a
played card leaves hand before its effects and reaches discard afterward. Rust
previously allowed draw cards to reshuffle themselves. Card cleanup is now
centralized, and a full hand gains the slot vacated by the card being played.
Power cards leave combat instead of entering exhaust. Intimidate exhausts.

Related inspected corrections: Entrench+ costs one and doubles existing block
without Dexterity/Frail adjustments (`DoubleYourBlockAction`); Sword Boomerang+
adds a hit without raising its three damage per hit; Headbutt is an Attack;
Warcry draws 1/2 and exhausts. Pommel Strike's damage/draw upgrades and draw
timing are also tested. This is not a complete audit of every configurable card.

## Current limits

Base Armaments, Warcry, Headbutt and upgraded True Grit require card-selection
actions that interface v4 does not represent. Deck configuration rejects them
with an explanation. Armaments+ is also rejected because its upgrade-all effect
is not implemented. Their older Rust implementations remain incomplete;
implementing selection states is the next step. Base True Grit remains usable
with random exhaust. Other configurable cards can still have unaudited behavior,
including Wild Strike's Wound placement and random targeting/selection order.

Interface v4 remains 196 observations / 61 actions. Deck composition is available
as environment configuration, but full piles and draw order remain hidden from
the policy. Existing v4 checkpoints still load, though changed card behavior and
new decks require reevaluation. This feature supplies combat snapshots, not
full Act 1 progression or proof of parity with the original game.

Validation: 89 Rust tests and 23 Python/Gym tests passed, including base/upgraded
preset effects, power cleanup, exhausted skills, draw/reshuffle boundaries,
configuration errors, isolated resets, and 40 preset elite/boss episodes. A
256-step late-preset Guardian training run saved and reloaded successfully;
20 evaluation episodes completed without truncation. Its 1/20 wins are a
pipeline smoke result, not evidence of policy improvement.
