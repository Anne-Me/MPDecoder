use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use mp_decoder::binary_decoding::{DecodeError, Decoder, DecoderRule};
use mp_decoder::parsing::alist_to_tanner;
use mp_decoder::tanner_graph::{Bit, TannerGraph};
use mp_decoder::binary_decoding_quantum::QuantumDecoder;


use crate::parse_rule;

pub enum CodeSpec {
    Classical { code_path: String },
    Quantum { code_x: String, code_z: String },
}

pub struct BenchConfig {
    pub code: CodeSpec,
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

struct QSample {
    error_x: Vec<Bit>,
    syndrome_x: Vec<Bit>,
    error_z: Vec<Bit>,
    syndrome_z: Vec<Bit>,
}

#[derive(Default)]
struct Stats {
    exact: usize,         // correction == sampled error
    equivalent: usize,    // quantum only, correction leads to correct codeword but is not the original error
    other_valid: usize,   // correction matches the syndrome but differs from the error
    not_converged: usize,
    time: Duration,
}

pub fn parse_bench_file(path: &str) -> Result<BenchConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("could not read benchmark file '{path}': {e}"))?;

    let mut mode = None;
    let mut code_path = None;
    let mut code_x = None;
    let mut code_z = None;
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
            "mode" => mode = Some(match value {
                "classical" | "quantum" => value.to_string(),
                _ => return Err(format!("line {line_nr}: unknown mode '{value}' (expected classical or quantum)")),
            }),
            "code" => code_path = Some((value.to_string(), line_nr)),
            "code_x" => code_x = Some((value.to_string(), line_nr)),
            "code_z" => code_z = Some((value.to_string(), line_nr)),
            "decoders" => decoders = Some(parse_list(value, line_nr, |s| parse_rule(s))?),
            "error_rates" => error_rates = Some(parse_list(value, line_nr, |s| parse_rate(s))?),
            "samples" => {
                let s: usize = parse_value(value, key, line_nr)?;
                if s == 0 {
                    return Err(format!("line {line_nr}: samples must be greater than 0"));
                }
                samples = Some(s);
            }
            "max_rounds" => max_rounds = parse_value(value, key, line_nr)?,
            "max_weight" => max_weight = Some(parse_value(value, key, line_nr)?),
            "seed" => seed = Some(parse_value(value, key, line_nr)?),
            "output" => output = Some(value.to_string()),
            _ => return Err(format!("line {line_nr}: unknown key '{key}'")),
        }
    }

    // keys are remembered with their line number to point at the one that does not fit the mode
    let code = match mode.as_deref().unwrap_or("classical") {
        "classical" => {
            if let Some((_, line_nr)) = code_x.or(code_z) {
                return Err(format!("line {line_nr}: code_x/code_z are only allowed with mode = quantum, use 'code'"));
            }
            CodeSpec::Classical { code_path: code_path.ok_or("benchmark file is missing 'code'")?.0 }
        }
        _ => {
            if let Some((_, line_nr)) = code_path {
                return Err(format!("line {line_nr}: 'code' is not allowed with mode = quantum, use code_x and code_z"));
            }
            CodeSpec::Quantum {
                code_x: code_x.ok_or("mode = quantum needs 'code_x' (alist file of Hx)")?.0,
                code_z: code_z.ok_or("mode = quantum needs 'code_z' (alist file of Hz)")?.0,
            }
        }
    };

    Ok(BenchConfig {
        code,
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
    match &config.code {
        CodeSpec::Classical { code_path } => run_classical(&config, code_path),
        CodeSpec::Quantum { code_x, code_z } => run_quantum(&config, code_x, code_z),
    }
}

fn run_quantum(config: &BenchConfig, code_x: &str, code_z: &str) -> ExitCode {
    // load both files so mistakes in them already show up
    let (hx, hz) = match (load_code(code_x), load_code(code_z)) {
        (Ok(hx), Ok(hz)) => (hx, hz),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if hx.n != hz.n {
        eprintln!("code_x and code_z must act on the same number of qubits, got n = {} and n = {}", hx.n, hz.n);
        return ExitCode::from(2);
    }

    // TODO: maybe also check hx * hz^T = 0 (mod 2)

    println!("code_x: {code_x} (n = {}, m = {})", hx.n, hx.m);
    println!("code_z: {code_z} (n = {}, m = {})", hz.n, hz.m);

    let seed = config.seed.unwrap_or_else(|| rand::random());
    let mut rng = StdRng::seed_from_u64(seed);


    let samples_per_rate: Vec<Vec<QSample>> = config
        .error_rates
        .iter()
        .map(|&p| (0..config.samples).map(|_| q_sample(&hx, &hz, p, &mut rng)).collect())
        .collect();

    // open the output file before decoding so a bad path fails right away
    let mut out = match &config.output {
        Some(path) => match create_csv(path, config, code_x, hx.n, hx.m, seed, QUANTUM_CSV_HEADER) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("could not write output file '{path}': {e}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    println!("code: {code_x} and {code_z}");
    println!("samples: {}  max_rounds: {}  max_weight: {}  seed: {seed}",
        config.samples,
        config.max_rounds,
        config.max_weight.map_or("none".to_string(), |w| w.to_string()));
    println!();
    println!("{:<10} {:<18} {:>9} {:>10} {:>9} {:>9} {:>12}", "p", "decoder", "exact", "equivalent", "other", "no conv.", "mean time");


    let decoder = QuantumDecoder::new(hx, hz);

    for (p, samples) in config.error_rates.iter().zip(&samples_per_rate) {
        for &rule in &config.decoders {
            let stats = match bench_one_quantum(&decoder, rule, *p, samples, config) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{} at p = {p}: {e}", rule_name(rule));
                        return ExitCode::from(2);
                    }
                };
                let total = samples.len() as f64;
                let mean_time = stats.time.div_f64(total.max(1.0));

                println!("{:<10} {:<18} {:>8.2}% {:>8.2}% {:>8.2}% {:>8.2}% {:>12?}",
                p,
                rule_name(rule),
                100.0 * stats.exact as f64 / total,
                100.0 * stats.equivalent as f64 / total,
                100.0 * stats.other_valid as f64 / total,
                100.0 * stats.not_converged as f64 / total,
                mean_time);

            if let Some(w) = out.as_mut() {
                if let Err(e) = writeln!(w, "{p},{},{},{},{},{},{},{:.3}",
                    rule_name(rule),
                    samples.len(),
                    stats.exact,
                    stats.equivalent,
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

fn load_code(path: &str) -> Result<TannerGraph, String> {
    alist_to_tanner(path).map_err(|e| format!("could not read code file '{path}': {e}"))
}

fn run_classical(config: &BenchConfig, code_path: &str) -> ExitCode {
    let tanner = match load_code(code_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
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
        Some(path) => match create_csv(path, config, code_path, n, m, seed, CLASSICAL_CSV_HEADER) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("could not write output file '{path}': {e}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    println!("code: {code_path} (n = {n}, m = {m})");
    println!("samples: {}  max_rounds: {}  max_weight: {}  seed: {seed}",
        config.samples,
        config.max_rounds,
        config.max_weight.map_or("none".to_string(), |w| w.to_string()));
    println!();
    println!("{:<10} {:<18} {:>9} {:>9} {:>9} {:>12}", "p", "decoder", "exact", "other", "no conv.", "mean time");

    for (p, samples) in config.error_rates.iter().zip(&samples_per_rate) {
        for &rule in &config.decoders {
            let stats = match bench_one(&decoder, rule, *p, samples, config) {
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
const CLASSICAL_CSV_HEADER: &str = "p,decoder,samples,exact,other_valid,not_converged,mean_time_us";
const QUANTUM_CSV_HEADER: &str = "p,decoder,samples,exact,equivalent,other_valid,not_converged,mean_time_us";

fn create_csv(path: &str, config: &BenchConfig, code_path: &str, n: usize, m: usize, seed: u64, header: &str) -> std::io::Result<BufWriter<File>> {
    let mut w = BufWriter::new(File::create(path)?);
    writeln!(w, "# code: {code_path} (n = {n}, m = {m})")?;
    writeln!(w, "# samples: {}  max_rounds: {}  max_weight: {}  seed: {seed}",
        config.samples,
        config.max_rounds,
        config.max_weight.map_or("none".to_string(), |w| w.to_string()))?;
    writeln!(w, "{header}")?;
    Ok(w)
}

fn bench_one(decoder: &Decoder, rule: DecoderRule, p: f64, samples: &[Sample], config: &BenchConfig) -> Result<Stats, DecodeError> {
    let mut stats = Stats::default();
    for s in samples {
        let start = Instant::now();
        let result = decoder.decode(&s.syndrome, rule, config.max_rounds, Some(p), config.max_weight);
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
    let syndrome = syndrome_of(tanner, &error);
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

// depolariting noise X, Z or Y with prob p/3
// syndrome_x belongs to error_x and is measured by Hz, syndrome_z by Hx
fn q_sample(hx: &TannerGraph, hz: &TannerGraph, p: f64, rng: &mut StdRng) -> QSample {
    let mut error_x: Vec<Bit> = vec![Bit::ZERO; hx.n];
    let mut error_z: Vec<Bit> = vec![Bit::ZERO; hx.n];
    for i in 0..hx.n {
        // one draw per qubit decides between I, X, Z and Y
        let u: f64 = rng.random();
        if u < p / 3.0 {
            error_x[i] = Bit::ONE;
        } else if u < 2.0 * p / 3.0 {
            error_z[i] = Bit::ONE;
        } else if u < p {
            error_x[i] = Bit::ONE;
            error_z[i] = Bit::ONE;
        }
    }
    let syndrome_x = syndrome_of(hz, &error_x);
    let syndrome_z = syndrome_of(hx, &error_z);
    QSample { error_x, syndrome_x, error_z, syndrome_z }
}

fn syndrome_of(tanner: &TannerGraph, error: &[Bit]) -> Vec<Bit> {
    tanner
        .c_nodes
        .iter()
        .map(|check| check.iter().fold(Bit::ZERO, |acc, &v| acc ^ error[v]))
        .collect()
}

fn bench_one_quantum(decoder: &QuantumDecoder, rule: DecoderRule, p: f64, samples: &[QSample], config: &BenchConfig) -> Result<Stats, DecodeError> {
    let mut stats = Stats::default();
    for s in samples {
        let start = Instant::now();
        let result = decoder.decode(&s.syndrome_x, &s.syndrome_z, rule, config.max_rounds, Some(p), config.max_weight);
        stats.time += start.elapsed();

        match result {
            Ok((correction_x, correction_z)) => {
                if correction_x == s.error_x && correction_z == s.error_z {
                    stats.exact += 1;
                } else if decoder.degenerate_correction(&correction_x, &s.error_x, &correction_z, &s.error_z) {
                    stats.equivalent += 1;
                } else {
                    stats.other_valid +=1;
                }
            }
            Err(DecodeError::NotConverged { .. }) => stats.not_converged += 1,
            // any other error means the setup is wrong, abort the benchmark
            Err(e) => return Err(e),
        }
    }
    Ok(stats)
}
