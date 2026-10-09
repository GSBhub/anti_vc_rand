use anti_lcg::{Generator, Lcg, Options, Truncation, observations_from_tail, presets, recover_seed};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn tail(g: &Generator, seed: u64, n: u64, len: u64) -> Vec<u64> {
    let start = g.lcg.jump((n - len) as i128).unwrap().apply(seed);
    g.outputs(start).take(len as usize).collect()
}

fn bench(c: &mut Criterion) {
    for (name, n) in [("msvc", 10_000u64), ("java", 10_000)] {
        let g = presets::find(name).unwrap().generator();
        let obs = observations_from_tail(&tail(&g, 0xC0FFEE, n, 3), n).unwrap();
        c.bench_function(&format!("{name} n={n} 3 values (2^16 search)"), |b| {
            b.iter(|| recover_seed(black_box(&g), black_box(&obs), &Options::default()).unwrap())
        });
    }
    // 64-bit state, top 32 bits shown: a 2^32 search across all cores.
    let lcg = Lcg::new(6364136223846793005, 1442695040888963407, 1 << 64).unwrap();
    let g = Generator::new(lcg, Truncation { shift: 26, bits: 32 }).unwrap();
    let obs = observations_from_tail(&tail(&g, 0xC0FFEE, 100, 4), 100).unwrap();
    let mut group = c.benchmark_group("large");
    group.sample_size(10);
    group.bench_function("64-bit state, shift 26 (2^26 search)", |b| {
        b.iter(|| recover_seed(black_box(&g), black_box(&obs), &Options::default()).unwrap())
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
