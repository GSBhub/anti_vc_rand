//! Seed recovery for LCGs with known constants and truncated outputs.
//!
//! Convention (same as the original Python): the seed is the state `x_0`, and the
//! k-th call to the generator (k >= 1) returns `out(x_k)`.
//!
//! Instead of stepping 2^17 generators backwards one call at a time, every
//! observation is related to an *anchor* observation by an affine map computed
//! in O(log n). A candidate anchor state is checked with one multiply-add per
//! observation, and survivors are mapped straight to index 0.
//!
//! For a power-of-two modulus, carries only propagate upward, so state bits
//! above the output window never influence any output. Those bits cannot be
//! recovered; only the bits below the window are brute forced, and each result
//! is a [`SeedClass`] covering every seed that produces the same outputs.

use std::ops::RangeInclusive;

use rayon::prelude::*;

use crate::Error;
use crate::lcg::{Affine, Modulus, mask};
use crate::output::Generator;

/// Output `value` was returned by the `index`-th call (1-based) after seeding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub index: u64,
    pub value: u64,
}

/// Contiguous `values` whose last element was the `n`-th output.
pub fn observations_from_tail(values: &[u64], n: u64) -> Result<Vec<Observation>, Error> {
    let len = values.len() as u64;
    if len == 0 {
        return Err(Error::InvalidObservation("no values given".into()));
    }
    if n < len {
        return Err(Error::InvalidObservation(format!(
            "n = {n} is smaller than the number of values ({len})"
        )));
    }
    let first = n - len + 1;
    Ok(values
        .iter()
        .enumerate()
        .map(|(i, &value)| Observation {
            index: first + i as u64,
            value,
        })
        .collect())
}

/// The set `{ seed + j * 2^fixed_bits : 0 <= j < 2^free_bits }`.
///
/// All members produce identical outputs forever. `free_bits == 0` is a single seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeedClass {
    pub seed: u64,
    pub fixed_bits: u32,
    pub free_bits: u32,
}

impl SeedClass {
    pub fn single(seed: u64) -> Self {
        SeedClass {
            seed,
            fixed_bits: 64,
            free_bits: 0,
        }
    }

    pub fn len(&self) -> u128 {
        1u128 << self.free_bits
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn contains(&self, s: u64) -> bool {
        if self.free_bits == 0 {
            return s == self.seed;
        }
        (s & mask(self.fixed_bits)) == self.seed && (s >> self.fixed_bits) < (1u64 << self.free_bits)
    }

    pub fn seeds(&self) -> impl Iterator<Item = u64> + '_ {
        (0..self.len()).map(move |j| self.seed + ((j as u64) << self.fixed_bits.min(63)))
    }

