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

Noise Models:
- **Depolarizing** X, Y and Z errors with probability p/3 each
- **Independent** X errors with probability p_x and Z errors with probability p_z
- **Stim** STIM is a software package designed to simulate circuit-level noise. This decoder can read a .dem file created from this package and then sample syndromes based on it.

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

A benchmark runs the decoding for each specified decoder, for each error rate for the number of samples where the sample is a syndrome created at random according to the error pattern.
A benchmark file contains one `key = value` per line; lists are comma separated and `#` starts a comment. 

| Key | Required | Description |
|---|---|---|
| `mode` | no | `classical` (default) or `quantum` |
| `code` | classical | alist file of the code |
| `code_x`, `code_z` | quantum | alist files of `Hx` and `Hz` |
| `noise` | no | quantum only: `depolarizing` (default) or `independent` or `dem`|
| `dem`| for noise=stim | dem files |
| `error_rates` | yes (for noise!=stim) | list of error rates, each strictly between 0 and 1: bit flip probability `p` (classical), depolarizing rate `p` with X, Y, Z each `p/3` (`noise = depolarizing`), or pairs `px:pz` of independent X and Z flip probabilities (`noise = independent`) |
| `decoders` | yes | list of decoders, e.g. `logratio, minsum` |
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
noise = depolarizing # optional, depolarizing (default) or independent (use px:pz pairs in error_rates) or stim
code_x = tests/testfiles/bb144_gross_Hx.alist
code_z = tests/testfiles/bb144_gross_Hz.alist
decoders = gallager, logratio, minsum, normalizedminsum
error_rates = 0.01, 0.02, 0.05, 0.08
samples = 1000
seed = 84932
output = benchmarks/quantum_example_results.csv
max_rounds = 50
```

With `noise = independent` every qubit gets an X error with probability `px` and, independently, a Z error with probability `pz`
(see [`benchmarks/quantum_independent_example.bench`](benchmarks/quantum_independent_example.bench)):
```
mode = quantum
code_x = tests/testfiles/bb144_gross_Hx.alist
code_z = tests/testfiles/bb144_gross_Hz.alist
noise = independent
decoders = gallager, logratio, minsum, normalizedminsum
error_rates = 0.01:0.01, 0.01:0.05, 0.05:0.01, 0.05:0.05
samples = 1000
seed = 84932
output = benchmarks/quantum_independent_example_results.csv
```

With `noise = stim` you need to remove the lines for code_x, code_z and error_rates and add a line `dem = file.dem` which contains one or more paths to dem files. 


For each error rate and decoder the benchmark reports:
- **exact** – correction equals the sampled error
- **equivalent** (quantum only) – correction differs from the error only by a stabilizer, so it is still correct
- **other** – correction matches the syndrome but differs from the error (quantum: a logical error)
- **no conv.** – decoder did not converge
- **mean time** – average time per decoding

## Example Benchmark

Quantum benchmark of the [[144, 12, 12]] gross code with depolarizing noise, independent X/Z decoding,
1000 samples per error rate, `max_rounds = 50`, seed `84932`, decoder prior `2p/3` ([`benchmarks/quantum_example.bench`](benchmarks/quantum_example.bench)):

| p | decoder | exact | equivalent | other | no conv. | mean time |
|---|---|---:|---:|---:|---:|---:|
| 0.01 | gallager | 99.50% | 0.00% | 0.00% | 0.50% | 149 µs |
| 0.01 | logratio | 99.90% | 0.00% | 0.00% | 0.10% | 137 µs |
| 0.01 | minsum | 99.40% | 0.00% | 0.00% | 0.60% | 69 µs |
| 0.01 | normalizedminsum | 99.90% | 0.00% | 0.00% | 0.10% | 65 µs |
| 0.02 | gallager | 97.90% | 0.10% | 0.00% | 2.00% | 124 µs |
| 0.02 | logratio | 99.90% | 0.00% | 0.00% | 0.10% | 168 µs |
| 0.02 | minsum | 97.60% | 0.10% | 0.00% | 2.30% | 98 µs |
| 0.02 | normalizedminsum | 99.80% | 0.10% | 0.00% | 0.10% | 83 µs |
| 0.05 | gallager | 56.80% | 0.40% | 0.00% | 42.80% | 487 µs |
| 0.05 | logratio | 93.00% | 4.30% | 0.10% | 2.60% | 461 µs |
| 0.05 | minsum | 58.00% | 0.50% | 0.00% | 41.50% | 419 µs |
| 0.05 | normalizedminsum | 92.60% | 3.90% | 0.10% | 3.40% | 182 µs |
| 0.08 | gallager | 13.00% | 0.00% | 0.00% | 87.00% | 796 µs |
| 0.08 | logratio | 71.30% | 10.10% | 1.70% | 16.90% | 1148 µs |
| 0.08 | minsum | 14.90% | 0.00% | 0.00% | 85.10% | 732 µs |
| 0.08 | normalizedminsum | 70.10% | 7.60% | 1.60% | 20.70% | 386 µs |

