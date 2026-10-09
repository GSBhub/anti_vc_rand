//! LCG parameters and affine-map arithmetic (jump ahead / jump back in O(log n)).

use crate::Error;
use crate::math::{mod_inverse, mul_mod};

/// Modulus of an LCG. States are always `< m`, and `m <= 2^64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modulus {
    /// `m = 2^k`, `1 <= k <= 64`. Reduction is a mask.
    Pow2(u32),
    /// Any other modulus `m >= 2`.
    Arbitrary(u64),
}

impl Modulus {
    /// Build from a raw value (`m <= 2^64`), choosing `Pow2` when possible.
    pub fn new(m: u128) -> Result<Self, Error> {
        if !(2..=1u128 << 64).contains(&m) {
            return Err(Error::InvalidParams(format!("modulus {m} must be in 2..=2^64")));
        }
        if m.is_power_of_two() {
            Ok(Modulus::Pow2(m.trailing_zeros()))
        } else {
            Ok(Modulus::Arbitrary(m as u64))
        }
    }

    pub fn value(self) -> u128 {
        match self {
            Modulus::Pow2(k) => 1u128 << k,
            Modulus::Arbitrary(m) => m as u128,
        }
    }

    /// Number of bits needed to hold any state.
    pub fn state_bits(self) -> u32 {
        match self {
            Modulus::Pow2(k) => k,
            Modulus::Arbitrary(m) => 64 - (m - 1).leading_zeros(),
        }
    }

    #[inline(always)]
    pub fn reduce(self, x: u128) -> u64 {
        match self {
            Modulus::Pow2(k) => (x as u64) & mask(k),
            Modulus::Arbitrary(m) => (x % m as u128) as u64,
        }
    }

    #[inline(always)]
    pub fn mul_add(self, a: u64, x: u64, c: u64) -> u64 {
        match self {
            Modulus::Pow2(k) => a.wrapping_mul(x).wrapping_add(c) & mask(k),
            Modulus::Arbitrary(m) => ((a as u128 * x as u128 + c as u128) % m as u128) as u64,
        }
    }

    pub fn neg(self, x: u64) -> u64 {
        self.reduce(self.value() - self.reduce(x as u128) as u128)
    }

    pub fn inverse(self, x: u64) -> Option<u64> {
        mod_inverse(x as u128, self.value()).map(|v| v as u64)
    }
}

/// Low `k` bits set (`k <= 64`).
#[inline(always)]
pub const fn mask(k: u32) -> u64 {
    if k >= 64 { u64::MAX } else { (1u64 << k) - 1 }
}

/// The map `x -> mul*x + add (mod m)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Affine {
    pub mul: u64,
    pub add: u64,
    pub m: Modulus,
}

impl Affine {
    pub fn identity(m: Modulus) -> Self {
        Affine {
            mul: m.reduce(1),
            add: 0,
            m,
        }
    }

    #[inline(always)]
    pub fn apply(&self, x: u64) -> u64 {
        self.m.mul_add(self.mul, x, self.add)
    }

    /// `self` followed by `next`, i.e. `next(self(x))`.
    pub fn then(&self, next: &Affine) -> Affine {
        let m = self.m.value();
        Affine {
            mul: mul_mod(next.mul as u128, self.mul as u128, m) as u64,
            add: next.apply(self.add),
            m: self.m,
        }
    }

    /// `self` applied `n` times, by repeated squaring.
    pub fn pow(&self, mut n: u64) -> Affine {
        let mut result = Affine::identity(self.m);
        let mut base = *self;
        while n > 0 {
            if n & 1 == 1 {
                result = result.then(&base);
            }
            base = base.then(&base);
            n >>= 1;
        }
        result
    }

    /// The inverse map, if `mul` is invertible mod `m`.
    pub fn inverse(&self) -> Option<Affine> {
        let inv = self.m.inverse(self.mul)?;
        // x = inv*(y - add) = inv*y - inv*add
        let add = self
            .m
            .neg(mul_mod(inv as u128, self.add as u128, self.m.value()) as u64);
        Some(Affine {
            mul: inv,
            add,
            m: self.m,
        })
    }
}

/// `x_{n+1} = (a * x_n + c) mod m`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Lcg {
    pub a: u64,
    pub c: u64,
    pub m: Modulus,
}

impl Lcg {
    pub fn new(a: u128, c: u128, m: u128) -> Result<Self, Error> {
        let m = Modulus::new(m)?;
        Ok(Lcg {
            a: m.reduce(a),
            c: m.reduce(c),
            m,
        })
    }

    #[inline(always)]
    pub fn next(&self, x: u64) -> u64 {
        self.m.mul_add(self.a, x, self.c)
    }

    pub fn step(&self) -> Affine {
        Affine {
            mul: self.a,
            add: self.c,
            m: self.m,
        }
    }

    pub fn is_invertible(&self) -> bool {
        self.m.inverse(self.a).is_some()
    }

    /// Map taking the state at index `i` to the state at index `i + delta`.
    /// Negative `delta` needs `a` to be invertible.
    pub fn jump(&self, delta: i128) -> Result<Affine, Error> {
        if delta >= 0 {
            Ok(self.step().pow(delta as u64))
        } else {
            let back = self.step().inverse().ok_or_else(|| {
                Error::InvalidParams(format!(
                    "multiplier {} is not invertible mod {}, cannot step backwards",
                    self.a,
                    self.m.value()
                ))
            })?;
            Ok(back.pow(delta.unsigned_abs() as u64))
        }
    }

    /// Iterator over `x_1, x_2, ...` starting from `seed = x_0`.
    pub fn states(&self, seed: u64) -> impl Iterator<Item = u64> + '_ {
        let mut x = self.m.reduce(seed as u128);
        std::iter::repeat_with(move || {
            x = self.next(x);
            x
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_matches_iteration() {
        for lcg in [
            Lcg::new(214013, 2531011, 1 << 32).unwrap(),
            Lcg::new(0x5DEECE66D, 11, 1 << 48).unwrap(),
            Lcg::new(16807, 0, (1 << 31) - 1).unwrap(),
            Lcg::new(6364136223846793005, 1442695040888963407, 1 << 64).unwrap(),
        ] {
            let seed = lcg.m.reduce(0x1234_5678_9abc);
            let states: Vec<u64> = lcg.states(seed).take(1000).collect();
            for n in [1usize, 2, 7, 500, 1000] {
                assert_eq!(lcg.jump(n as i128).unwrap().apply(seed), states[n - 1]);
                assert_eq!(lcg.jump(-(n as i128)).unwrap().apply(states[n - 1]), seed);
            }
            let far = lcg.jump(1_000_000_007).unwrap().apply(seed);
            assert_eq!(lcg.jump(-1_000_000_007).unwrap().apply(far), seed);
        }
    }
}