    /// Members inside `range`, as single seeds. Errors if more than `limit` would be produced.
    pub fn restrict(&self, range: &RangeInclusive<u64>, limit: u64) -> Result<Vec<SeedClass>, Error> {
        if self.free_bits == 0 {
            return Ok(if range.contains(&self.seed) {
                vec![*self]
            } else {
                vec![]
            });
        }
        let step = 1u128 << self.fixed_bits;
        let (lo, hi, base) = (*range.start() as u128, *range.end() as u128, self.seed as u128);
        let j_lo = if lo <= base { 0 } else { (lo - base).div_ceil(step) };
        let j_hi = if hi < base {
            return Ok(vec![]);
        } else {
            ((hi - base) / step).min(self.len() - 1)
        };
        if j_lo > j_hi {
            return Ok(vec![]);
        }
        if j_hi - j_lo + 1 > limit as u128 {
            return Err(Error::Underdetermined(format!(
                "seed filter still leaves {} seeds in one class",
                j_hi - j_lo + 1
            )));
        }
        Ok((j_lo..=j_hi)
            .map(|j| SeedClass::single((base + j * step) as u64))
            .collect())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Allow searches larger than 2^40 candidates.
    pub force: bool,
    /// Keep only seeds inside this range (classes are expanded to single seeds).
    pub seed_filter: Option<RangeInclusive<u64>>,
}

/// Searches above this many candidates are refused unless forced.
pub const MAX_SEARCH_LOG2: u32 = 40;
/// Searches below this many candidates run on one thread.
const PARALLEL_THRESHOLD: u128 = 1 << 18;
/// Most seeds a seed filter may expand a single class into.
const FILTER_EXPAND_LIMIT: u64 = 1 << 20;

/// States at the anchor (first) observation's index that are consistent with every observation.
struct AnchorSearch {
    anchor_index: u64,
    states: Vec<u64>,
    /// `Some(h)` when bits `>= h` of the state are unobservable (power-of-two modulus).
    observable_bits: Option<u32>,
}

fn validate(generator: &Generator, obs: &[Observation]) -> Result<(), Error> {
    if obs.is_empty() {
        return Err(Error::InvalidObservation("no observations given".into()));
    }
    if !generator.lcg.is_invertible() {
        return Err(Error::InvalidParams(format!(
            "multiplier {} is not invertible mod {}, the seed cannot be recovered by stepping backwards",
            generator.lcg.a,
            generator.lcg.m.value()
        )));
    }
    let max = generator.trunc.max_value();
    for o in obs {
        if o.index == 0 {
            return Err(Error::InvalidObservation("observation indices start at 1".into()));
        }
        if o.value > max {
            return Err(Error::InvalidObservation(format!(
                "value {} does not fit in {} output bits",
                o.value, generator.trunc.bits
            )));
        }
    }
    Ok(())
}

fn search_anchor(generator: &Generator, obs: &[Observation], opts: &Options) -> Result<AnchorSearch, Error> {
    validate(generator, obs)?;
    let lcg = &generator.lcg;
    let trunc = generator.trunc;
    let anchor = obs[0];

    let mut checks: Vec<(i128, Affine, u64)> = obs[1..]
        .iter()
        .map(|o| {
            let delta = o.index as i128 - anchor.index as i128;
            Ok((delta, lcg.jump(delta)?, o.value))
        })
        .collect::<Result<_, Error>>()?;
    checks.sort_by_key(|(delta, _, _)| delta.unsigned_abs());
    let checks: Vec<(Affine, u64)> = checks.into_iter().map(|(_, f, v)| (f, v)).collect();

    let window = trunc.shift + trunc.bits;
    let base = anchor.value << trunc.shift;
    let low_count = 1u128 << trunc.shift;
    let (high_count, observable_bits) = match lcg.m {
        Modulus::Pow2(k) => (1u128, (window < k).then_some(window)),
        Modulus::Arbitrary(m) => ((((m - 1) as u128) >> window) + 1, None),
    };
    let space = low_count * high_count;
    if space > 1u128 << MAX_SEARCH_LOG2 && !opts.force {
        return Err(Error::SearchTooLarge {
            log2: (space as f64).log2(),
        });
    }

    // `space <= 2^64` because states fit in 64 bits.
    let last = (space - 1) as u64;
    let parallel = space >= PARALLEL_THRESHOLD;
    let shift = trunc.shift;
    let mut states = match lcg.m {
        Modulus::Pow2(_) => {
            // Fast path: mod 2^k is a wrapping u64 multiply-add, and the output mask
            // discards every bit at or above 2^k. Almost every candidate fails the
            // first (nearest) check, so it is inlined without the generic dispatch.
            let out_mask = mask(trunc.bits);
            let (first, rest) = match checks.split_first() {
                Some((&(f, v), rest)) => (Some((f.mul, f.add, v)), rest),
                None => (None, &checks[..]),
            };
            scan(last, parallel, |i| {
                let x = base | i;
                if let Some((mul, add, v)) = first
                    && (mul.wrapping_mul(x).wrapping_add(add) >> shift) & out_mask != v
                {
                    return None;
                }
                rest.iter().all(|(f, v)| trunc.apply(f.apply(x)) == *v).then_some(x)
            })
        }
        Modulus::Arbitrary(m) => {
            let low_mask = mask(shift);
            scan(last, parallel, |i| {
                let x = ((i >> shift) as u128) << window | base as u128 | (i & low_mask) as u128;
                if x >= m as u128 {
                    return None;
                }
                let x = x as u64;
                checks.iter().all(|(f, v)| trunc.apply(f.apply(x)) == *v).then_some(x)
            })
        }
    };
    states.sort_unstable();
    Ok(AnchorSearch {
        anchor_index: anchor.index,
        states,
        observable_bits,
    })
}

/// Collect `candidate(i)` for `i in 0..=last`, across all cores when `parallel`.
fn scan<F>(last: u64, parallel: bool, candidate: F) -> Vec<u64>
where
    F: Fn(u64) -> Option<u64> + Sync + Send,
{
    if parallel {
        // Coarse chunks keep each core in a tight serial loop; survivors are rare.
        const CHUNK_BITS: u32 = 16;
        (0..=last >> CHUNK_BITS)
            .into_par_iter()
            .flat_map_iter(|chunk| {
                let start = chunk << CHUNK_BITS;
                let end = start.saturating_add(mask(CHUNK_BITS)).min(last);
                (start..=end).filter_map(&candidate).collect::<Vec<_>>()
            })
            .collect()
    } else {
        (0..=last).filter_map(candidate).collect()
    }
}

fn to_classes(search: &AnchorSearch, to_seed: &Affine, opts: &Options) -> Result<Vec<SeedClass>, Error> {
    let k = to_seed.m.state_bits();
    let mut classes: Vec<SeedClass> = search
        .states
        .iter()
        .map(|&x| {
            let seed = to_seed.apply(x);
            match search.observable_bits {
                Some(h) => SeedClass {
                    seed: seed & mask(h),
                    fixed_bits: h,
                    free_bits: k - h,
                },
                None => SeedClass::single(seed),
            }
        })
        .collect();
    if let Some(range) = &opts.seed_filter {
        let mut filtered = Vec::new();
        for c in &classes {
            filtered.extend(c.restrict(range, FILTER_EXPAND_LIMIT)?);
        }
        classes = filtered;
    }
    classes.sort_unstable();
    classes.dedup();
    Ok(classes)
}

/// Recover every seed class consistent with `obs`.
pub fn recover_seed(generator: &Generator, obs: &[Observation], opts: &Options) -> Result<Vec<SeedClass>, Error> {
    let search = search_anchor(generator, obs, opts)?;
    let to_seed = generator.lcg.jump(-(search.anchor_index as i128))?;
    to_classes(&search, &to_seed, opts)
}

/// Like [`recover_seed`] for contiguous `values`, trying every `n` (index of the
/// last value) in `n_range`. The brute force runs once; only the final jump back
/// to the seed depends on `n`. Entries with no seeds (after filtering) are omitted.
pub fn recover_seed_range(
    generator: &Generator,
    values: &[u64],
    n_range: RangeInclusive<u64>,
    opts: &Options,
) -> Result<Vec<(u64, Vec<SeedClass>)>, Error> {
    let (lo, hi) = (*n_range.start(), *n_range.end());
    let obs = observations_from_tail(values, lo)?;
    let search = search_anchor(generator, &obs, opts)?;
    let back = generator.lcg.jump(-1)?;
    let mut to_seed = generator.lcg.jump(-(search.anchor_index as i128))?;
    let mut results = Vec::new();
    for n in lo..=hi {
        let classes = to_classes(&search, &to_seed, opts)?;
        if !classes.is_empty() {
            results.push((n, classes));
        }
        to_seed = to_seed.then(&back);
    }
    Ok(results)
}

/// Port of the original Python `anti_win_rand(values, n)`: MSVC `rand()`, contiguous
/// values ending at call `n`, every candidate seed expanded.
pub fn anti_win_rand(values: &[u64], n: u64) -> Result<Vec<u64>, Error> {
    let generator = crate::presets::find("msvc").expect("msvc preset").generator();
    let obs = observations_from_tail(values, n)?;
    let classes = recover_seed(&generator, &obs, &Options::default())?;
    Ok(classes.iter().flat_map(|c| c.seeds().collect::<Vec<_>>()).collect())
}
