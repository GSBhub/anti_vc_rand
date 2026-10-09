//! Python bindings (`import anti_lcg`). Build with `maturin develop --release`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::{Error, Generator, Observation, Options, SeedClass, presets};

impl From<Error> for PyErr {
    fn from(e: Error) -> PyErr {
        PyValueError::new_err(e.to_string())
    }
}

type GenSpec<'a> = (
    Option<&'a str>,
    Option<u128>,
    Option<u128>,
    Option<u128>,
    Option<u32>,
    Option<u32>,
);

fn generator((preset, a, c, m, shift, bits): GenSpec) -> Result<Generator, Error> {
    presets::resolve(preset, a, c, m, shift, bits)
}

fn options(seed_range: Option<(u64, u64)>, force: bool) -> Options {
    Options {
        force,
        seed_filter: seed_range.map(|(lo, hi)| lo..=hi),
    }
}

fn expand(classes: &[SeedClass]) -> Vec<u64> {
    classes.iter().flat_map(|c| c.seeds().collect::<Vec<_>>()).collect()
}

fn observations(
    values: Option<Vec<u64>>,
    n: Option<u64>,
    samples: Option<Vec<(u64, u64)>>,
) -> Result<Vec<Observation>, Error> {
    match (values, samples) {
        (Some(values), None) => {
            let n = n.unwrap_or(values.len() as u64);
            crate::observations_from_tail(&values, n)
        }
        (None, Some(samples)) => Ok(samples
            .into_iter()
            .map(|(index, value)| Observation { index, value })
            .collect()),
        _ => Err(Error::InvalidObservation(
            "pass exactly one of `values` (with `n`) or `samples`".into(),
        )),
    }
}

#[pymodule]
mod anti_lcg {
    use super::*;

    /// Recover every seed consistent with the observed outputs.
    ///
    /// Either `values` (consecutive outputs, the last one produced by call `n`, 1-based)
    /// or `samples` (list of `(index, value)` pairs). The generator is a `preset` name
    /// (default "msvc") or custom `a`, `c`, `m` with optional `shift`/`bits`.
    /// Returns every seed; seeds differing only in unobservable high bits are all listed.
    #[pyfunction]
    #[pyo3(signature = (values=None, n=None, *, samples=None, preset=None, a=None, c=None, m=None, shift=None, bits=None, seed_range=None, force=false))]
    #[allow(clippy::too_many_arguments)]
    fn recover_seed(
        py: Python<'_>,
        values: Option<Vec<u64>>,
        n: Option<u64>,
        samples: Option<Vec<(u64, u64)>>,
        preset: Option<&str>,
        a: Option<u128>,
        c: Option<u128>,
        m: Option<u128>,
        shift: Option<u32>,
        bits: Option<u32>,
        seed_range: Option<(u64, u64)>,
        force: bool,
    ) -> PyResult<Vec<u64>> {
        let classes = recover_seed_classes(py, values, n, samples, preset, a, c, m, shift, bits, seed_range, force)?;
        Ok(classes
            .into_iter()
            .flat_map(|(seed, fixed_bits, free_bits)| {
                SeedClass {
                    seed,
                    fixed_bits,
                    free_bits,
                }
                .seeds()
                .collect::<Vec<_>>()
            })
            .collect())
    }

    /// Like `recover_seed`, but returns compact `(seed, fixed_bits, free_bits)` classes:
    /// every `seed + k * 2**fixed_bits` for `k < 2**free_bits` is equivalent.
    #[pyfunction]
    #[pyo3(signature = (values=None, n=None, *, samples=None, preset=None, a=None, c=None, m=None, shift=None, bits=None, seed_range=None, force=false))]
    #[allow(clippy::too_many_arguments)]
    fn recover_seed_classes(
        py: Python<'_>,
        values: Option<Vec<u64>>,
        n: Option<u64>,
        samples: Option<Vec<(u64, u64)>>,
        preset: Option<&str>,
        a: Option<u128>,
        c: Option<u128>,
        m: Option<u128>,
        shift: Option<u32>,
        bits: Option<u32>,
        seed_range: Option<(u64, u64)>,
        force: bool,
    ) -> PyResult<Vec<(u64, u32, u32)>> {
        let g = generator((preset, a, c, m, shift, bits))?;
        let obs = observations(values, n, samples)?;
        let opts = options(seed_range, force);
        let classes = py.detach(|| crate::recover_seed(&g, &obs, &opts))?;
        Ok(classes.iter().map(|c| (c.seed, c.fixed_bits, c.free_bits)).collect())
    }

