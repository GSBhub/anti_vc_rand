//! How a generator turns its internal state into the value the caller sees.

use crate::Error;
use crate::lcg::{Lcg, mask};

/// `output = (state >> shift) & (2^bits - 1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Truncation {
    pub shift: u32,
    pub bits: u32,
}

impl Truncation {
    /// The whole state is the output.
    pub fn full(lcg: &Lcg) -> Self {
        Truncation {
            shift: 0,
            bits: lcg.m.state_bits(),
        }
    }

    #[inline(always)]
    pub fn apply(&self, state: u64) -> u64 {
        (state >> self.shift) & mask(self.bits)
    }

    pub fn max_value(&self) -> u64 {
        mask(self.bits)
    }
}

/// An LCG plus its output truncation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Generator {
    pub lcg: Lcg,
    pub trunc: Truncation,
}

impl Generator {
    pub fn new(lcg: Lcg, trunc: Truncation) -> Result<Self, Error> {
        let state_bits = lcg.m.state_bits();
        if trunc.bits == 0 || trunc.shift + trunc.bits > state_bits {
            return Err(Error::InvalidParams(format!(
                "output window shift={} bits={} does not fit in a {state_bits}-bit state",
                trunc.shift, trunc.bits
            )));
        }
        Ok(Generator { lcg, trunc })
    }

    /// Iterator over the outputs `out(x_1), out(x_2), ...` for `seed = x_0`.
    pub fn outputs(&self, seed: u64) -> impl Iterator<Item = u64> + '_ {
        self.lcg.states(seed).map(|x| self.trunc.apply(x))
    }
}
