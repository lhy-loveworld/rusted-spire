# Executed original-bytecode combat traces

The simulator now matches **530 decision-boundary snapshots** generated from the
locally installed original game JAR: 54 across 19 card-action scenarios, 46
across eight controlled Cultist turn scenarios, 200 across 25 Act 1 Jaw Worm
scenarios, and 230 across 85 encounter-start scenarios.
These expectations are produced by executing the shipped classes, not by
transcribing the decompiled methods or recording Rust's own output.

This is a controlled action/turn comparison, not a complete fight, full game-loop
replay, or proof of whole-game parity. Selected monster constructors and opening
draws are now executed, but untested enemies, victory/death and run progression
remain outside this suite.

## Reference and execution boundary

- Game reports `[V2.3.4] (12-18-2022)`; local Steam manifest build ID: `10180494`.
- JAR SHA-256: `cfad868ac8d65a88e71a0bf096fb09f78811e553effe0787c5309a655e081673`.
- The recorded execution uses Linux OpenJDK 21 in WSL against the Windows
  installation's JAR, using its bundled Linux libGDX native library.
- [java_combat.json](../tests/fixtures/java_combat.json) and
  [java_turns.json](../tests/fixtures/java_turns.json), plus
  [java_jaw_worm.json](../tests/fixtures/java_jaw_worm.json) and
  [java_encounters.json](../tests/fixtures/java_encounters.json), record the JVM version,
  JAR hash, driver/scenario/fixture hashes, and action classes entered at each
  boundary. No game code, graphics, JARs or save files are checked in.

`tools/CombatOracle.java` uses original card constructors/upgrades, player
`useCard`, damage calculations, power hooks, action `update` methods, card-group
movement, selection actions, and RNG. It does not transform or replace those
methods. A bounded driver drains the game's action queue in order and pauses
when a real hand/grid selection screen opens. It feeds the scripted selection
into that screen and resumes the original action. A 0.05-second mock frame clock
advances action durations and pile animations; rendered visual effects are not
run through the full application update loop.

Graphics/audio/input are mocked. Fonts and textures are placeholders, tutorial
prompts are disabled, and preferences are in-memory. Character, target and room
constructors are bypassed with Java `Unsafe` in controlled scenarios, then
explicitly initialized. Each controlled scenario starts with an Ironclad at 80 HP
and 3 energy and a Cultist or Jaw Worm
target at 200 HP. Card-only scenarios keep the target stationary. Listed powers,
block and piles are injected into both runtimes;
the target can carry synthetic powers such as Curl Up for focused interaction
tests. There are no relics or potions.

Turn scenarios initialize the Cultist's constructor-owned `firstMove` and
`ritualAmount` fields, or Jaw Worm's initial-move flag and ascension-dependent
attack/block/Strength fields, then call the original `rollMove` and `createIntent`.
Jaw Worm uses Act 1 settings (`hardMode=false`); its Act 3 pre-battle bonus is
outside the simulator's supported scope.
An `end` directive invokes the original end-turn button and drives
`GameActionManager.update`, including monster queues, `takeTurn`, next-move rolls,
power hooks, energy recharge and the next draw. At the idle boundary after the
end-turn card queue drains, the harness makes the same handoff to
`AbstractRoom.endTurn` as the normal player-input/room loop. It stops after the
turn number advances, all queues drain, and the end-turn button is enabled.
Empty Spine hit/idle animations satisfy the original Ironclad damage method's
visual dependency; Jaw Worm also receives empty idle/chomp/tailslam animations
and a screen-shake helper. No damage or AI method is replaced. These controlled
turn cases do not exercise actor constructors or the full room/input/render loop.

Encounter scenarios keep the mock Ironclad/room but execute real Cultist,
Act 1 Jaw Worm, and red/green Louse constructors, including their bundled texture
and animation loading. No enemy HP, damage, powers, or AI fields are assigned by
the harness in this mode. It constructs all listed monsters, calls
`MonsterGroup.init`, then runs `AbstractPlayer.preBattlePrep` with the supplied
ordered master deck. The original room battle-start branch queues energy and
opening-draw actions. The driver drains them and captures the first player turn.
Inputs are effective combat seeds, not run seeds requiring a floor offset.
Relics, potions, innate/bottled cards, deck generation and automatic Ascender's
Bane insertion are outside this explicit-deck test setup.

