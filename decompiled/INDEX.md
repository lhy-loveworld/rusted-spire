# Decompiled Source Index

Root: `decompiled/sources/com/megacrit/cardcrawl/`  
Total files: ~2008 Java classes.

---

## Package Summary

| Package | Files | Purpose |
|---------|------:|---------|
| `cards/` | 447 | All card classes (abstract + per-character) |
| `vfx/` | 302 | Visual effects — irrelevant for headless |
| `actions/` | 280 | Action queue system — key for mechanics |
| `relics/` | 191 | Relic effects |
| `powers/` | 162 | Power (buff/debuff) effects |
| `screens/` | 83 | UI screens — irrelevant for headless |
| `monsters/` | 73 | Monster base classes + all enemies |
| `unlock/` | 66 | Unlock/achievement tracking — irrelevant |
| `events/` | 56 | Story events — irrelevant for combat |
| `helpers/` | 53 | Utility helpers (CardLibrary, EnemyData…) |
| `ui/` | 44 | UI components — irrelevant |
| `potions/` | 44 | Potion effects |
| `daily/` | 38 | Daily challenge modifiers |
| `localization/` | 19 | String bundles — irrelevant |
| `characters/` | 7 | Player character classes |
| `dungeons/` | 5 | Map/dungeon logic (AbstractDungeon) |
| `rooms/` | 13 | Room types (MonsterRoom, RestRoom…) |
| `random/` | 1 | `Random` — xorshift128+ RNG |
| `stances/` | 5 | Watcher stances |
| `orbs/` | 6 | Defect orbs |
| `blights/` | 14 | Blight items (Heartbreaker run) |
| `relics/` | 191 | Relic effects |
| `rewards/` | 7 | Reward screen logic |
| `core/` | 13 | Game loop, settings, AbstractCreature |

---

## Core Mechanics (start here)

### `random/Random.java`
The game's counter/inclusive-bound wrapper around libGDX `RandomXS128`.
**Key methods:** `random(int range)`, `random(int start, int end)`, `randomLong()`, `randomBoolean()`.
The underlying generator is `com/badlogic/gdx/math/RandomXS128.java` (outside the cardcrawl root).
→ Rust equivalent: `src/rng.rs`

### `core/AbstractCreature.java`
Base class for both players and monsters.  
**Key fields:** `currentHealth`, `maxHealth`, `currentBlock`, `powers: ArrayList<AbstractPower>`, `hasPower(String id)`, `getPower(String id)`  
**Key methods:** `addBlock(int)`, `decreaseMaxHealth(int)`, `damage(DamageInfo)`, `applyPowers()`, `applyStartOfTurnPowers()`, `applyEndOfTurnTriggers()`

### `cards/AbstractCard.java`
Base card class.  
**Key fields:** `baseDamage`, `baseBlock`, `cost`, `costForTurn`, `energyOnUse`, `type` (ATTACK/SKILL/POWER/STATUS/CURSE), `rarity`, `upgraded`, `exhaust`, `ethereal`  
**Key methods:** `use(AbstractPlayer p, AbstractMonster m)`, `upgrade()`, `applyPowers()`, `calculateCardDamage(AbstractMonster)`  
**Key inner enums:** `CardType`, `CardRarity`, `CardColor`, `CardTarget`

### `cards/DamageInfo.java`
Damage calculation container.  
**Fields:** `base`, `output` (after power modifiers), `owner`, `type: DamageType` (`NORMAL`, `THORNS`, `HP_LOSS`, `HEAL`)

### `characters/AbstractPlayer.java`
Extends `AbstractCreature`.  
**Key fields:** `hand/drawPile/discardPile/exhaustPile: CardGroup`, `energy: EnergyManager`, `masterDeck: CardGroup`, `relics`, `potions`, `startingDeck`  
**Key methods:** `applyStartOfTurnRelics()`, `applyStartOfTurnPreDrawCards()`, `drawCards(int)`, `discardHand()`

---

## Characters

