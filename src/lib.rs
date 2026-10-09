//! Recover seeds of linear congruential generators from their (possibly truncated)
//! outputs, and recover unknown LCG constants from full-state outputs.

pub mod lcg;
pub mod math;
pub mod output;
pub mod params;
pub mod presets;
#[cfg(feature = "python")]
mod python;
pub mod seed;

pub use lcg::{Affine, Lcg, Modulus};
pub use output::{Generator, Truncation};
pub use params::recover_params;
pub use seed::{
    Observation, Options, SeedClass, anti_win_rand, observations_from_tail, recover_seed, recover_seed_range,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidParams(String),
    InvalidObservation(String),
    SearchTooLarge { log2: f64 },
    Underdetermined(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidParams(s) => write!(f, "invalid parameters: {s}"),
            Error::InvalidObservation(s) => write!(f, "invalid observation: {s}"),
            Error::SearchTooLarge { log2 } => write!(
                f,
                "search space is 2^{log2:.1} candidates (limit 2^{}); pass force to run anyway",
                seed::MAX_SEARCH_LOG2
            ),
            Error::Underdetermined(s) => write!(f, "underdetermined: {s}"),
        }
    }
}

impl std::error::Error for Error {}
