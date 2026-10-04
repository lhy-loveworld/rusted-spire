# Card selection (interface v5)

Armaments, True Grit+, Headbutt and Warcry now pause combat for a card choice.
Both base and upgraded versions can be supplied through `SlayEnv(deck=...)`,
`SpireEnv(deck=...)` and the existing deck CLI options. Armaments+ upgrades all
eligible cards in hand without prompting.

The original 196 observation fields and actions 0–60 retain their positions.
Version 5 adds choice observations and actions: **242 observations / 73 actions**.
Checkpoints from v4 or earlier need retraining. See [RL_INTERFACE.md](RL_INTERFACE.md)
for the complete encoding.

## Behavior

| Card | Before choosing | Legal choices | After choosing |
|---|---|---|---|
| Armaments | Gain 5 block, modified by Dexterity/Frail | Unupgraded non-status cards in hand | Upgrade selected card for this combat; discard Armaments |
| Armaments+ | Gain 5 block | No prompt | Upgrade all eligible hand cards; discard Armaments+ |
| True Grit+ | Gain 9 block, modified by Dexterity/Frail | Any remaining hand card | Exhaust selected card; discard True Grit+ |
| Headbutt / Headbutt+ | Deal 9 / 12 damage, modified normally | Any card already in discard | Put selected card on top of draw pile; discard Headbutt |
| Warcry / Warcry+ | Draw 1 / 2, respecting the hand cap and reshuffles | Any remaining hand card, including cards held before drawing | Put selected card on top of draw pile; exhaust Warcry |

No candidates means the selection effect does nothing. One candidate resolves
automatically, matching the source shortcuts. Headbutt skips its selection when
its damage ends combat. The card in use is outside all piles until completion:
Headbutt cannot retrieve itself, and Warcry cannot draw or select itself.

During a prompt only choosing a card or changing choice pages is legal. Playing
another card and ending the turn are masked out. Energy is charged once when
the card is played. Choice/page actions do not advance turns or rerun on-use
hooks. The Python wrapper rejects illegal actions without modifying state.
Reset clears the pending choice and restores the original deck and upgrades.

Options retain distinct card instances, including duplicates. Hand selections
fit on one page; discard selections use pages of ten without imposing a discard
pile size limit. Navigation changes only the displayed page; it consumes no RNG,
energy or game time. Gymnasium counts navigation and selections as decisions
toward `max_steps`, so a policy that navigates forever is eventually truncated.

## Source evidence

All paths below are relative to the local, ignored
`decompiled/sources/com/megacrit/cardcrawl/` tree:

- `cards/red/{Armaments,TrueGrit,Headbutt,Warcry}.java`: costs, upgrades and queued effect order.
- `actions/unique/ArmamentsAction.java`: eligibility, automatic singleton,
  upgrade-all, and hand ordering after a multi-card choice. Eligible unchosen
  cards remain first, followed by the upgraded selected card, then excluded cards.
- `actions/common/ExhaustAction.java`: selected versus random exhaust;
  zero/singleton hand paths do not consume RNG for base True Grit either.
- `actions/common/PutOnDeckAction.java`: topdeck from the whole hand, even if
  nothing was drawn. Its automatic singleton path still calls card RNG once.
- `actions/unique/DiscardPileToTopOfDeckAction.java`: discard choice and
  battle-ending/empty/singleton shortcuts.
- `characters/AbstractPlayer.java`, `actions/utility/UseCardAction.java`:
  card-in-use lifetime and on-use hooks before cleanup.
- `powers/AngerPower.java`: Strength is queued at the front, before the skill's
  effects and selection. `powers/SharpHidePower.java`: retaliation is queued
  after the attack's effects, including its choice, before cleanup.

These are source-derived implementations and regression expectations, not
executed original-game combat traces. The simulator still lacks a general action
queue and many powers/relics. UI animation and confirmation clicks are omitted;
the chosen card resolves immediately. Broader RNG-consumption and reaction
parity remains to be established against controlled original-game traces.

## Validation and usage

```bash
.venv/bin/python train.py --timesteps 256 --n-envs 2 --n-steps 32 \
  --eval-freq 128 --eval-episodes 4 --enemies Cultist --ascension 0 \
  --deck Armaments TrueGrit+ Headbutt Warcry+ Strike Defend BodySlam ShrugItOff \
  --save-path models/selection_smoke_v5
.venv/bin/python evaluate.py --model models/selection_smoke_v5/final \
  --enemies Cultist --ascension 0 --episodes 20
```

100 Rust tests and 27 Python/Gym tests pass. Coverage includes every choice type,
upgrades/costs, filtered/empty/singleton choices, full-hand draw and reshuffle,
23-card discard pagination, duplicates, illegal-action preservation, deferred
cleanup/retaliation, lethal Headbutt, reset isolation and 30 reproducible seeded
selection-deck episodes. CI also trains and reloads a selection-deck policy.
The local 256-step smoke run reloaded and completed 20 Cultist evaluations with
zero truncations. This establishes pipeline operation, not policy improvement.
