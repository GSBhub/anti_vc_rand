# anti_lcg (Anti-Windows Rand)
Recover the seed of a linear congruential generator (LCG), such as VC++'s `rand()`, from a few of its outputs. Also recovers the constants of an unknown LCG. The core is written in Rust, with a CLI and Python bindings.

Recovering an MSVC seed from 3 outputs 10,000 calls after seeding takes about 50 µs. The original Python PoC took about 330 ms.

# Usage
## CLI
```sh
cargo install --path .            # or: cargo run --release -- <args>

# MSVC rand(): the last value given was the 10000th call after srand()
anti-lcg seed -n 10000 -r 6334 26500 19169

# Other generators: a preset, or custom constants and output window
anti-lcg presets
anti-lcg seed --preset java -n 1002 -r 1985285816 3004261432 2181189561
anti-lcg seed -a 214013 -c 2531011 -m 2^32 --shift 16 --bits 15 -n 10000 -r ...

# Non-contiguous samples (CALL_NUMBER:VALUE)
anti-lcg seed --preset msvc --sample 17:1234 --sample 900:20000 --sample 2500:77

# Unknown call count, optionally restricted to plausible seeds (e.g. time(NULL))
anti-lcg seed --n-range 9000..11000 --seed-range 1600000000..1800000000 -r ...

# Generate outputs (replaces lib/windows_rand.py)
anti-lcg gen --preset msvc --seed 1 -n 5            # 41 18467 6334 26500 19169

# Recover unknown a, c, m from consecutive full-state outputs
anti-lcg params -s 4029102 3289278137 3144650392 461394683 3164688850 2574545229   # add -m / -a if known
```
Numbers can be written in decimal, `0x...`, or `2^k`.

## Python
```sh
uv sync          # builds the Rust extension with maturin
uv run pytest
uv run anti_rand.py -r 6334 26500 19169 -n 5
```
```python
import anti_lcg
anti_lcg.anti_win_rand([6334, 26500, 19169], 5)             # [1, 2147483649]
anti_lcg.recover_seed(values, n, preset="java")
anti_lcg.recover_seed(samples=[(17, 1234), (900, 20000)])
anti_lcg.recover_seed_classes(values, n)                     # compact (seed, fixed_bits, free_bits)
anti_lcg.recover_seed_range(values, 9000, 11000, seed_range=(lo, hi))
anti_lcg.generate(seed, count, skip=0, states=False, preset="msvc")
anti_lcg.recover_params(states, m=None, a=None)              # [(a, c, m), ...]
```
`lib/windows_rand.py` is kept as a pure-Python MSVC reference that the tests check against.

## Presets
| name | a | c | m | output bits |
|---|---|---|---|---|
| msvc | 214013 | 2531011 | 2^32 | 16..30 |
| borland | 22695477 | 1 | 2^32 | 16..30 |
| ansi-c | 1103515245 | 12345 | 2^32 | 16..30 |
| glibc-type0 | 1103515245 | 12345 | 2^31 | 0..30 |
| java | 0x5DEECE66D | 11 | 2^48 | 16..47 |
| lrand48 / mrand48 | 0x5DEECE66D | 11 | 2^48 | 17..47 / 16..47 |
| minstd0 / minstd | 16807 / 48271 | 0 | 2^31-1 | full |
| numerical-recipes | 1664525 | 1013904223 | 2^32 | full |
| delphi | 134775813 | 1 | 2^32 | full |

* glibc's default `rand()` is not an LCG (TYPE_3 additive feedback). Only TYPE_0 is.
* Java's `new Random(s)` stores `s ^ 0x5DEECE66D`. The recovered value is that internal state, and `presets::java_unscramble` maps it back to `s`.

## Theory
An LCG computes `x = (a * x + c) % m`, starting from `x_0 = seed`. VC++'s `rand()` uses `a=214013`, `c=2531011`, `m=2^32` and returns only bits 16..30 of each state, `(x >> 16) & 0x7FFF`.

### Jumping instead of stepping
One LCG step is the affine map `x -> a*x + c`. Composing it with itself `n` times is still affine, and squaring computes it in O(log n) steps. Running it backwards uses the modular inverse of `a` (extended Euclid). Every observation can therefore be related to any other, and to the seed at index 0, with a single multiply-add, however many calls lie between them.

### Seed search
1. Take one observed output `v`. The full state behind it is `(v << shift) | low` for some unknown `low` below the output window.
2. For each `low`, use the precomputed jump maps to predict every other observation, and drop the candidate on the first mismatch.
3. Map each survivor back to index 0 to get the seed.

The search covers `2^shift` candidates: 2^16 for MSVC and Java, which takes microseconds. Larger spaces run on all cores; 2^32 takes about 1 s on 12 cores. Spaces over 2^40 are refused unless forced (`--force`). For a non-power-of-two modulus, the bits above the window are enumerated too.

### Why there are always two MSVC seeds
With `m = 2^k`, addition and multiplication only carry upward. State bits *above* the output window therefore never affect any output, now or later. For MSVC, bit 31 is never seen, so `s` and `s ^ 0x80000000` produce identical sequences forever. They cannot be told apart, and either one works. Results are reported as seed classes: `seed + k*2^31` for `k < 2`.

### Recovering unknown constants
This needs consecutive full-state outputs `s_i`. Let `t_i = s_{i+1} - s_i`. Every `t_{i+2}*t_i - t_{i+1}^2` is a multiple of `m`, so their gcd gives `m` once you have about 5 or more states. Solving `a*t_i ≡ t_{i+1} (mod m)` gives `a`, and when `t_i` isn't invertible every solution is enumerated. Then `c = s_1 - a*s_0`. Recovering constants from *truncated* outputs would need lattice reduction and is not supported.
