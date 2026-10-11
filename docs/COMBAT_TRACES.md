# Executed original-bytecode combat traces

The simulator now matches **90 decision-boundary snapshots** generated from the
locally installed original game JAR: 44 across 15 card-action scenarios and 46
across eight controlled Cultist turn scenarios.
These expectations are produced by executing the shipped classes, not by
transcribing the decompiled methods or recording Rust's own output.

This is a controlled action/turn comparison, not a complete fight, full game-loop
replay, or proof of whole-game parity. Constructor RNG, opening draws, other
enemies, victory/death and run progression remain outside this suite.

## Reference and execution boundary

- Game reports `[V2.3.4] (12-18-2022)`; local Steam manifest build ID: `10180494`.
- JAR SHA-256: `cfad868ac8d65a88e71a0bf096fb09f78811e553effe0787c5309a655e081673`.
- The recorded execution uses Linux OpenJDK 21 in WSL against the Windows
  installation's JAR, using its bundled Linux libGDX native library.
- [java_combat.json](../tests/fixtures/java_combat.json) and
  [java_turns.json](../tests/fixtures/java_turns.json) record the JVM version,
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
constructors are bypassed with Java `Unsafe`, then explicitly initialized. Each
scenario starts with an Ironclad at 80 HP and 3 energy and a Cultist
target at 200 HP. Card-only scenarios keep the target stationary. Listed powers,
block and piles are injected into both runtimes;
the target can carry synthetic powers such as Curl Up for focused interaction
tests. There are no relics or potions.

Turn scenarios initialize the Cultist's constructor-owned `firstMove` and
`ritualAmount` fields, then call the original `rollMove` and `createIntent`.
An `end` directive invokes the original end-turn button and drives
`GameActionManager.update`, including monster queues, `takeTurn`, next-move rolls,
power hooks, energy recharge and the next draw. At the idle boundary after the
end-turn card queue drains, the harness makes the same handoff to
`AbstractRoom.endTurn` as the normal player-input/room loop. It stops after the
turn number advances, all queues drain, and the end-turn button is enabled.
Empty Spine hit/idle animations satisfy the original Ironclad damage method's
visual dependency. No damage or AI method is replaced. This still does not
exercise actor constructors or the full room/input/render loop.

This setup deliberately isolates action semantics. It must not be described as
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
The turn scenarios cover Ascensions 0, 1, 2, 16, 17 and 20; initial Incantation
followed by two attacks; interleaved Bash/Defend plays; end-turn discard order;
Dazed exhaust; repeated reshuffles; energy/block reset; Flex expiry; Demon Form;
Metallicize; and Weak/Vulnerable/Frail decay. Turn snapshots additionally compare
the turn number, named next move, intent damage, and AI RNG counter, including
the initial move roll. HP RNG is excluded because actor construction is bypassed.
This checks selected fields at decision boundaries, not every private field or
intermediate queue state. The Cultist has a fixed move sequence; random AI
branching remains unvalidated by this executed suite.

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
produced identical snapshots and metadata for both suites. All 102 Rust tests and
29 Python/Gym tests pass.

The next extension should cover an enemy with random move choices, such as Jaw
Worm, then actual encounter initialization and longer seeded fights. This suite
provides executed combat references alongside the RNG primitive fixtures and
source audits; it does not establish encounter-wide parity.
