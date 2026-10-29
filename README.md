# mp_decoder

Message-passing decoders for binary LDPC codes and quantum CSS codes, written in Rust.
Codes are read from [alist](https://www.inference.org.uk/mackay/codes/alist.html) files and decoded on their Tanner graph.

> **Work in progress:** this project is ongoing

## Features

Decoders (classical, syndrome based):
- **Gallager** – hard-decision bit flipping
- **Log-ratio** – belief propagation (sum-product) on log-likelihood ratios
- **Min-sum** – min-sum approximation of belief propagation
- **Normalized min-sum** – min-sum with a scaling factor on the check messages

Quantum (CSS codes):
- Independent decoding of X and Z errors, using `Hz` for X errors and `Hx` for Z errors
- **TODO** decoding that takes the dependency between X and Z errors into account 


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
| `error_rates` | yes | list of error rates `0 < p < 1`: bit flip probability (classical) or depolarizing rate, X, Y, Z each `p/3` (quantum) |
| `samples` | yes | number of random errors per error rate, `> 0` |
| `max_rounds` | no | maximum number of rounds, default `50` |
| `max_weight` | no | reject corrections with more bit flips than this |
| `seed` | no | RNG seed; if missing, a random seed is drawn and printed |
| `output` | no | CSV file for the results (overwritten) |

Relative paths are resolved from the directory the program is started in.

Classical example (see [`benchmarks/example.bench`](benchmarks/example.bench)):
```
code = tests/testfiles/96-3-963
decoders = gallager, logratio, minsum, normalizedminsum
error_rates = 0.01, 0.02, 0.05
samples = 1000
seed = 42
output = results.csv
```

Quantum example with the [[144, 12, 12]] bivariate bicycle (gross) code (see [`benchmarks/quantum_example.bench`](benchmarks/quantum_example.bench)):
```
mode = quantum
code_x = tests/testfiles/bb144_gross_Hx.alist
code_z = tests/testfiles/bb144_gross_Hz.alist
decoders = gallager, logratio, minsum, normalizedminsum
error_rates = 0.01, 0.02, 0.05, 0.08
samples = 1000
seed = 84932
output = benchmarks/quantum_example_results.csv
```

For each error rate and decoder the benchmark reports:
- **exact** – correction equals the sampled error
- **equivalent** (quantum only) – correction differs from the error only by a stabilizer, so it is still correct
- **other** – correction matches the syndrome but differs from the error (quantum: a logical error)
- **no conv.** – decoder did not converge
- **mean time** – average time per decoding

## Example Benchmark

Quantum benchmark of the [[144, 12, 12]] gross code with depolarizing noise, independent X/Z decoding,
1000 samples per error rate, `max_rounds = 50`, seed `84932` ([`benchmarks/quantum_example.bench`](benchmarks/quantum_example.bench)):

| p | decoder | exact | equivalent | other | no conv. | mean time |
|---|---|---:|---:|---:|---:|---:|
| 0.01 | gallager | 99.50% | 0.00% | 0.00% | 0.50% | 121 µs |
| 0.01 | logratio | 100.00% | 0.00% | 0.00% | 0.00% | 209 µs |
| 0.01 | minsum | 99.40% | 0.00% | 0.00% | 0.60% | 105 µs |
| 0.01 | normalizedminsum | 99.90% | 0.00% | 0.00% | 0.10% | 99 µs |
| 0.02 | gallager | 97.90% | 0.10% | 0.00% | 2.00% | 195 µs |
| 0.02 | logratio | 100.00% | 0.00% | 0.00% | 0.00% | 293 µs |
| 0.02 | minsum | 97.90% | 0.00% | 0.00% | 2.10% | 154 µs |
| 0.02 | normalizedminsum | 99.80% | 0.00% | 0.10% | 0.10% | 128 µs |
| 0.05 | gallager | 56.80% | 0.30% | 0.10% | 42.80% | 775 µs |
| 0.05 | logratio | 93.10% | 2.10% | 2.20% | 2.60% | 798 µs |
| 0.05 | minsum | 57.70% | 0.30% | 0.20% | 41.80% | 711 µs |
| 0.05 | normalizedminsum | 92.60% | 2.10% | 1.90% | 3.40% | 291 µs |
| 0.08 | gallager | 13.00% | 0.00% | 0.00% | 87.00% | 1.27 ms |
| 0.08 | logratio | 72.20% | 6.40% | 4.50% | 16.90% | 1.93 ms |
| 0.08 | minsum | 15.00% | 0.00% | 0.00% | 85.00% | 1.08 ms |
| 0.08 | normalizedminsum | 70.10% | 5.20% | 4.00% | 20.70% | 401 µs |