This setup deliberately isolates combat semantics. It must not be described as
running an unmodified full game session. The original combat methods are executed,
but their environment and the outer scheduler are supplied by the harness.

## Compared state and scenarios

At setup and after every play/choice/end-turn, the regression compares phase, HP, block,
energy, ordered hand/draw/discard/exhaust piles (including upgrades and current
cost), target HP/block, power amounts, card/shuffle RNG counters and choice options.
Draw piles are normalized to next-card-first. During Armaments' choice screen,
the original UI temporarily removes ineligible cards; the snapshot includes their
logical presence using the captured hand, without modifying Java state. Choice
options remain the actual eligible cards presented by the original action.

Scenarios cover Bash followed by Strike; Twin Strike versus Curl Up with and
without initial block; Strength/Weak/Vulnerable and negative Strength; Entrench
with Dexterity/Frail; Pommel Strike reshuffling; Warcry drawing and selecting an
existing card; singleton Warcry RNG; filtered Armaments and Armaments+; selected
and singleton True Grit; Headbutt with Sharp Hide and with a pending Curl Up.
The Cultist turn scenarios cover Ascensions 0, 1, 2, 16, 17 and 20; initial Incantation
followed by two attacks; interleaved Bash/Defend plays; end-turn discard order;
Dazed exhaust; repeated reshuffles; energy/block reset; Flex expiry; Demon Form;
Metallicize; and Weak/Vulnerable/Frail decay. Turn snapshots additionally compare
the turn number, named next move, intent damage, and AI RNG counter, including
the initial move roll. HP RNG is excluded because actor construction is bypassed.
This checks selected fields at decision boundaries, not every private field or
intermediate queue state. Non-attacking intent damage is normalized to zero:
the original caches old attack damage even when the current move is a buff.

Jaw Worm coverage adds 171 executed enemy turns with all three moves, both
outcomes of each random repeat fallback, histories longer than Rust's five-entry
window, and Ascensions 0/1/2/16/17/20. Eighteen eight-turn scenarios use injected
Metallicize 30 on the player to keep the AI sequences alive; six unprotected
four-turn scenarios compare actual HP loss at the ascension boundaries. One
scenario interleaves attacks/block with Weak, Vulnerable and Frail. No primary
AI roll or fallback outcome is forced: all are drawn by original `rollMove` calls.
A fixture-coverage regression requires all six fallback outcomes to remain
present. Separate source-guided unit tests cover every primary roll and exact
float fallback probabilities/RNG state; these are not additional JAR snapshots.

Encounter coverage includes six seeds (including zero and the largest signed
64-bit seed), Ascensions 0/2/6/7/17/20, an ordered starter deck, an upgraded
seven-card deck, a three-card deck and an empty deck. Seventy-two Cultist/Jaw Worm
cases compare initialization followed by two enemy turns; the empty-deck case
continues one turn. Twelve two-Louse cases compare initialization only, checking
both constructors' HP/damage rolls before both Curl Up rolls. Encounter snapshots
add the HP RNG counter, max HP, and variable damage roll (Louse bite damage;
zero for enemies without a variable roll). Enemy fields are separated by `|`
in formation order. Louse turn/AI parity is not claimed by these setup checks.
Additional action cases check Warcry/Pommel overdraw, exact draw and hand limits.

## Divergences found and fixed

1. **Curl Up resolved too early.** Against a target with Curl Up 9, Twin Strike
   previously dealt 5 total HP damage and left 4 block. The original deals 10 HP
   damage and then grants 9 block: Curl Up enqueues its block behind both hits.
   Rust now records the trigger and resolves the block/removal after the card's
   effects and cleanup. Headbutt also preserves the pending power during selection.
2. **Unplayable status cost encoding differed.** Rust used `99`; Wound and Dazed
   use `-2` in the game. Rust now exposes `-2`; the explicit unplayable-card check
   continues to prevent playing them, regardless of available energy.
