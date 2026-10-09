//! Well-known LCG-based generators.
//!
//! Notes:
//! * glibc's default `rand()` is an additive-feedback generator (TYPE_3), not an LCG.
//!   Only `srandom`/`initstate` with an 8-byte state buffer (TYPE_0) uses the LCG below.
//! * Java's `new Random(s)` stores `s ^ 0x5DEECE66D` as its state; see [`java_unscramble`].
//! * MINSTD generators output the whole state (`1..m-1`).

use crate::Error;
use crate::lcg::Lcg;
use crate::output::{Generator, Truncation};

pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub a: u64,
    pub c: u64,
    pub m: u128,
    pub shift: u32,
    pub bits: u32,
}

impl Preset {
    pub fn generator(&self) -> Generator {
        let lcg = Lcg::new(self.a as u128, self.c as u128, self.m).expect("valid preset");
        Generator::new(
            lcg,
            Truncation {
                shift: self.shift,
                bits: self.bits,
            },
        )
        .expect("valid preset")
    }
}

#[rustfmt::skip]
pub const PRESETS: &[Preset] = &[
    Preset { name: "msvc", description: "Microsoft Visual C++ rand()", a: 214013, c: 2531011, m: 1 << 32, shift: 16, bits: 15 },
    Preset { name: "borland", description: "Borland C/C++ rand()", a: 22695477, c: 1, m: 1 << 32, shift: 16, bits: 15 },
    Preset { name: "ansi-c", description: "ANSI C reference rand()", a: 1103515245, c: 12345, m: 1 << 32, shift: 16, bits: 15 },
    Preset { name: "glibc-type0", description: "glibc random() with TYPE_0 state", a: 1103515245, c: 12345, m: 1 << 31, shift: 0, bits: 31 },
    Preset { name: "java", description: "java.util.Random.nextInt() (as unsigned 32-bit)", a: 0x5DEECE66D, c: 11, m: 1 << 48, shift: 16, bits: 32 },
    Preset { name: "lrand48", description: "POSIX lrand48()", a: 0x5DEECE66D, c: 11, m: 1 << 48, shift: 17, bits: 31 },
    Preset { name: "mrand48", description: "POSIX mrand48() (as unsigned 32-bit)", a: 0x5DEECE66D, c: 11, m: 1 << 48, shift: 16, bits: 32 },
    Preset { name: "minstd0", description: "Park-Miller MINSTD (C++ minstd_rand0)", a: 16807, c: 0, m: (1 << 31) - 1, shift: 0, bits: 31 },
    Preset { name: "minstd", description: "Park-Miller revised (C++ minstd_rand)", a: 48271, c: 0, m: (1 << 31) - 1, shift: 0, bits: 31 },
    Preset { name: "numerical-recipes", description: "Numerical Recipes ranqd1", a: 1664525, c: 1013904223, m: 1 << 32, shift: 0, bits: 32 },
    Preset { name: "delphi", description: "Borland Delphi / Turbo Pascal RandSeed", a: 134775813, c: 1, m: 1 << 32, shift: 0, bits: 32 },
];

pub fn find(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

/// Build a generator from a preset name, or from custom `a`/`c`/`m` (all three
/// required, full-state output by default). `shift`/`bits` override the output window.
/// With nothing given, the `msvc` preset is used.
pub fn resolve(
    preset: Option<&str>,
    a: Option<u128>,
    c: Option<u128>,
    m: Option<u128>,
    shift: Option<u32>,
    bits: Option<u32>,
) -> Result<Generator, Error> {
    let base = match (preset, a, c, m) {
        (None, Some(a), Some(c), Some(m)) => {
            let lcg = Lcg::new(a, c, m)?;
            Generator {
                lcg,
                trunc: Truncation::full(&lcg),
            }
        }
        (name, None, None, None) => {
            let name = name.unwrap_or("msvc");
            find(name)
                .ok_or_else(|| Error::InvalidParams(format!("unknown preset {name:?}")))?
                .generator()
        }
        (Some(_), ..) => return Err(Error::InvalidParams("give either a preset or a/c/m, not both".into())),
        _ => return Err(Error::InvalidParams("a custom LCG needs all of a, c and m".into())),
    };
    let trunc = Truncation {
        shift: shift.unwrap_or(base.trunc.shift),
        bits: bits.unwrap_or(base.trunc.bits),
    };
    Generator::new(base.lcg, trunc)
}

/// Java's `Random(seed)` constructor scrambles the seed; this maps an internal
/// state back to the value passed to the constructor (low 48 bits).
pub fn java_unscramble(state: u64) -> u64 {
    (state ^ 0x5DEECE66D) & ((1 << 48) - 1)
}

/// The internal state Java's `Random(seed)` starts from.
pub fn java_scramble(seed: u64) -> u64 {
    java_unscramble(seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msvc_srand_1() {
        let g = find("msvc").unwrap().generator();
        let out: Vec<u64> = g.outputs(1).take(5).collect();
        assert_eq!(out, [41, 18467, 6334, 26500, 19169]);
    }

    #[test]
    fn java_random_42() {
        let g = find("java").unwrap().generator();
        let first = g.outputs(java_scramble(42)).next().unwrap();
        assert_eq!(first as u32 as i32, -1170105035);
    }

    #[test]
    fn all_presets_construct() {
        for p in PRESETS {
            let _ = p.generator();
        }
    }
}
