use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use mp_decoder::binary_decoding::{DecodeError, Decoder, DecoderRule};
use mp_decoder::parsing::alist_to_tanner;
use mp_decoder::tanner_graph::{Bit, TannerGraph};

use crate::parse_rule;

pub struct BenchConfig {
    pub code_path: String,
    pub decoders: Vec<DecoderRule>,
    pub error_rates: Vec<f64>,
    pub samples: usize,
    pub max_rounds: usize,
    pub max_weight: Option<usize>,
    pub seed: Option<u64>,
    pub output: Option<String>,
}

struct Sample {
    error: Vec<Bit>,
    syndrome: Vec<Bit>,
}

#[derive(Default)]
struct Stats {
    exact: usize,         // correction == sampled error
    other_valid: usize,   // correction matches the syndrome but differs from the error
    not_converged: usize,
    time: Duration,
}

pub fn parse_bench_file(path: &str) -> Result<BenchConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("could not read benchmark file '{path}': {e}"))?;

    let mut code_path = None;
    let mut decoders = None;
    let mut error_rates = None;
    let mut samples = None;
    let mut max_rounds = 50;
    let mut max_weight = None;
    let mut seed = None;
    let mut output = None;
    let mut seen_keys: Vec<String> = Vec::new();

    for (i, raw_line) in content.lines().enumerate() {
        let line_nr = i + 1;
        // everything after '#' is a comment
        let line = raw_line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or(format!("line {line_nr}: expected 'key = value', got '{line}'"))?;
        let key = key.trim();
        let value = value.trim();

        if seen_keys.iter().any(|k| k == key) {
            return Err(format!("line {line_nr}: key '{key}' is set twice"));
        }
        seen_keys.push(key.to_string());

        match key {
            "code" => code_path = Some(value.to_string()),
            "decoders" => decoders = Some(parse_list(value, line_nr, |s| parse_rule(s))?),
            "error_rates" => error_rates = Some(parse_list(value, line_nr, |s| parse_rate(s))?),
            "samples" => samples = Some(parse_value(value, key, line_nr)?),
            "max_rounds" => max_rounds = parse_value(value, key, line_nr)?,
            "max_weight" => max_weight = Some(parse_value(value, key, line_nr)?),
            "seed" => seed = Some(parse_value(value, key, line_nr)?),
            "output" => output = Some(value.to_string()),
            _ => return Err(format!("line {line_nr}: unknown key '{key}'")),
        }
    }

    Ok(BenchConfig {
        code_path: code_path.ok_or("benchmark file is missing 'code'")?,
        decoders: decoders.ok_or("benchmark file is missing 'decoders'")?,
        error_rates: error_rates.ok_or("benchmark file is missing 'error_rates'")?,
        samples: samples.ok_or("benchmark file is missing 'samples'")?,
        max_rounds,
        max_weight,
        seed,
        output,
    })
}

fn parse_list<T>(value: &str, line_nr: usize, parse: impl Fn(&str) -> Result<T, String>) -> Result<Vec<T>, String> {
    let items: Vec<T> = value
        .split(',')
        .map(|s| parse(s.trim()).map_err(|e| format!("line {line_nr}: {e}")))
        .collect::<Result<_, _>>()?;
    if items.is_empty() {
        return Err(format!("line {line_nr}: list must not be empty"));
    }
    Ok(items)
}

fn parse_value<T: std::str::FromStr>(value: &str, key: &str, line_nr: usize) -> Result<T, String> {
    value.parse().map_err(|_| format!("line {line_nr}: invalid value for {key}: '{value}'"))
}

fn parse_rate(s: &str) -> Result<f64, String> {
    let p: f64 = s.parse().map_err(|_| format!("invalid error rate '{s}'"))?;
    if !(p > 0.0 && p < 1.0) {
        return Err(format!("error rate must be strictly between 0 and 1, got {p}"));
    }
    Ok(p)
}