### `characters/` (7 files)
| File | Description |
|------|-------------|
| `AbstractPlayer.java` | Base player (see above) |
| `Ironclad.java` | Starting deck: Strike×5, Defend×4, Bash×1. Relic: BurningBlood (+6 HP after combat) |
| `TheSilent.java` | Starting deck: Strike×5, Defend×5, Survivor×1, Neutralize×1 |
| `Defect.java` | Starting deck + orb slots |
| `Watcher.java` | Starting deck + stance mechanic |
| `CharacterManager.java` | Registry |
| `AnimatedNpc.java` | NPC characters (Merchant etc.) |

---

## Monsters

### `monsters/AbstractMonster.java`
**Key fields:** `intent: Intent` (ATTACK, ATTACK_BUFF, ATTACK_DEBUFF, ATTACK_DEFEND, BUFF, DEBUFF, DEFEND, DEFEND_DEBUFF, ESCAPE, NONE, SLEEP, STUN, UNKNOWN), `intentDmg`, `intentMultiAmt`, `move` (current move byte), `lastMove`, `lastMoveBefore`  
**Key methods:** `getMove(int num)` (abstract — roll AI), `takeTurn()` (abstract — execute move), `rollMove()`, `makeMove(byte id)`, `setMove(String name, byte id, Intent intent, int dmg, int mult, boolean isMulti)`

### `monsters/EnemyMoveInfo.java`
Struct: `id: byte`, `intent: Intent`, `baseDamage: int`, `multiplier: int`, `isMultiDamage: boolean`

### Exordium Enemies (`monsters/exordium/`)
| Class | HP (Asc0) | Key Moves |
|-------|-----------|-----------|
| `JawWorm.java` | 40–44 | CHOMP(11), BELLOW(Str+block), THRASH(7) |
| `Cultist.java` | 48–54 | INCANTATION(Ritual buff), DARK_STRIKE(6) |
| `LouseNormal.java` | 10–15 | BITE(5–7), GROW(Str) |
| `LouseDefensive.java` | 11–17 | BITE(5–7), SPIT_WEB(Weak) |
| `AcidSlime_L.java` | 65–69 | CORROSIVE_SPIT(11+Slimed), TACKLE(16), LICK(Weak) |
| `AcidSlime_M.java` | 28–32 | CORROSIVE_SPIT(7+Slimed), TACKLE(10), LICK(Weak) |
| `AcidSlime_S.java` | 8–12 | TACKLE(3–5), LICK(Weak) |
| `SpikeSlime_L.java` | 64–70 | FLAME_TACKLE(16+Slimed), LICK(Frail) |
| `SpikeSlime_M.java` | 28–32 | FLAME_TACKLE(8+Slimed), LICK(Frail) |
| `SpikeSlime_S.java` | 10–14 | TACKLE(5–7) |
| `FungiBeast.java` | 22–28 | BITE(6), GROW(Str) |
| `GremlinFat.java` | 13–17 | SMASH(4–5), SUPPORT(Str+3 to ally) |
| `GremlinWarrior.java` | 10–12 | SHARPEN_SWORD(Str), ATTACK(4) |
| `GremlinThief.java` | 10–14 | MAGIC_MISSILE(DMG), SNATCH(gold) |
| `GremlinTsundere.java` | 9–12 | SCRATCH(3) |
| `GremlinWizard.java` | 21–25 | ULTIMATE_BLAST(25 every 4th turn), charging otherwise |
| `GremlinNob.java` | 82–86 | BELLOW(Enrage), SKULL_BASH(6+Vulnerable), RUSH(14) |
| `Looter.java` | 44–48 | MUG(10+gold steal), LUNGE(12), SMOKE_BOMB(Disengage), ESCAPE |
| `SlaverBlue.java` | 46–50 | STAB(12), RAKE(7+Weak) |
| `SlaverRed.java` | 46–50 | STAB(13), SCRAPE(9+Wound), ENTANGLE(Entangled) |
| `Sentry.java` | 38–42 | BEAM(9), BOLT(Dazed) |
| `ApologySlime.java` | 1 | (special encounter) |
| `Lagavulin.java` | 109–111 | SLEEP→ATTACK/DEBUFF |
| `Hexaghost.java` | Elite | ACTIVATE, DIVIDER, INFERNO, SEAR, TACKLE, INFLAME |
| `TheGuardian.java` | Elite boss | MODE_SHIFT mechanic |
| `SlimeBoss.java` | Boss | GOOP_SPRAY, PREPARING, SLAM + split |

