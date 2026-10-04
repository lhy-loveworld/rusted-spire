# Constructor, Lagavulin and Guardian audit

This is a static comparison with local decompiled Java, followed by Rust
regression tests. It does not execute the original combat engine. The separate
[RNG fixtures](RNG_VALIDATION.md) execute only the Java RNG and JDK shuffle.
Paths below are relative to `decompiled/sources/com/megacrit/cardcrawl/`.

## Constructor and pre-battle RNG

`monsters/AbstractMonster.java:setHp(int)` delegates to the two-bound overload,
so **fixed HP consumes a random call**. The base constructor instead assigns HP
directly. Explicit-HP slime children use the base constructor, without an HP
roll; normal randomly sized slimes call `setHp` afterward.

`LouseNormal.java` and `LouseDefensive.java` roll HP then Bite damage from
`monsterHpRng` in their constructors, and Curl Up from the same stream in
`usePreBattleAction`. `MonsterGroup.init()` rolls all initial moves, followed by
`MonsterGroup.usePreBattleAction()` across the formation. Rust previously
completed these phases for each monster before constructing the next. It now
constructs every monster, rolls every move, then runs every pre-battle hook.
Two-Louse tests verify the resulting HP, damage, Curl Up and RNG continuation.

HP ranges were checked for the 21 supported IDs against their constructor
`setHp` calls. Corrected discrepancies: Lagavulin 109–111 below A8; Guardian
240 below A9 and 250 thereafter; Gremlin Wizard 21–25 below A7 and 22–26 thereafter.
Louse damage/Curl Up already used the correct RNG stream. Fixed boss HP and
explicit-HP split children retain their respective one/zero HP-roll behavior.

## Lagavulin

`monsters/exordium/Lagavulin.java` governs the sleeping elite variant:

- Starts with 8 block and Metallicize 8 at every ascension. Three idle turns
  precede its first attack; the third idle removes Metallicize and sets the
  next move directly. Initial move and first two idle turns consume AI rolls.
- Actual HP loss wakes it, removes Metallicize, and replaces its next action
  with one stunned turn. Fully blocked damage does not wake it.
- After waking: attack, attack, Siphon, repeat beyond the bounded move history.
  Attack is 18, or 20 at A3. Siphon applies Dexterity then Strength, -1 each,
  or -2 at A18. These losses persist; Artifact blocks applications individually.
- The initially awake event variant is not exposed by this environment.

`powers/DexterityPower.java` supports negative Dexterity and adds it before
Frail for card block. Power-generated Metallicize block ignores both modifiers.

## Guardian

`monsters/exordium/TheGuardian.java` supplies the state machine:

- Offensive cycle: Charge Up (9 block), Fierce Bash (32, or 36 at A4), Vent
  Steam (Weak 2 and Vulnerable 2), Whirlwind (5 × 4), then Charge Up again.
- Mode Shift counts actual HP lost, starting at 30, 35 at A9, or 40 at A19.
  Crossing it queues 20 block and interrupts the next move with Close Up.
  The next threshold increases by 10. A lethal hit does not shift.
- The queued shift resolves after the current card's hits, not between Twin
  Strike hits. Close Up applies Sharp Hide (3, or 4 at A19). Roll Attack is
  9, or 10 at A4; Twin Slam is 8 × 2 and restores offensive Mode Shift,
  removes Sharp Hide, and schedules Whirlwind. These transitions consume no
  additional AI rolls after initialization.

`powers/SharpHidePower.java:onUseCard` queues one THORNS hit per attack card,
including attacks targeting another monster. `AbstractPlayer.useCard` queues
card effects before `UseCardAction` invokes that hook. The damage therefore
resolves after card effects, including Iron Wave's block, and uses the powers
present when the card was played. It is not one hit per damage instance.
`DamageAction` exempts THORNS from owner-death cancellation, and
`GameActionManager.clearPostCombatActions` preserves damage actions. Tests
cover lethal retaliation, including player defeat when both combatants die.

## Validation and limits

`tests/elite_boss_semantics.rs` covers constructor ordering, fixed/explicit HP,
ascension boundaries, repeated cycles, blocked waking, Artifact, Dexterity/Frail,
multi-hit shifting, threshold reset, Sharp Hide timing/targeting, and observations.
Interface v4 is 196 observations / 61 actions; v3 policies must be retrained.

Enemy mechanics were tested by source-derived expectations, not original-game
traces. Remaining work includes other enemy AI branches, card choice behavior,
card/discard ordering, Curl Up's queued timing, player Ritual, and unsupported
relic/potion reactions. The immediate-effect architecture will need additional
ordering work as those interactions are added. Original JAR build/checksum and
live combat traces remain unavailable.
