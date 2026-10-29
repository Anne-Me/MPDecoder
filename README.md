# mp_decoder

Message-passing decoders for binary LDPC codes and quantum CSS codes, written in Rust.
Codes are read from [alist](https://www.inference.org.uk/mackay/codes/alist.html) files and decoded on their Tanner graph.

> **Work in progress:** this project is ongoing, the interface and results may still change.

## Features

Decoders (classical, syndrome based):
- **Gallager** – hard-decision bit flipping
- **Log-ratio** – belief propagation (sum-product) on log-likelihood ratios
- **Min-sum** – min-sum approximation of belief propagation
- **Normalized min-sum** – min-sum with a scaling factor on the check messages

Quantum (CSS codes):
- Independent decoding of X and Z errors, using `Hz` for X errors and `Hx` for Z errors


### TODO
- Binary decoding that takes the dependency between X and Z errors into account (e.g. under depolarizing noise)

## Usage

Build with `cargo build --release`. The binary is `target/release/mp_decoder`; `cargo run --release -- <args>` builds and runs it in one step.

### Decoding an Individual Syndrome

```
mp_decoder --code <alist file> [--rule <decoder>] [--p <error rate>] [--rounds <n>] [--max-weight <w>] <syndrome>
```

| Flag | Default | Description |
|---|---|---|
| `--code` | required | alist file of the parity check matrix |
| `--rule` | `logratio` | `gallager`, `logratio`, `minsum`, `normalizedminsum` |
| `--p` | – | channel error rate, required for all rules except `gallager`, `0 < p < 1` |
| `--rounds` | `50` | maximum number of message-passing rounds |
| `--max-weight` | none | reject corrections with more bit flips than this |
| `<syndrome>` | required | syndrome as a string of `0` and `1` |

Example:
```
cargo run --release -- --code tests/testfiles/96-3-963 --rule minsum --p 0.05 000010000000000001011000000000000001010010000000
```
On success the correction and the indices of the flipped bits are printed.
Exit codes: `0` decoded, `1` did not converge, `2` invalid input.

### Running a Benchmark

```
mp_decoder --bench <benchmark file>
```

A benchmark file contains one `key = value` per line; lists are comma separated and `#` starts a comment.

| Key | Required | Description |
|---|---|---|
| `mode` | no | `classical` (default) or `quantum` |
| `code` | classical | alist file of the code |
| `code_x`, `code_z` | quantum | alist files of `Hx` and `Hz` |
| `decoders` | yes | list of decoders, e.g. `logratio, minsum` |
| `error_rates` | yes | list of bit flip probabilities, `0 < p < 1` |
| `samples` | yes | number of random errors per error rate, `> 0` |
| `max_rounds` | no | maximum number of rounds, default `50` |
| `max_weight` | no | reject corrections with more bit flips than this |
| `seed` | no | RNG seed; if missing, a random seed is drawn and printed |
| `output` | no | CSV file for the results (overwritten) |

Relative paths are resolved from the directory the program is started in.

Minimal example (see [`benchmarks/example.bench`](benchmarks/example.bench)):
```
code = tests/testfiles/96-3-963
decoders = gallager, logratio, minsum, normalizedminsum
error_rates = 0.01, 0.02, 0.05
samples = 1000
seed = 42
output = results.csv
```

For each error rate and decoder the benchmark reports:
- **exact** – correction equals the sampled error
- **other** – correction matches the syndrome but differs from the error
- **no conv.** – decoder did not converge
- **mean time** – average time per decoding

## Example Benchmark

TODO