3. **End-turn discard order was reversed.** The game repeatedly discards the last
   hand entry. Rust previously discarded from the first entry, changing subsequent
   seeded shuffles and drawn hands. Rust now exhausts Ethereal cards and discards
   the remaining hand from last to first.
4. **Ascension 17 Cultist Ritual was too weak.** The original grants 3 below A2,
   4 at A2–16, and 5 at A17+. Rust previously used 4 at all A2+ levels. The
   multi-turn trace also catches the resulting Strength, intent and HP differences.

5. **Jaw Worm repeat handling skipped RNG.** Rust chose a fixed alternative for
   forbidden repeats. The original uses one extra probability-based roll: after
   Chomp, 0.5625 selects Bellow over Thrash; after two Thrashes, 0.357 selects Chomp
   over Bellow; after Bellow, 0.416 selects Chomp over Thrash. The missing rolls
   changed subsequent moves and AI state. Rust now uses the same probabilities.
6. **Jaw Worm ascension bonuses were absent.** Chomp now deals 12 from A2 onward
   (11 below A2). Bellow grants 3 Strength/6 block below A2, 4/6 at A2–16, and
   5/9 at A17+. Thrash remains 7 damage/5 block.

7. **Overdraw omitted an empty shuffle.** When a draw request exceeds the cards
   remaining, the original queues a shuffle even after drawing the last available
   card. That empty shuffle consumes RNG. Rust previously stopped immediately,
   so short-deck opening hands matched but later hands diverged. Drawing now
   follows the original split-draw/shuffle sequence. Starting with both piles
   empty, drawing exactly the available count, and stopping at the hand limit
   still avoid an extra shuffle.

Interface dimensions remain v5 (242 observations / 73 actions). These corrections
change behavior, seeded trajectories and status observations, so existing v5
policies need reevaluation.

## Regenerate and test

Use a locally owned game JAR and JDK 17+; the checked-in Java driver uses switch
expressions. The JAR must provide the libGDX headless classes/native library.

```bash
.venv/bin/python tools/generate_combat_fixtures.py \
  --jar /path/to/SlayTheSpire/desktop-1.0.jar --java-home /path/to/jdk
.venv/bin/python tools/generate_combat_fixtures.py \
  --jar /path/to/SlayTheSpire/desktop-1.0.jar --java-home /path/to/jdk \
  --scenarios tests/fixtures/turn_scenarios.tsv --output tests/fixtures/java_turns.tsv
.venv/bin/python tools/generate_combat_fixtures.py \
  --jar /path/to/SlayTheSpire/desktop-1.0.jar --java-home /path/to/jdk \
  --scenarios tests/fixtures/jaw_worm_scenarios.tsv --output tests/fixtures/java_jaw_worm.tsv
.venv/bin/python tools/generate_combat_fixtures.py \
  --jar /path/to/SlayTheSpire/desktop-1.0.jar --java-home /path/to/jdk \
  --scenarios tests/fixtures/encounter_scenarios.tsv --output tests/fixtures/java_encounters.tsv
cargo test --locked --test java_combat_traces
.venv/bin/python -m unittest discover -s tests -v
```

The generator compiles and runs in a temporary directory: game diagnostic files
and display settings stay there, not in the repository or installation. It checks
process failures, timeout and snapshot counts, and never substitutes Rust results
for unavailable Java output. `--output` can select a separate comparison file.
Inspect changed hashes and trace values before accepting a new reference build.

Normal tests/CI only consume the committed numeric/card-state fixtures. The Rust
test reports the scenario, step and differing field; the Python provenance test
detects stale driver/scenario/fixture hashes. A second independent Java execution
produced identical snapshots and metadata for all four suites. All 107 Rust tests and
31 Python/Gym tests pass.

The next extension should follow initialized encounters through scripted card
plays to victory or defeat, validating lethal-hit cleanup and terminal state. This suite
provides executed combat references alongside the RNG primitive fixtures and
source audits; it does not establish encounter-wide parity.
