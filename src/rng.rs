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
        let s0 = murmur_hash3(if seed == 0 { u64::MIN.wrapping_add(i64::MIN.unsigned_abs()) } else { seed });
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
        self.counter += 1;
        // matches libGDX nextInt(n) where n = range + 1
        let n = (range + 1) as u64;
        (((self.next_long() >> 32) * n) >> 32) as i32
    }

    /// Returns a value in `[lo, hi]` inclusive — matches `Random.random(int start, int end)`.
    pub fn random_range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.random_int(hi - lo)
    }

    pub fn random_bool(&mut self) -> bool {
        self.counter += 1;
        self.next_long() < (1u64 << 63)
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
    /// Derives all streams from a single master seed.
    /// Sub-seeds are offset versions of the master to ensure independence.
    pub fn new(seed: u64) -> Self {
        RngBundle {
            ai:         Rng::new(seed),
            shuffle:    Rng::new(seed.wrapping_add(1)),
            card:       Rng::new(seed.wrapping_add(2)),
            monster_hp: Rng::new(seed.wrapping_add(3)),
            relic:      Rng::new(seed.wrapping_add(4)),
            potion:     Rng::new(seed.wrapping_add(5)),
            misc:       Rng::new(seed.wrapping_add(6)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
