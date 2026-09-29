//! Небольшой детерминированный ГПСЧ (SplitMix64) для тасования и генеративных задач.
//! Не криптографический.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn from_time() -> Self {
        Rng(time_seed())
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(GOLDEN);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Число из диапазона `0..n`; `n` должно быть больше нуля.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "Rng::below(0)");
        ((u128::from(self.next_u64()) * u128::from(n)) >> 64) as u64
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }
}

/// Сид из системных часов; два вызова подряд никогда не совпадают.
pub fn time_seed() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ COUNTER.fetch_add(GOLDEN, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn same_seed_same_stream() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        assert!((0..50).all(|_| a.next_u64() == b.next_u64()));
    }

    #[test]
    fn neighbouring_seeds_do_not_collide() {
        // регресс: прежний `seed | 1` склеивал 2k и 2k+1 (в генераторе было 10 уникальных задач из 20)
        let firsts: HashSet<u64> = (0..1000).map(|s| Rng::new(s).next_u64()).collect();
        assert_eq!(firsts.len(), 1000);
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut r = Rng::new(7);
        let seen: HashSet<u64> = (0..2000).map(|_| r.below(10)).collect();
        assert_eq!(seen, (0..10).collect());
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut v: Vec<u32> = (0..100).collect();
        Rng::new(1).shuffle(&mut v);
        assert_ne!(v, (0..100).collect::<Vec<_>>());
        v.sort_unstable();
        assert_eq!(v, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn time_seeds_are_unique() {
        let seeds: HashSet<u64> = (0..1000).map(|_| time_seed()).collect();
        assert_eq!(seeds.len(), 1000);
    }
}
