//! Recover unknown LCG constants from consecutive *full-state* outputs.
//!
//! * `m` unknown: with `t_i = s_{i+1} - s_i`, every `t_{i+2}*t_i - t_{i+1}^2` is a
//!   multiple of `m`, so their gcd is (a small multiple of) `m`.
//! * `a` unknown: solve `a * t_i = t_{i+1} (mod m)`, enumerating all solutions when
//!   `t_i` is not invertible.
//! * `c = s_1 - a * s_0 (mod m)`.
//!
//! Truncated outputs with unknown constants need lattice reduction and are not supported.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{ToPrimitive, Zero};

use crate::Error;
use crate::lcg::Lcg;
use crate::math::{gcd, mod_inverse, mul_mod};

/// Most solutions for `a` that will be enumerated before giving up.
const MAX_A_SOLUTIONS: u128 = 1 << 20;

/// Recover `(a, c, m)` from consecutive states `s_0, s_1, ...` (each the full LCG state).
/// `m` and `a` may be supplied if known. Every returned LCG reproduces all of `states`.
pub fn recover_params(states: &[u64], m: Option<u128>, a: Option<u64>) -> Result<Vec<Lcg>, Error> {
    match m {
        Some(m) => solve_known_m(states, m, a),
        None => {
            if a.is_some() {
                return Err(Error::InvalidParams(
                    "a known multiplier requires a known modulus".into(),
                ));
            }
            recover_unknown_m(states)
        }
    }
}

fn recover_unknown_m(states: &[u64]) -> Result<Vec<Lcg>, Error> {
    if states.len() < 5 {
        return Err(Error::Underdetermined(
            "need at least 5 consecutive states to recover an unknown modulus (more is better)".into(),
        ));
    }
    let t: Vec<BigInt> = states
        .windows(2)
        .map(|w| BigInt::from(w[1]) - BigInt::from(w[0]))
        .collect();
    let g = t
        .windows(3)
        .map(|w| &w[2] * &w[0] - &w[1] * &w[1])
        .fold(BigInt::zero(), |acc, u| acc.gcd(&u));
    if g.is_zero() {
        return Err(Error::Underdetermined(
            "states are degenerate (constant or arithmetic sequence)".into(),
        ));
    }
    let max_state = *states.iter().max().unwrap() as u128;
    let mut m = match g.to_u128() {
        Some(m) if m <= 1u128 << 64 => m,
        _ => {
            return Err(Error::Underdetermined(format!(
                "modulus candidate {g} exceeds 2^64; supply more states"
            )));
        }
    };
    if m <= max_state {
        return Err(Error::Underdetermined(format!(
            "modulus candidate {m} is not larger than every state; the states may not be consecutive"
        )));
    }

    // The gcd can carry extra small factors; strip them while the data stays consistent.
    let mut p = 2u128;
    while p < 1 << 16 && p * p <= m {
        while m % p == 0 && m / p > max_state && !solve_known_m(states, m / p, None)?.is_empty() {
            m /= p;
        }
        p += 1;
    }
    let lcgs = solve_known_m(states, m, None)?;
    if lcgs.is_empty() {
        return Err(Error::Underdetermined(format!(
            "no consistent multiplier for modulus candidate {m}; supply more states"
        )));
    }
    Ok(lcgs)
}

fn solve_known_m(states: &[u64], m: u128, a: Option<u64>) -> Result<Vec<Lcg>, Error> {
    if !(2..=1u128 << 64).contains(&m) {
        return Err(Error::InvalidParams(format!("modulus {m} must be in 2..=2^64")));
    }
    if let Some(&s) = states.iter().find(|&&s| s as u128 >= m) {
        return Err(Error::InvalidObservation(format!(
            "state {s} is not below the modulus {m}"
        )));
    }
    let needed = if a.is_some() { 2 } else { 3 };
    if states.len() < needed {
        return Err(Error::Underdetermined(format!(
            "need at least {needed} consecutive states"
        )));
    }
    let s: Vec<u128> = states.iter().map(|&x| x as u128).collect();
    let t: Vec<u128> = s.windows(2).map(|w| (w[1] + m - w[0]) % m).collect();

    let candidates: Vec<u128> = match a {
        Some(a) => vec![a as u128 % m],
        None => {
            // Pick the difference sharing the fewest factors with m.
            let (i, g) = (0..t.len() - 1)
                .map(|i| (i, gcd(t[i], m)))
                .min_by_key(|&(_, g)| g)
                .unwrap();
            if !t[i + 1].is_multiple_of(g) {
                return Ok(vec![]);
            }
            let reduced_m = m / g;
            let count = g;
            if count > MAX_A_SOLUTIONS {
                return Err(Error::Underdetermined(format!(
                    "{count} multipliers fit the data; supply more states"
                )));
            }
            let a0 = if reduced_m == 1 {
                0
            } else {
                let inv = mod_inverse((t[i] / g) % reduced_m, reduced_m).expect("coprime after dividing gcd");
                mul_mod((t[i + 1] / g) % reduced_m, inv, reduced_m)
            };
            (0..count).map(|j| a0 + j * reduced_m).collect()
        }
    };

    let mut out = Vec::new();
    for a in candidates {
        let c = (s[1] + m - mul_mod(a, s[0], m)) % m;
        if s.windows(2).all(|w| (mul_mod(a, w[0], m) + c) % m == w[1]) {
            out.push(Lcg::new(a, c, m)?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(lcg: &Lcg, seed: u64, n: usize) -> Vec<u64> {
        std::iter::once(seed).chain(lcg.states(seed)).take(n).collect()
    }

    #[test]
    fn unknown_everything() {
        for lcg in [
            Lcg::new(214013, 2531011, 1 << 32).unwrap(),
            Lcg::new(1103515245, 12345, 1 << 31).unwrap(),
            Lcg::new(48271, 0, (1 << 31) - 1).unwrap(),
            Lcg::new(0x5DEECE66D, 11, 1 << 48).unwrap(),
            Lcg::new(123456789, 987654321, 4294967291).unwrap(),
        ] {
            let s = states(&lcg, 0xdead_beef % lcg.m.value() as u64, 12);
            let found = recover_params(&s, None, None).unwrap();
            assert!(found.contains(&lcg), "{lcg:?} not in {found:?}");
        }
    }

    #[test]
    fn known_modulus() {
        let lcg = Lcg::new(214013, 2531011, 1 << 32).unwrap();
        let s = states(&lcg, 42, 4);
        let found = recover_params(&s, Some(1 << 32), None).unwrap();
        assert!(found.contains(&lcg));
        let found = recover_params(&s[..2], Some(1 << 32), Some(214013)).unwrap();
        assert_eq!(found, vec![lcg]);
    }
}
