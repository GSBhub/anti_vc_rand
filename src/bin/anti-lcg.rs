use std::ops::RangeInclusive;
use std::process::ExitCode;

use anti_lcg::presets::{self, PRESETS};
use anti_lcg::{Generator, Observation, Options, SeedClass};
use clap::{Args, Parser, Subcommand};

/// Recover seeds and constants of linear congruential generators.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Recover the seed from observed outputs.
    Seed(SeedArgs),
    /// Generate outputs from a seed.
    Gen(GenArgs),
    /// Recover unknown a, c (and m) from consecutive full-state outputs.
    Params(ParamsArgs),
    /// List built-in generator presets.
    Presets,
}

#[derive(Args)]
struct GeneratorArgs {
    /// Built-in generator (see `presets`). Defaults to msvc unless -a/-c/-m are given.
    #[arg(short, long, conflicts_with_all = ["a", "c", "m"])]
    preset: Option<String>,
    /// Multiplier.
    #[arg(short, value_parser = parse_num, requires_all = ["c", "m"])]
    a: Option<u128>,
    /// Increment.
    #[arg(short, value_parser = parse_num, requires_all = ["a", "m"])]
    c: Option<u128>,
    /// Modulus (e.g. 2^32, 0x7fffffff).
    #[arg(short, value_parser = parse_num, requires_all = ["a", "c"])]
    m: Option<u128>,
    /// Output = (state >> shift) & (2^bits - 1). Overrides the preset.
    #[arg(long)]
    shift: Option<u32>,
    /// Number of output bits. Overrides the preset.
    #[arg(long)]
    bits: Option<u32>,
}

impl GeneratorArgs {
    fn build(&self) -> Result<Generator, String> {
        presets::resolve(self.preset.as_deref(), self.a, self.c, self.m, self.shift, self.bits)
            .map_err(|e| e.to_string())
    }
}

#[derive(Args)]
struct SeedArgs {
    #[command(flatten)]
    generator: GeneratorArgs,
    /// Consecutive output values, oldest first.
    #[arg(short = 'r', long = "random-values", num_args = 1.., value_parser = parse_u64, conflicts_with = "sample")]
    values: Vec<u64>,
    /// Call number (1-based) that produced the last value in -r.
    #[arg(short = 'n', long, value_parser = parse_u64, conflicts_with = "n_range")]
    n: Option<u64>,
    /// Try every n in LO..HI (inclusive) when the exact call count is unknown.
    #[arg(long, value_parser = parse_range)]
    n_range: Option<RangeInclusive<u64>>,
    /// Non-contiguous observation INDEX:VALUE (1-based index). Repeatable.
    #[arg(long, value_parser = parse_sample)]
    sample: Vec<Observation>,
    /// Only report seeds in LO..HI (inclusive), e.g. a plausible time(NULL) window.
    #[arg(long, value_parser = parse_range)]
    seed_range: Option<RangeInclusive<u64>>,
    /// List every seed in each equivalence class.
    #[arg(long)]
    expand: bool,
    /// Allow brute-force searches larger than 2^40 candidates.
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct GenArgs {
    #[command(flatten)]
    generator: GeneratorArgs,
    #[arg(short, long, value_parser = parse_u64)]
    seed: u64,
    /// Number of outputs.
    #[arg(short = 'n', long, default_value_t = 1)]
    count: u64,
    /// Skip this many calls first (computed in O(log n)).
    #[arg(long, default_value_t = 0, value_parser = parse_u64)]
    skip: u64,
    /// Print full states instead of truncated outputs.
    #[arg(long)]
    states: bool,
}

#[derive(Args)]
struct ParamsArgs {
    /// Consecutive full states.
    #[arg(short, long, num_args = 1.., required = true, value_parser = parse_u64)]
    states: Vec<u64>,
    /// Known modulus.
    #[arg(short, value_parser = parse_num)]
    m: Option<u128>,
    /// Known multiplier (requires -m).
    #[arg(short, value_parser = parse_u64, requires = "m")]
    a: Option<u64>,
}

fn parse_num(s: &str) -> Result<u128, String> {
    let s = s.trim().replace('_', "");
    if let Some((base, exp)) = s.split_once("**").or_else(|| s.split_once('^')) {
        let base = parse_num(base)?;
        let exp: u32 = exp.parse().map_err(|e| format!("bad exponent {exp:?}: {e}"))?;
        return base.checked_pow(exp).ok_or_else(|| format!("{s} overflows"));
    }
    let r = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u128::from_str_radix(hex, 16),
        None => s.parse(),
    };
    r.map_err(|e| format!("bad number {s:?}: {e}"))
}

