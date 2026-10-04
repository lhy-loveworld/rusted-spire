/// xorshift128+ matching libGDX RandomXS128, used by the original game.
///
/// Seeding mirrors the Java side: the single master seed is hashed twice with
/// MurmurHash3 to produce the two 64-bit state words.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rng {
    s0: u64,
    s1: u64,
    pub counter: u32,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let s0 = murmur_hash3(if seed == 0 { 1 << 63 } else { seed });
        let s1 = murmur_hash3(s0);
        Rng { s0, s1, counter: 0 }
    }

    pub fn copy(&self) -> Self {
        self.clone()
    }

    fn next_long(&mut self) -> u64 {
        let mut s1 = self.s0;
        let s0 = self.s1;
        self.s0 = s0;
        s1 ^= s1 << 23;
        self.s1 = s1 ^ s0 ^ (s1 >> 17) ^ (s0 >> 26);
        self.s1.wrapping_add(s0)
    }

    /// Returns a value in `[0, range]` inclusive — matches `Random.random(int range)`.
    pub fn random_int(&mut self, range: i32) -> i32 {
        assert!(
            (0..i32::MAX).contains(&range),
            "inclusive range must fit a positive Java int bound"
        );
        self.counter += 1;
        let n = (range + 1) as u64;
        // RandomXS128.nextInt delegates to nextLong(n): use 63-bit modulo
        // and Java signed-overflow rejection, not multiply-and-scale.
        loop {
            let bits = self.next_long() >> 1;
            let value = bits % n;
            if bits.wrapping_sub(value).wrapping_add(n - 1) as i64 >= 0 {
                return value as i32;
            }
        }
    }

    /// Returns a value in `[lo, hi]` inclusive — matches `Random.random(int start, int end)`.
    pub fn random_range(&mut self, lo: i32, hi: i32) -> i32 {
        let range = i64::from(hi) - i64::from(lo);
        assert!(
            (0..i64::from(i32::MAX)).contains(&range),
            "invalid inclusive Java int range"
        );
        lo + self.random_int(range as i32)
    }

    pub fn random_bool(&mut self) -> bool {
        self.counter += 1;
        self.next_long() & 1 != 0
    }

    /// Raw 64-bit result, retaining the bit pattern of a Java signed long.
    pub fn random_long(&mut self) -> u64 {
        self.counter += 1;
        self.next_long()
    }

    /// Returns `true` with probability `chance` — matches `Random.randomBoolean(float chance)`.
    pub fn random_bool_chance(&mut self, chance: f32) -> bool {
        self.counter += 1;
        self.random_float_raw() < chance
    }

    pub fn random_float(&mut self) -> f32 {
        self.counter += 1;
        self.random_float_raw()
    }

    fn random_float_raw(&mut self) -> f32 {
        // matches libGDX: (nextLong() >>> 40) * NORM_FLOAT
        const NORM: f32 = 1.0 / (1u64 << 24) as f32;
        ((self.next_long() >> 40) as f32) * NORM
    }
}

fn murmur_hash3(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

/// All the named RNG streams that AbstractDungeon keeps as separate fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RngBundle {
    pub ai: Rng,
    pub shuffle: Rng,
    pub card: Rng,
    pub monster_hp: Rng,
    pub relic: Rng,
    pub potion: Rng,
    pub misc: Rng,
}

impl RngBundle {
    /// Independent streams starting at the same effective combat seed.
    /// The full game reseeds combat streams with run seed + floor; callers
    /// must supply that effective seed here when reproducing a game combat.
    pub fn new(seed: u64) -> Self {
        RngBundle {
            ai:         Rng::new(seed),
            shuffle:    Rng::new(seed),
            card:       Rng::new(seed),
            monster_hp: Rng::new(seed),
            relic:      Rng::new(seed),
            potion:     Rng::new(seed),
            misc:       Rng::new(seed),
        }
    }
}

/// java.util.Random's 48-bit generator, used only by Collections.shuffle.
struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    fn new(seed: u64) -> Self {
        Self { seed: (seed ^ 0x5deece66d) & ((1 << 48) - 1) }
    }

    fn next(&mut self, bits: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(0x5deece66d).wrapping_add(11) & ((1 << 48) - 1);
        (self.seed >> (48 - bits)) as u32
    }

    fn next_int(&mut self, bound: u32) -> usize {
        assert!(bound > 0 && bound <= i32::MAX as u32);
        if bound.is_power_of_two() {
            return ((u64::from(bound) * u64::from(self.next(31))) >> 31) as usize;
        }
        loop {
            let bits = self.next(31);
            let value = bits % bound;
            if bits.wrapping_sub(value).wrapping_add(bound - 1) as i32 >= 0 {
                return value as usize;
            }
        }
    }
}

/// CardGroup.shuffle consumes one game RNG long, then shuffles with a fresh
/// java.util.Random. Convert Java's top-at-end list to Rust's top-at-zero pile.
pub(crate) fn shuffle_draw_pile<T>(cards: &mut [T], rng: &mut Rng) {
    let mut java_rng = JavaRandom::new(rng.random_long());
    for i in (1..cards.len()).rev() {
        cards.swap(i, java_rng.next_int((i + 1).try_into().expect("deck too large")));
    }
    cards.reverse();
}

#[cfg(test)]
mod tests {
    use super::*;

    const JAVA_FIXTURE: &str = include_str!("../tests/fixtures/java_rng.tsv");

