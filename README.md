# machine-cat

Generic AIR (Algebraic Intermediate Representation) chip framework built on [proof-cat](https://github.com/MavenRain/proof-cat), [plonkish-cat](https://github.com/MavenRain/plonkish-cat), and [comp-cat-rs](https://github.com/MavenRain/comp-cat-rs).

Generalizes from plonkish-cat's wire-indexed constraints to **execution trace tables** where constraint polynomials must hold at every consecutive row pair.  This is the standard STARK/AIR model, and Subset 2 from the SP1 decomposition.

## Key concept

plonkish-cat constrains individual wires: `Wire(3) - Wire(1) * Wire(2) = 0`.

machine-cat constrains **columns across rows**: "for every row i, `next_a - current_b = 0`."  This is how real proof systems (STARKs, SP1, Plonky3) work: constraints are **transition rules** that the execution trace must obey.

## Architecture

```text
Air::generate_trace(input) -> Trace<F>
             |
   bridge::air_prove(air, trace) -> AirProof<F>
             |
   bridge::air_verify(air, proof) -> Ok(true)
```

## Modules

| Module | Purpose |
|--------|---------|
| `column` | `Column`, `ColumnCount`, `ColumnRef` newtypes (row-relative addressing) |
| `air_expr` | `AirExpr<F>`: constraint expressions with `Current(col)` / `Next(col)` references |
| `trace` | `Trace<F>`: 2D execution trace (rows x columns) |
| `air` | `Air<F>` trait: defines columns, constraints, and trace generation |
| `fibonacci` | `FibonacciAir`: a concrete example proving Fibonacci computation |
| `bridge` | `air_prove` / `air_verify`: trace-to-sumcheck proof via proof-cat |

## Quick start

```rust
use plonkish_cat::F101;
use machine_cat::{Air, FibonacciAir, FibonacciInput, StepCount};
use machine_cat::bridge::{air_prove, air_verify};

fn main() -> Result<(), machine_cat::Error> {
    let air = FibonacciAir;
    let input = FibonacciInput::new(
        F101::new(1), F101::new(1), StepCount::new(7),
    );

    // Generate the execution trace: 8 rows of Fibonacci values.
    let trace = air.generate_trace(&input)?;

    // Prove the trace satisfies the Fibonacci transition constraints.
    let proof = air_prove(&air, &trace)?;

    // Verify (no trace needed, only the AIR definition and proof).
    assert!(air_verify(&air, &proof)?);
    Ok(())
}
```

## How it works

### The Air trait

An `Air<F>` defines:
1. **Column count**: how many field elements per row.
2. **Transition constraints**: `AirExpr<F>` expressions using `Current(col)` and `Next(col)` that must equal zero at every consecutive row pair.
3. **Trace generation**: given an input, produce the execution trace.

### The Fibonacci example

The `FibonacciAir` has 2 columns (a, b) and 2 constraints:
- `next_a - current_b = 0` (next row's a = current row's b)
- `next_b - current_a - current_b = 0` (next row's b = sum of current a and b)

Given initial values (1, 1), the trace is:

| Row | a | b |
|-----|---|---|
| 0   | 1 | 1 |
| 1   | 1 | 2 |
| 2   | 2 | 3 |
| 3   | 3 | 5 |
| 4   | 5 | 8 |
| ... | ... | ... |

### Proof protocol

1. **Validate**: check that the trace satisfies all constraints at every row pair.
2. **Commit**: flatten the trace into a Merkle tree.
3. **Batch**: squeeze random challenges from a Fiat-Shamir transcript and form a random linear combination of constraint evaluations.
4. **Sumcheck**: prove the combined evaluation polynomial sums to zero.
5. **Open**: open all trace column values with Merkle proofs.

The verifier replays the transcript, checks the sumcheck, verifies Merkle openings, and re-evaluates the constraints.

### Categorical interpretation

Each AIR is a morphism in a category where objects are trace shapes (column counts).  Parallel composition of independent AIRs is the tensor product (column concatenation).  This structure is documented but not type-level enforced in v0.1; multi-chip machines with cross-chip lookup arguments are planned for v0.2.

## Building

```bash
cargo build
cargo test
RUSTFLAGS="-D warnings" cargo clippy
cargo doc --no-deps --open
```

## Dependencies

- **proof-cat**: sumcheck proving infrastructure, Merkle commitment, Fiat-Shamir transcript
- **plonkish-cat**: `Field` trait (transitive through proof-cat)
- **sha2**: SHA-256 for transcript hashing (transitive through proof-cat)

## Roadmap

- **v0.1** (current): single-AIR proving with Fibonacci example
- **v0.2**: `Machine` type (collection of AIRs), cross-chip lookup arguments via spans from comp-cat-rs
- **v0.3**: `MonoidalCategory` implementation for parallel AIR composition

## License

MIT