### City Enemies (`monsters/city/`)
`BanditBear`, `BanditLeader`, `BanditPointy`, `BookOfStabbing`, `BronzeAutomaton`, `BronzeOrb`, `Byrd`, `Centurion`, `Champ`, `Chosen`, `GremlinLeader`, `Healer`, `Mugger`, `ShelledParasite`, `SnakePlant`, `Snecko`, `SphericGuardian`, `Taskmaster`, `TheCollector`, `TorchHead`

### Beyond Enemies (`monsters/beyond/`)
`AwakenedOne`, `Darkling`, `Deca`, `Donu`, `Exploder`, `GiantHead`, `Maw`, `Nemesis`, `OrbWalker`, `Reptomancer`, `Repulsor`, `SnakeDagger`, `Spiker`, `SpireGrowth`, `TimeEater`, `Transient`, `WrithingMass`

### Ending Bosses (`monsters/ending/`)
`CorruptHeart`, `SpireShield`, `SpireSpear`

---

## Powers

### `powers/AbstractPower.java`
**Key hooks (override to implement effects):**
```
atDamageGive(float dmg, DamageType type) -> float
atDamageFinalGive(float dmg, DamageType type) -> float
atDamageReceive(float dmg, DamageType type) -> float
atDamageFinalReceive(float dmg, DamageType type) -> float
modifyBlock(float block) -> float
atStartOfTurn()
atEndOfTurn(boolean isPlayer)
onPlayCard(AbstractCard card, AbstractMonster m)
onCardDraw(AbstractCard card)
onExhaust(AbstractCard card)
onAttack(DamageInfo info, int damageAmount, AbstractCreature target)
onAttacked(DamageInfo info, int damageAmount)
reducePower(int amt)   // tick-down helper
```
**Key fields:** `ID: String`, `amount: int`, `type: PowerType` (BUFF/DEBUFF)

### Implemented powers (relevant to combat sim)
| Class | ID | Effect |
|-------|----|--------|
| `StrengthPower` | `Strength` | `atDamageGive`: +amount to attack damage |
| `VulnerablePower` | `Vulnerable` | `atDamageReceive`: ×1.5; ticks end-of-turn |
| `WeakPower` | `Weak` | `atDamageGive`: ×0.75; ticks end-of-turn |
| `FrailPower` | `Frail` | `modifyBlock`: ×0.75; ticks end-of-turn |
| `DexterityPower` | `Dexterity` | `modifyBlock`: +amount to block gained |
| `RitualPower` | `Ritual` | `atEndOfTurn`: gain Strength each turn (Cultist) |
| `CurlUpPower` | `Curl Up` | `onAttacked`: gain block once on first hit (LouseNormal) |
| `MetallicizePower` | `Metallicize` | `atEndOfTurn`: gain block |
| `ThornsPower` | `Thorns` | `onAttacked`: deal thorns damage back |
| `SporeCloudPower` | `Spore Cloud` | `onDeath`: apply Vulnerable (FungiBeast) |
| `AngerPower` | `Anger` | `onPlayCard(SKILL)`: lose HP (GremlinNob Enrage) |
| `PlatedArmorPower` | `Plated Armor` | end of turn gain block; reduce on unblocked hit |
| `RupturePower` | `Rupture` | gain Strength when losing HP from a card |
| `EvolvePower` | `Evolve` | draw when receiving Status card |
| `FeelNoPainPower` | `Feel No Pain` | gain block when exhausting |
| `FireBreathingPower` | `Fire Breathing` | deal fire dmg when drawing Status/Curse |
| `DarkEmbracePower` | `Dark Embrace` | draw when exhausting |
| `InflamePower` | *see card* | one-time Strength gain (card → power) |
| `BerserkPower` | `Berserk` | gain energy at start of turn; lose HP each turn |
| `BarricadePower` | `Barricade` | block does not reset between turns |
| `DemonFormPower` | `Demon Form` | gain Strength each turn |
| `LimitBreakPower` | *via card* | double Strength (card only, not a lasting power) |