    #[test]
    fn matches_executed_java_primitives_and_rejection() {
        let mut seed = None;
        let mut rng = Rng::new(0);
        let mut cases = 0;
        for line in JAVA_FIXTURE.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 6);
            if fields[1] == "shuffle" {
                continue;
            }
            let next_seed = fields[0].parse::<u64>().unwrap();
            if seed != Some(next_seed) {
                rng = Rng::new(next_seed);
                seed = Some(next_seed);
            }
            let value = match fields[1] {
                "int" => rng.random_int(fields[2].parse().unwrap()).to_string(),
                "range" => rng.random_range(
                    fields[2].parse().unwrap(), fields[3].parse().unwrap()
                ).to_string(),
                "bool" => u8::from(rng.random_bool()).to_string(),
                "chance" => {
                    let chance = f32::from_bits(fields[2].parse().unwrap());
                    u8::from(rng.random_bool_chance(chance)).to_string()
                }
                "float" => rng.random_float().to_bits().to_string(),
                "long" => rng.random_long().to_string(),
                "forced_int" => {
                    rng = Rng {
                        s0: next_seed,
                        s1: fields[2].parse().unwrap(),
                        counter: 0,
                    };
                    rng.random_int(fields[3].parse().unwrap()).to_string()
                }
                op => panic!("unknown Java fixture operation: {op}"),
            };
            assert_eq!(value, fields[4], "Java case: {line}");
            assert_eq!(rng.counter, fields[5].parse::<u32>().unwrap(), "Java case: {line}");
            cases += 1;
        }
        assert_eq!(cases, 1234);
    }

    #[test]
    fn initial_deck_and_discard_draw_order_match_executed_java_shuffle() {
        use crate::card::{Card, CardId};
        use crate::player::PlayerState;
        let mut seed = None;
        let mut initial_rng = Rng::new(0);
        let mut discard_rng = Rng::new(0);
        let mut cases = 0;
        for line in JAVA_FIXTURE.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<_> = line.split('\t').collect();
            if fields[1] != "shuffle" {
                continue;
            }
            let next_seed = fields[0].parse::<u64>().unwrap();
            if seed != Some(next_seed) {
                initial_rng = Rng::new(next_seed);
                discard_rng = Rng::new(next_seed);
                seed = Some(next_seed);
            }
            let size = fields[2].parse::<i32>().unwrap();
            // Unique cost markers identify cards without requiring 100 card IDs.
            let cards: Vec<_> = (0..size).map(|marker| {
                let mut card = Card::new(CardId::Strike);
                card.cost = marker;
                card
            }).collect();
            let expected: Vec<i32> = fields[4].split(',').filter(|s| !s.is_empty())
                .map(|s| s.parse().unwrap()).collect();
            let initial = PlayerState::new(80, 80, 3, cards.clone(), &mut initial_rng);
            assert_eq!(
                initial.draw_pile.iter().map(|c| c.cost).collect::<Vec<_>>(),
                expected,
                "initial: {line}"
            );
            let mut player = PlayerState::new(80, 80, 3, vec![], &mut Rng::new(0));
            player.discard_pile = cards;
            let mut drawn = Vec::new();
            if size == 0 {
                // Empty discard does not trigger a shuffle. Advance the fixture
                // stream explicitly to align the following nonempty cases.
                discard_rng.random_long();
            } else {
                for _ in 0..size {
                    player.draw(1, &mut discard_rng);
                    drawn.push(player.hand.pop().unwrap().cost);
                }
            }
            assert_eq!(drawn, expected, "discard: {line}");
            assert!(player.discard_pile.is_empty());
            let counter = fields[5].parse::<u32>().unwrap();
            assert_eq!(initial_rng.counter, counter);
            assert_eq!(discard_rng.counter, counter);
            cases += 1;
        }
        assert_eq!(cases, 49);
    }

    #[test]
    fn combat_streams_share_seed_but_advance_independently() {
        let mut bundle = RngBundle::new(42);
        let mut reference = Rng::new(42);
        let first = reference.random_long();
        assert_eq!(bundle.ai.random_long(), first);
        assert_eq!(bundle.ai.random_long(), reference.random_long());
        for stream in [
            &mut bundle.shuffle, &mut bundle.card, &mut bundle.monster_hp,
            &mut bundle.relic, &mut bundle.potion, &mut bundle.misc,
        ] {
            assert_eq!(stream.counter, 0);
            assert_eq!(stream.random_long(), first);
        }
    }

    #[test]
    fn rng_deterministic() {
        let mut a = Rng::new(12345);
        let mut b = Rng::new(12345);
        for _ in 0..100 {
            assert_eq!(a.random_int(99), b.random_int(99));
        }
    }

    #[test]
    fn rng_copy_independent() {
        let mut original = Rng::new(42);
        original.random_int(10);
        let mut copy = original.copy();
        // both should produce the same sequence from this point
        for _ in 0..50 {
            assert_eq!(original.random_int(99), copy.random_int(99));
        }
    }

    #[test]
    fn random_int_in_range() {
        let mut rng = Rng::new(0xdeadbeef);
        for _ in 0..1000 {
            let v = rng.random_int(9);
            assert!((0..=9).contains(&v));
        }
    }

    #[test]
    fn random_range_bounds() {
        let mut rng = Rng::new(0xbeefcafe);
        for _ in 0..1000 {
            let v = rng.random_range(5, 15);
            assert!((5..=15).contains(&v));
        }
    }

    #[test]
    fn random_float_unit() {
        let mut rng = Rng::new(1);
        for _ in 0..1000 {
            let v = rng.random_float();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
