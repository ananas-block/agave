# BN254 (`alt_bn128`) syscall benches

Original PR: [solana-labs/solana#27961](https://github.com/solana-labs/solana/pull/27961)


> The 2022 pricing is based on **33 ns execution time per CU**.
>
> - Addition: 334 CU
> - Multiplication: 3,840 CU
> - Pairing: 36,364 CU + (numberOfPairings - 1) × 12,121 CU

## Cross-platform comparison: 2022 PR (Zen 3) vs this bench (M5 Pro)

| op | 2022 c6a.2xlarge (Zen 3) | M5 Pro (ark 0.5) | M5 Pro (ark 0.4) |
| --- | ---: | ---: | ---: |
| G1 add (BE) | 4.155 µs | 2.367 µs | 88.484 µs |
| G1 mul (BE) | 126.68 µs | 48.453 µs | 91.743 µs |
| Pairing n=2 | 1.372 ms | — | — |
| Pairing n=3 | 1.682 ms | — | — |
| Pairing n=4 | 1.998 ms | 789.33 µs | 1.191 ms |
| Pairing n=5 | 2.323 ms | — | — |

## Full M5 Pro results (cycled-input, BE)

| op | time (ark 0.5) | time (ark 0.4) | CU @ 33 ns (ark 0.5) | mainnet CU |
| --- | ---: | ---: | ---: | ---: |
| G1 add | 2.367 µs | 88.484 µs | 72 | 334 |
| G1 mul | 48.453 µs | 91.743 µs | 1,468 | 3,840 |
| G2 add | 4.090 µs | 4.127 µs | 124 | 535 |
| G2 mul | 199.28 µs | 256.96 µs | 6,039 | 15,670 |
| Pairing n=4 | 789.33 µs | 1.191 ms | 23,919 | 72,727 |



## Run

```
cargo bench -p solana-syscalls --bench alt_bn128 -- "random"
```

---

# Poseidon hash syscall bench

Original PR: [solana-labs/solana#32680](https://github.com/solana-labs/solana/pull/32680)

> Mainnet pricing is `61 * n² + 542` CU where `n` is the number of 32-byte
> field-element inputs (1 ≤ n ≤ 12). Coefficients live at
> `program-runtime/src/execution_budget.rs:240-241`.

## Cross-platform comparison: light-poseidon README (Zen 4) vs this bench (M5 Pro)

| n inputs | light-poseidon Ryzen 9 7945HX (Zen 4) | this bench, M5 Pro |
| --- | ---: | ---: |
| 1 | 12.735 µs | 9.995 µs |
| 2 | 18.963 µs | 15.125 µs |
| 4 | 38.513 µs | 31.629 µs |
| 8 | 105.49 µs | 86.948 µs |
| 12 | 210.81 µs | 176.00 µs |

## Full M5 Pro results (cycled-input, BE)

| n inputs | time | CU @ 33 ns/CU | mainnet CU (61n² + 542) |
| --- | ---: | ---: | ---: |
| 1 | 9.995 µs | 303 | 603 |
| 2 | 15.125 µs | 458 | 786 |
| 4 | 31.629 µs | 958 | 1,518 |
| 8 | 86.948 µs | 2,635 | 4,446 |
| 12 | 176.00 µs | 5,333 | 9,326 |

## Run

```
cargo bench -p solana-syscalls --bench poseidon
```

---

# alt_bn128 G1/G2 compression syscall bench

Original PR: [solana-labs/solana#32870](https://github.com/solana-labs/solana/pull/32870)

> Mainnet pricing was derived directly from the 2023 PR's c6a.2xlarge (Zen 3)
> bench at **33 ns per CU**, no buffer:
>
> - g1_compress: 30 CU
> - g1_decompress: 398 CU
> - g2_compress: 86 CU
> - g2_decompress: 13,610 CU

## Cross-platform comparison: 2023 PR (Zen 3) vs this bench (M5 Pro)

| op | 2023 c6a.2xlarge (BE) | M5 Pro (ark 0.5, BE) | M5 Pro (ark 0.4, BE) |
| --- | ---: | ---: | ---: |
| g1_compress | 1.0049 µs | 365.02 ns | 366.25 ns |
| g1_decompress | 13.154 µs | 4.6363 µs | 4.6524 µs |
| g2_compress | 2.8543 µs | 1.0421 µs | 1.0429 µs |
| g2_decompress | 449.13 µs | 16.049 µs | 15.514 µs |

## Full M5 Pro results (cycled-input)

| op | BE (ark 0.5) | LE (ark 0.5) | BE (ark 0.4) | LE (ark 0.4) | CU @ 33 ns (ark 0.5, BE) | mainnet CU |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| g1_compress | 365.02 ns | 57.95 ns | 366.25 ns | 59.32 ns | 11 | 30 |
| g1_decompress | 4.636 µs | 4.331 µs | 4.652 µs | 4.377 µs | 141 | 398 |
| g2_compress | 1.042 µs | 92.18 ns | 1.043 µs | 91.38 ns | 32 | 86 |
| g2_decompress | 16.049 µs | 15.236 µs | 15.514 µs | 14.554 µs | 486 | 13,610 |

## Run

```
cargo bench -p solana-syscalls --bench compression
```