---

## Cards

### `cards/AbstractCard.java` — base (see Core Mechanics)

### Ironclad (red) cards (`cards/red/`) — 75 cards
**Attacks:** Anger, Bash, Bludgeon, BodySlam, Carnage, Clash, Cleave, Clothesline, Dropkick, FiendFire, FlameBarrier, HeavyBlade, Hemokinesis, Immolate, IronWave, Juggernaut, PerfectedStrike, PommelStrike, Pummel, Rampage, RecklessCharge, Reaper, SearingBlow, SeverSoul, Shockwave, Strike_Red, SwordBoomerang, ThunderClap, TwinStrike, Uppercut, WildStrike  
**Skills:** Armaments, BattleTrance, BloodForBlood, Bloodletting, BurningPact, Corruption, Defend_Red, Disarm, DoubleTap, DualWield, Entrench, Exhume, Feed, FeelNoPain, Flex, GhostlyArmor, Havoc, Headbutt, Impervious, InfernalBlade, Intimidate, Offering, PowerThrough, Rage, SecondWind, SeeingRed, Sentinel, ShrugItOff, SpotWeakness, TrueGrit, Warcry, Whirlwind  
**Powers:** Berserk, Brutality, Combust, DarkEmbrace, Evolve, FireBreathing, Inflame, LimitBreak, Metallicize, Rupture  

**Starter deck:** `Strike_Red`×5, `Defend_Red`×4, `Bash`×1

### Colorless cards (`cards/colorless/`) — ~40 cards
Notable: Apotheosis, Bite, Chrysalis, DramaticEntrance, Enlightenment, HandOfGreed, JAX, Madness, Metamorphosis, PanicButton, RitualDagger, SadisticNature, TheBomb

### Curses (`cards/curses/`) — 14 cards
`AscendersBane`, `Clumsy`, `CurseOfTheBell`, `Decay`, `Doubt`, `Injury`, `Necronomicurse`, `Normality`, `Pain`, `Parasite`, `Pride`, `Regret`, `Shame`, `Writhe`

### Status cards (`cards/status/`) — 5 cards
`Burn`, `Dazed`, `Slimed`, `VoidCard`, `Wound`

---

## Actions (action queue pattern)

The game uses an action queue (`GameActionManager`). Cards enqueue actions; actions execute sequentially during `update()`.  
**Queue order matters even without rendering.** Multi-hit reactions such as
Curl Up enqueue block after the remaining hits; card selections can suspend
resolution. Immediate execution must preserve these ordering boundaries. See
`docs/COMBAT_TRACES.md` for executed original-bytecode comparisons.

### `actions/common/` — bread-and-butter actions
| Class | Effect |
|-------|--------|
| `DamageAction` | Deal DamageInfo to a target |
| `DamageAllEnemiesAction` | AoE damage |
| `GainBlockAction` | Gain block |
| `ApplyPowerAction` | Apply a power to a creature |
| `DrawCardAction` | Draw N cards |
| `ExhaustAction` | Exhaust a card |
| `DiscardAction` | Discard a card |
| `GainEnergyAction` | Gain energy |
| `HealAction` | Heal HP |
| `LoseHPAction` | Lose HP (hp loss type) |
| `RollMoveAction` | Roll enemy's next move |
| `SetMoveAction` | Set enemy move directly |
| `EndTurnAction` | End player turn |
| `MonsterStartTurnAction` | Trigger monster start-of-turn hooks |

### `actions/utility/`
`UseCardAction` (central card use logic — checks costs, calls `card.use()`, triggers hooks), `WaitAction`, `QueueCardAction`, `ScryAction`