pub fn run_benchmark(config: BenchConfig) -> ExitCode {
    let tanner = match alist_to_tanner(&config.code_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not read code file '{}': {e}", config.code_path);
            return ExitCode::from(2);
        }
    };
    let (n, m) = (tanner.n, tanner.m);

    let seed = config.seed.unwrap_or_else(|| rand::random());
    let mut rng = StdRng::seed_from_u64(seed);


    let samples_per_rate: Vec<Vec<Sample>> = config
        .error_rates
        .iter()
        .map(|&p| (0..config.samples).map(|_| sample(&tanner, p, &mut rng)).collect())
        .collect();
    let decoder = Decoder::new(tanner);

    // open the output file before decoding so a bad path fails right away
    let mut out = match &config.output {
        Some(path) => match create_csv(path, &config, n, m, seed) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("could not write output file '{path}': {e}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    println!("code: {} (n = {n}, m = {m})", config.code_path);
    println!("samples: {}  max_rounds: {}  max_weight: {}  seed: {seed}",
        config.samples,
        config.max_rounds,
        config.max_weight.map_or("none".to_string(), |w| w.to_string()));
    println!();
    println!("{:<10} {:<18} {:>9} {:>9} {:>9} {:>12}", "p", "decoder", "exact", "other", "no conv.", "mean time");

    for (p, samples) in config.error_rates.iter().zip(&samples_per_rate) {
        for &rule in &config.decoders {
            let stats = match bench_one(&decoder, rule, *p, samples, &config) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{} at p = {p}: {e}", rule_name(rule));
                    return ExitCode::from(2);
                }
            };
            let total = samples.len() as f64;
            let mean_time = stats.time.div_f64(total.max(1.0));
            println!("{:<10} {:<18} {:>8.2}% {:>8.2}% {:>8.2}% {:>12?}",
                p,
                rule_name(rule),
                100.0 * stats.exact as f64 / total,
                100.0 * stats.other_valid as f64 / total,
                100.0 * stats.not_converged as f64 / total,
                mean_time);

            if let Some(w) = out.as_mut() {
                if let Err(e) = writeln!(w, "{p},{},{},{},{},{},{:.3}",
                    rule_name(rule),
                    samples.len(),
                    stats.exact,
                    stats.other_valid,
                    stats.not_converged,
                    mean_time.as_secs_f64() * 1e6) {
                    eprintln!("could not write output file: {e}");
                    return ExitCode::from(2);
                }
            }
        }
    }

    if let Some(mut w) = out {
        if let Err(e) = w.flush() {
            eprintln!("could not write output file: {e}");
            return ExitCode::from(2);
        }
        println!();
        println!("results written to {}", config.output.as_deref().unwrap());
    }
    ExitCode::SUCCESS
}

// csv with the run parameters as '#' comment lines, then one row per (p, decoder)
fn create_csv(path: &str, config: &BenchConfig, n: usize, m: usize, seed: u64) -> std::io::Result<BufWriter<File>> {
    let mut w = BufWriter::new(File::create(path)?);
    writeln!(w, "# code: {} (n = {n}, m = {m})", config.code_path)?;
    writeln!(w, "# samples: {}  max_rounds: {}  max_weight: {}  seed: {seed}",
        config.samples,
        config.max_rounds,
        config.max_weight.map_or("none".to_string(), |w| w.to_string()))?;
    writeln!(w, "p,decoder,samples,exact,other_valid,not_converged,mean_time_us")?;
    Ok(w)
}

fn bench_one(decoder: &Decoder, rule: DecoderRule, p: f64, samples: &[Sample], config: &BenchConfig) -> Result<Stats, DecodeError> {
    let mut stats = Stats::default();
    for s in samples {
        let start = Instant::now();
        let result = match rule {
            DecoderRule::Gallager => decoder.gallager_decode(&s.syndrome, config.max_rounds, config.max_weight),
            rule => decoder.decode(&s.syndrome, rule, config.max_rounds, Some(p), config.max_weight),
        };
        stats.time += start.elapsed();

        match result {
            Ok(correction) if correction == s.error => stats.exact += 1,
            Ok(_) => stats.other_valid += 1,
            Err(DecodeError::NotConverged { .. }) => stats.not_converged += 1,
            // any other error means the setup is wrong, abort the benchmark
            Err(e) => return Err(e),
        }
    }
    Ok(stats)
}

// flip every bit independently with probability p and compute the syndrome H*e
fn sample(tanner: &TannerGraph, p: f64, rng: &mut StdRng) -> Sample {
    let error: Vec<Bit> = (0..tanner.n)
        .map(|_| if rng.random_bool(p) { Bit::ONE } else { Bit::ZERO })
        .collect();
    let syndrome = tanner
        .c_nodes
        .iter()
        .map(|check| check.iter().fold(Bit::ZERO, |acc, &v| acc ^ error[v]))
        .collect();
    Sample { error, syndrome }
}

fn rule_name(rule: DecoderRule) -> &'static str {
    match rule {
        DecoderRule::Gallager => "gallager",
        DecoderRule::LogRatio => "logratio",
        DecoderRule::MinSum => "minsum",
        DecoderRule::NormalizedMinSum => "normalizedminsum",
    }
}
