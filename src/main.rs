mod benchmark;

use std::env;
use std::process::ExitCode;
use mp_decoder::binary_decoding::{DecodeError, Decoder, DecoderRule};
use mp_decoder::parsing::alist_to_tanner;
use mp_decoder::tanner_graph::Bit;

const USAGE: &str = "usage: mp_decoder --code <alist file> [--rule gallager|logratio|minsum] \
[--p <error rate>] [--rounds <n>] [--max-weight <w>] <syndrome>\n       mp_decoder --bench <benchmark file>";

struct Config {
    code_path: String,
    rule: DecoderRule,
    error_rate: Option<f64>,
    max_rounds: usize,
    max_weight: Option<usize>,
    syndrome: Vec<Bit>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.first().map(|a| a.as_str()) == Some("--bench") {
        let Some(path) = args.get(1) else {
            eprintln!("--bench needs a value\n{USAGE}");
            return ExitCode::from(2);
        };
        return match benchmark::parse_bench_file(path) {
            Ok(config) => benchmark::run_benchmark(config),
            Err(msg) => {
                eprintln!("{msg}");
                ExitCode::from(2)
            }
        };
    }

    let config = match parse_args(&args) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };
    run(config)
}

fn run(config: Config) -> ExitCode {
    let tanner = match alist_to_tanner(&config.code_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not read code file '{}': {e}", config.code_path);
            return ExitCode::from(2);
        }
    };
    let decoder = Decoder::new(tanner);

    let result = match config.rule {
        DecoderRule::Gallager => decoder.gallager_decode(&config.syndrome, config.max_rounds, config.max_weight),
        rule => decoder.decode(&config.syndrome, rule, config.max_rounds, config.error_rate, config.max_weight),
    };

    match result {
        Ok(correction) => {
            println!("correction:   {}", bits_to_string(&correction));
            println!("flipped bits: {:?}", flipped_indices(&correction));
            ExitCode::SUCCESS
        }
        Err(e @ DecodeError::NotConverged { .. }) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut code_path = None;
    let mut rule = DecoderRule::LogRatio;
    let mut error_rate = None;
    let mut max_rounds = 50;
    let mut max_weight = None;
    let mut syndrome = None;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--code" => code_path = Some(next_value(&mut it, "--code")?.to_string()),
            "--rule" => rule = parse_rule(next_value(&mut it, "--rule")?)?,
            "--p" => error_rate = Some(parse_num::<f64>(next_value(&mut it, "--p")?, "--p")?),
            "--rounds" => max_rounds = parse_num(next_value(&mut it, "--rounds")?, "--rounds")?,
            "--max-weight" => max_weight = Some(parse_num(next_value(&mut it, "--max-weight")?, "--max-weight")?),
            "-h" | "--help" => return Err(USAGE.to_string()),
            s if s.starts_with("--") => return Err(format!("unknown flag '{s}'\n{USAGE}")),
            s => syndrome = Some(parse_syndrome(s)?),
        }
    }

    if let Some(p) = error_rate {
        if !(p > 0.0 && p < 1.0) {
            return Err(format!("--p must be strictly between 0 and 1, got {p}"));
        }
    }

    Ok(Config {
        code_path: code_path.ok_or(format!("missing --code\n{USAGE}"))?,
        rule,
        error_rate,
        max_rounds,
        max_weight,
        syndrome: syndrome.ok_or(format!("missing syndrome\n{USAGE}"))?,
    })
}

fn next_value<'a>(it: &mut std::slice::Iter<'a, String>, flag: &str) -> Result<&'a str, String> {
    it.next().map(|s| s.as_str()).ok_or(format!("{flag} needs a value"))
}

fn parse_num<T: std::str::FromStr>(s: &str, flag: &str) -> Result<T, String> {
    s.parse().map_err(|_| format!("invalid value for {flag}: '{s}'"))
}

fn parse_rule(s: &str) -> Result<DecoderRule, String> {
    match s.to_lowercase().as_str() {
        "gallager" => Ok(DecoderRule::Gallager),
        "logratio" => Ok(DecoderRule::LogRatio),
        "minsum" => Ok(DecoderRule::MinSum),
        "normalizedminsum" => Ok(DecoderRule::NormalizedMinSum),
        _ => Err(format!("unknown rule '{s}' (expected gallager, logratio, minsum or normalizedminsum)")),
    }
}

fn parse_syndrome(s: &str) -> Result<Vec<Bit>, String> {
    s.chars()
        .enumerate()
        .map(|(i, ch)| match ch {
            '0' => Ok(Bit::ZERO),
            '1' => Ok(Bit::ONE),
            _ => Err(format!("syndrome may only contain 0/1, found '{ch}' at position {i}")),
        })
        .collect()
}

fn bits_to_string(bits: &[Bit]) -> String {
    bits.iter().map(|b| b.to_string()).collect()
}

fn flipped_indices(bits: &[Bit]) -> Vec<usize> {
    bits.iter().enumerate().filter(|(_, b)| **b == Bit::ONE).map(|(i, _)| i).collect()
}