---

## Dungeons

### `dungeons/AbstractDungeon.java`
Central game state singleton (in the original game).  
**Key static fields:** `player`, `monsterList`, `actionManager`, `cardRandomRng`, `shuffleRng`, `aiRng`, `monsterHpRng`, `eventRng`, `potionRng`, `miscRng`, `relicRng`  
**Key methods:** `getMonsters()`, `initializeMonster()`, `nextRoomTransition()`, `generateMonsters()`

### `dungeons/Exordium.java`
Act 1. Defines encounter pools (normal/elite/boss), map generation, event pools.  
**Normal pool:** JawWorm, 2× Louse, 3× Slime, 2× Sentry, Cultist, FungiBeast, Gremlin Gang, Looter, 2× Slaver  
**Elite pool:** Lagavulin, 3× Sentries, Hexaghost, TheGuardian  
**Boss pool:** SlimeBoss, Hexaghost, TheGuardian

---

## RNG Streams

From `dungeons/AbstractDungeon.java`:
| Stream | Used for |
|--------|----------|
| `cardRandomRng` | Card rewards, card selection |
| `shuffleRng` | Deck shuffle |
| `aiRng` | Monster AI move selection |
| `monsterHpRng` | Monster HP rolls |
| `eventRng` | Event selection |
| `potionRng` | Potion drops |
| `miscRng` | Miscellaneous |
| `relicRng` | Relic selection |

`generateSeeds()` initializes independent streams with the same run seed.
On floor transitions, combat HP/AI/shuffle/card/misc streams are reset with
run seed + floor number (see `nextRoomTransition()`). There are no per-stream offsets.
→ Rust equivalent: `src/rng.rs` `RngBundle`

---

## Key Files by Lookup Topic

| Topic | File(s) |
|-------|---------|
| RNG implementation | `random/Random.java`; `com/badlogic/gdx/math/RandomXS128.java` outside this root |
| Damage calculation | `core/AbstractCreature.java` (`damage()`), `cards/DamageInfo.java` |
| Power hooks | `powers/AbstractPower.java` |
| Card execution | `cards/AbstractCard.java` (`use()`), `actions/utility/UseCardAction.java` |
| Monster AI pattern | `monsters/AbstractMonster.java` (`getMove()`, `takeTurn()`) |
| JawWorm AI | `monsters/exordium/JawWorm.java` |
| Cultist AI | `monsters/exordium/Cultist.java` |
| Louse AI | `monsters/exordium/LouseNormal.java`, `LouseDefensive.java` |
| Acid Slime AI | `monsters/exordium/AcidSlime_L/M/S.java` |
| Spike Slime AI | `monsters/exordium/SpikeSlime_L/M/S.java` |
| Ironclad cards | `cards/red/Strike_Red.java`, `Defend_Red.java`, `Bash.java`, … |
| Ironclad character | `characters/Ironclad.java` |
| Player mechanics | `characters/AbstractPlayer.java` |
| Energy system | `core/EnergyManager.java` |
| Deck shuffle | `cards/CardGroup.java` (`shuffle()`, `initializeDeck()`); `actions/common/EmptyDeckShuffleAction.java`; `cards/Soul.java` |
| Block reset | `core/AbstractCreature.java` (`loseBlock()` / `applyEndOfTurnTriggers()`) |
| Act 1 encounters | `dungeons/Exordium.java` |
| Gremlin Nob AI | `monsters/exordium/GremlinNob.java` |
| Strength power | `powers/StrengthPower.java` |
| Vulnerable power | `powers/VulnerablePower.java` |
| Weak power | `powers/WeakPower.java` |
| Ritual power (Cultist) | `powers/RitualPower.java` |
| Curl Up power (Louse) | `powers/CurlUpPower.java` |
| Metallicize power | `powers/MetallicizePower.java` |
| Thorns power | `powers/ThornsPower.java` |
| Spore Cloud power | `powers/SporeCloudPower.java` |