fn parse_u64(s: &str) -> Result<u64, String> {
    // Accept negative values the way C/Java print them (e.g. Java nextInt) as two's complement.
    if let Some(neg) = s.trim().strip_prefix('-') {
        let v = parse_num(neg)?;
        return u32::try_from(v)
            .map(|v| (v as i64).wrapping_neg() as u32 as u64)
            .map_err(|_| format!("{s} is out of range"));
    }
    u64::try_from(parse_num(s)?).map_err(|_| format!("{s} does not fit in 64 bits"))
}

fn parse_range(s: &str) -> Result<RangeInclusive<u64>, String> {
    let (lo, hi) = s
        .split_once("..")
        .ok_or_else(|| format!("expected LO..HI, got {s:?}"))?;
    let (lo, hi) = (parse_u64(lo)?, parse_u64(hi.trim_start_matches('='))?);
    if lo > hi {
        return Err(format!("empty range {s}"));
    }
    Ok(lo..=hi)
}

fn parse_sample(s: &str) -> Result<Observation, String> {
    let (i, v) = s
        .split_once(':')
        .ok_or_else(|| format!("expected INDEX:VALUE, got {s:?}"))?;
    Ok(Observation {
        index: parse_u64(i)?,
        value: parse_u64(v)?,
    })
}

fn print_classes(classes: &[SeedClass], expand: bool) {
    for c in classes {
        if c.free_bits == 0 {
            println!("  {} ({:#x})", c.seed, c.seed);
        } else if expand {
            let seeds: Vec<String> = c.seeds().take(1 << 16).map(|s| format!("{s} ({s:#x})")).collect();
            let more = if c.len() > 1 << 16 { " ..." } else { "" };
            println!("  {}{more}", seeds.join(", "));
        } else {
            println!(
                "  {} ({:#x}) + k*2^{} for k < 2^{}  (bits >= {} never affect output)",
                c.seed, c.seed, c.fixed_bits, c.free_bits, c.fixed_bits
            );
        }
    }
}

fn run_seed(args: SeedArgs) -> Result<(), String> {
    let generator = args.generator.build()?;
    let opts = Options {
        force: args.force,
        seed_filter: args.seed_range,
    };
    if let Some(range) = args.n_range {
        if args.values.is_empty() {
            return Err("--n-range needs -r values".into());
        }
        let results =
            anti_lcg::recover_seed_range(&generator, &args.values, range, &opts).map_err(|e| e.to_string())?;
        println!("{} value(s) of n produced candidates", results.len());
        for (n, classes) in results {
            println!("n = {n}:");
            print_classes(&classes, args.expand);
        }
        return Ok(());
    }
    let obs = if !args.sample.is_empty() {
        args.sample
    } else if !args.values.is_empty() {
        let n = args.n.unwrap_or(args.values.len() as u64);
        anti_lcg::observations_from_tail(&args.values, n).map_err(|e| e.to_string())?
    } else {
        return Err("give outputs with -r (and -n / --n-range) or --sample".into());
    };
    let classes = anti_lcg::recover_seed(&generator, &obs, &opts).map_err(|e| e.to_string())?;
    let total: u128 = classes.iter().map(SeedClass::len).sum();
    println!("Found {} seed class(es), {total} seed(s) in total:", classes.len());
    print_classes(&classes, args.expand);
    Ok(())
}

fn run_gen(args: GenArgs) -> Result<(), String> {
    let generator = args.generator.build()?;
    let seed = generator.lcg.m.reduce(args.seed as u128);
    let start = generator
        .lcg
        .jump(args.skip as i128)
        .map_err(|e| e.to_string())?
        .apply(seed);
    let items: Vec<String> = generator
        .lcg
        .states(start)
        .take(args.count as usize)
        .map(|x| if args.states { x } else { generator.trunc.apply(x) }.to_string())
        .collect();
    println!("{}", items.join(" "));
    Ok(())
}

fn run_params(args: ParamsArgs) -> Result<(), String> {
    let lcgs = anti_lcg::recover_params(&args.states, args.m, args.a).map_err(|e| e.to_string())?;
    if lcgs.is_empty() {
        return Err("no LCG fits these states".into());
    }
    println!("Found {} parameter set(s):", lcgs.len());
    for l in lcgs {
        println!(
            "  a = {} ({:#x}), c = {} ({:#x}), m = {} ({:#x})",
            l.a,
            l.a,
            l.c,
            l.c,
            l.m.value(),
            l.m.value()
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Seed(a) => run_seed(a),
        Command::Gen(a) => run_gen(a),
        Command::Params(a) => run_params(a),
        Command::Presets => {
            for p in PRESETS {
                println!(
                    "{:<18} a={:#x} c={:#x} m={:#x} shift={} bits={}  {}",
                    p.name, p.a, p.c, p.m, p.shift, p.bits, p.description
                );
            }
            Ok(())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