    /// Try every `n` in `n_lo..=n_hi` for consecutive `values`. Returns `[(n, [seeds])]`.
    #[pyfunction]
    #[pyo3(signature = (values, n_lo, n_hi, *, preset=None, a=None, c=None, m=None, shift=None, bits=None, seed_range=None, force=false))]
    #[allow(clippy::too_many_arguments)]
    fn recover_seed_range(
        py: Python<'_>,
        values: Vec<u64>,
        n_lo: u64,
        n_hi: u64,
        preset: Option<&str>,
        a: Option<u128>,
        c: Option<u128>,
        m: Option<u128>,
        shift: Option<u32>,
        bits: Option<u32>,
        seed_range: Option<(u64, u64)>,
        force: bool,
    ) -> PyResult<Vec<(u64, Vec<u64>)>> {
        let g = generator((preset, a, c, m, shift, bits))?;
        let opts = options(seed_range, force);
        let results = py.detach(|| crate::recover_seed_range(&g, &values, n_lo..=n_hi, &opts))?;
        Ok(results.into_iter().map(|(n, classes)| (n, expand(&classes))).collect())
    }

    /// Drop-in replacement for the original Python `anti_win_rand(values, n)` (MSVC rand()).
    #[pyfunction]
    fn anti_win_rand(py: Python<'_>, values: Vec<u64>, n: u64) -> PyResult<Vec<u64>> {
        Ok(py.detach(|| crate::anti_win_rand(&values, n))?)
    }

    /// Outputs of calls `skip+1 ..= skip+count` after seeding with `seed`
    /// (full states instead of truncated outputs if `states=True`).
    #[pyfunction]
    #[pyo3(signature = (seed, count=1, *, skip=0, states=false, preset=None, a=None, c=None, m=None, shift=None, bits=None))]
    #[allow(clippy::too_many_arguments)]
    fn generate(
        seed: u128,
        count: usize,
        skip: u64,
        states: bool,
        preset: Option<&str>,
        a: Option<u128>,
        c: Option<u128>,
        m: Option<u128>,
        shift: Option<u32>,
        bits: Option<u32>,
    ) -> PyResult<Vec<u64>> {
        let g = generator((preset, a, c, m, shift, bits))?;
        let start = g.lcg.jump(skip as i128)?.apply(g.lcg.m.reduce(seed));
        Ok(g.lcg
            .states(start)
            .take(count)
            .map(|x| if states { x } else { g.trunc.apply(x) })
            .collect())
    }

    /// Recover `(a, c, m)` candidates from consecutive full-state outputs.
    #[pyfunction]
    #[pyo3(signature = (states, m=None, a=None))]
    fn recover_params(states: Vec<u64>, m: Option<u128>, a: Option<u64>) -> PyResult<Vec<(u64, u64, u128)>> {
        let lcgs = crate::recover_params(&states, m, a)?;
        Ok(lcgs.iter().map(|l| (l.a, l.c, l.m.value())).collect())
    }

    /// Built-in presets as `(name, a, c, m, shift, bits, description)`.
    #[pyfunction]
    fn presets() -> Vec<(&'static str, u64, u64, u128, u32, u32, &'static str)> {
        crate::presets::PRESETS
            .iter()
            .map(|p| (p.name, p.a, p.c, p.m, p.shift, p.bits, p.description))
            .collect()
    }
}
