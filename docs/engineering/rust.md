# Rust engineering

**Purpose and scope.** Edition 2024; use the Cargo workspace and [pinned toolchain](../../rust-toolchain.toml) for all crates and services.

## Mandatory rules

- Run default rustfmt and locked workspace Clippy with warnings denied. Every member MUST inherit `[workspace.lints]`. Never globally suppress lints or change CI flags to pass. Remove unused imports/code and review resolved features and dependencies; Cargo does not prove a crate is needed.
- Production inputs and dependency failures MUST return errors, not cause a preventable panic, `unwrap`, `expect`, undocumented `unreachable!` or indexing failure. Tests MAY use assertions and unwraps that make a failed setup clear. For an invariant proven locally, prefer types or exhaustive matching; if a panic remains, document the proof and use the smallest lint exception with a reason. New unsafe code needs architectural/security review and a documented safety invariant; use a safe API where possible.
- Model domain IDs/units with meaningful types, finite states with enums, absence with `Option`, and expected failures with `Result`. Validate constructors and transitions before writes. Keep untrusted JSON/BSON, credentials and framework types at boundaries. Use existing decimal, quantity and date representations; never use binary floats for authoritative money. Carry currency, UoM and precision; state the rounding or conversion policy, version/historical inputs and business date's IANA zone separately from UTC instants.
- Domain/application errors MUST distinguish validation, domain rejection, authorization, not found, conflict, persistence, dependency unavailable, timeout and internal failure where relevant. Map them at HTTP/messaging/process boundaries to the existing contract; do not expose raw errors or invent an incompatible universal error envelope.
- Bound dependency deadlines, queues, retries and task concurrency. Define who owns and drains tasks, what happens on cancellation during a commit, and how uncertain outcomes are resolved. Keep blocking work off async executors. A caller timeout does not prove the underlying write was rolled back.

## Recommended and prohibited patterns

Prefer module privacy, explicit constructors, readable borrowing and simple owned values over deep cloning or generalized lifetimes. Share workspace dependency definitions when versions/features align. Prefer a typed domain error to `Box<dyn Error>` inside business rules; a technical bootstrap boundary can aggregate underlying errors while redacting output. Avoid blanket Pedantic/Nursery lints, unchecked casts and blanket allowances.

**Good example:** [Article validation and owner error mapping](references.md#rust). **Counterexample:** accept `serde_json::Value` as an inventory aggregate, then panic on a missing quantity or log the raw adapter error in an API reply.

**Enforcement.** Workspace lint inheritance, service dependency topology, rustfmt, Clippy (default correctness, suspicious, complexity, performance and style) and locked tests are gated. Panic prevention and unused dependency triage still require review; do not infer full panic freedom from Clippy. CI opts into proof features individually when they need disposable dependencies; do not assume all features are compatible.

**Exceptions and references.** Follow the [exception process](README.md#rule-strength-and-exceptions). Adopt defaults from [Rust Style](https://doc.rust-lang.org/style-guide/), [Clippy](https://doc.rust-lang.org/clippy/), and [Cargo](https://doc.rust-lang.org/cargo/reference/workspaces.html). Apply [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html) selectively for naming, safety and API clarity, not as additional mandatory lint suites.
