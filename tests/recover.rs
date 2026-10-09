use anti_lcg::presets::PRESETS;
use anti_lcg::{
    Generator, Observation, Options, SeedClass, anti_win_rand, observations_from_tail, recover_seed, recover_seed_range,
};

/// Tiny deterministic PRNG so tests don't depend on `rand`'s API.
struct SplitMix(u64);
impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

fn random_seed(g: &Generator, rng: &mut SplitMix) -> u64 {
    loop {
        let s = g.lcg.m.reduce(rng.next() as u128);
        // MINSTD (c = 0) never leaves state 0.
        if s != 0 || g.lcg.c != 0 {
            return s;
        }
    }
}

fn output_at(g: &Generator, seed: u64, index: u64) -> u64 {
    g.trunc.apply(g.lcg.jump(index as i128).unwrap().apply(seed))
}

fn assert_recovered(g: &Generator, name: &str, seed: u64, obs: &[Observation], classes: &[SeedClass]) {
    assert!(
        classes.iter().any(|c| c.contains(seed)),
        "{name}: seed {seed:#x} not in {} classes for {obs:?}",
        classes.len()
    );
    for c in classes {
        for o in obs {
            assert_eq!(
                output_at(g, c.seed, o.index),
                o.value,
                "{name}: class {c:?} disagrees at {o:?}"
            );
        }
    }
}

#[test]
fn contiguous_tail_every_preset() {
    let mut rng = SplitMix(1);
    for preset in PRESETS {
        let g = preset.generator();
        for n in [1u64, 3, 10, 10_000, 1_000_000_000] {
            let seed = random_seed(&g, &mut rng);
            let len = n.min(6);
            let values: Vec<u64> = (n - len + 1..=n).map(|i| output_at(&g, seed, i)).collect();
            let obs = observations_from_tail(&values, n).unwrap();
            let classes = recover_seed(&g, &obs, &Options::default()).unwrap();
            assert_recovered(&g, preset.name, seed, &obs, &classes);
            if len >= 3 {
                assert!(
                    classes.len() <= 2,
                    "{}: {} classes for n={n}",
                    preset.name,
                    classes.len()
                );
            }
        }
    }
}

#[test]
fn scattered_samples_every_preset() {
    let mut rng = SplitMix(2);
    for preset in PRESETS {
        let g = preset.generator();
        let seed = random_seed(&g, &mut rng);
        let obs: Vec<Observation> = (0..5)
            .map(|_| {
                let index = 1 + rng.next() % 1_000_000;
                Observation {
                    index,
                    value: output_at(&g, seed, index),
                }
            })
            .collect();
        let classes = recover_seed(&g, &obs, &Options::default()).unwrap();
        assert_recovered(&g, preset.name, seed, &obs, &classes);
        assert!(classes.len() <= 2, "{}: {} classes", preset.name, classes.len());
    }
}

#[test]
fn msvc_high_bit_equivalence() {
    // srand(1): 41, 18467, 6334, 26500, 19169
    let seeds = anti_win_rand(&[6334, 26500, 19169], 5).unwrap();
    assert_eq!(seeds, vec![1, 0x8000_0001]);
}

#[test]
fn n_range_and_seed_filter() {
    let g = anti_lcg::presets::find("msvc").unwrap().generator();
    let seed = 1_700_000_000; // looks like a time(NULL) seed
    let values: Vec<u64> = (996..=1000).map(|i| output_at(&g, seed, i)).collect();
    let opts = Options {
        force: false,
        seed_filter: Some(1_600_000_000..=1_800_000_000),
    };
    let results = recover_seed_range(&g, &values, 900..=1100, &opts).unwrap();
    assert!(
        results
            .iter()
            .any(|(n, cs)| *n == 1000 && cs.contains(&SeedClass::single(seed))),
        "{results:?}"
    );
}

#[test]
fn rejects_bad_input() {
    let g = anti_lcg::presets::find("msvc").unwrap().generator();
    assert!(
        recover_seed(
            &g,
            &[Observation {
                index: 1,
                value: 0x8000
            }],
            &Options::default()
        )
        .is_err()
    );
    assert!(observations_from_tail(&[1, 2, 3], 2).is_err());
}
