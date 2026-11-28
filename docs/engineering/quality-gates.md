# Quality gates

**Purpose and scope.** Use native tools through one read-only source check interface. Files generated under ignored build/output directories are not source changes. A failing group exits nonzero with the native output and a next step; unknown options exit 2.

## Local checks

Use Git, Python 3.12 (CI; 3.11+ for built-in TOML), Bash/POSIX shell tooling, the [pinned Rust toolchain](../../rust-toolchain.toml) with rustfmt and Clippy, and Node from [`ui/.nvmrc`](../../ui/.nvmrc) with npm. Install pinned Python packages in an isolated environment and UI packages from the lockfile:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r tests/contracts/requirements.txt
(cd ui && npm ci)
```

For infrastructure use Docker Compose v2, actionlint 1.7.12 and ShellCheck 0.11.0 on `PATH`. On Linux x86_64, `sh scripts/quality/install-tool.sh <tool> <directory>` downloads the versioned, checksummed upstream binary to a directory you choose. For other platforms use a trusted package of the same version. No validator installs tools or starts services.

| Local group | Existing CI placement | What it checks |
| --- | --- | --- |
| `./scripts/quality/check.sh --guidance` | `contracts` | Agent Skills and engineering links (local paths), architecture/ratchet/aggregate fixtures. Not agent behavior or link fragments. |
| `./scripts/quality/check.sh --contracts` | `contracts` | OpenAPI, JSON Schema, known semantic references and NATS fixtures, historic Article compatibility, and requirements/index/tests. |
| `./scripts/quality/check.sh --rust` | `rust` | Cargo metadata ownership, manifest lint inheritance, no direct `async-nats` outside the shared crate, workspace rustfmt/Clippy/locked tests. |
| `./scripts/quality/check.sh --ui` | `compose` | Prettier with the temporary legacy ratchet, targeted TypeScript/Angular ESLint, strict Playwright typing and strict Angular build. |
| `./scripts/quality/check.sh --infra` | `infra` (required by `contracts`) | actionlint (including inline ShellCheck), standalone shell files and the Compose base plus delivery overlay model using inert fixture values. Does not start any service. |

Pass multiple named groups in order or use `--all` (also the default). Use `./scripts/quality/format.sh --rust` or `./scripts/quality/format.sh --ui src/example.ts` from any directory. For a formatted or deleted legacy UI file, remove its entry from `ui/format-baseline.json`. CI rejects added or rehashed entries after initial adoption. Keep formatting-only changes separate where that reduces review or merge conflicts.

When changing enforcement, also run `python3 tests/quality/prove_native.py`. It exercises both passing and intentionally invalid inputs with the installed Rust, UI, OpenAPI and workflow tools. It uses temporary files or standard input, never application sources.

## Mandatory runtime and supply-chain checks

The wrapper is a static and workspace-unit entry point, not a substitute for real dependency tests or advisories. CI MUST retain the existing real MongoDB replica-set fixtures, NATS integration, Compose restart/outbox/restore and scope tests, npm audit, Playwright and axe, Rust advisories and cargo-deny, service image scan and independent infrastructure image scans. The Manufacturing Preview's separate identity advisory gate and explicit proof feature combinations are not modified by this foundation. For local equivalents see the [application quickstart](../../README.md) and [CI runbook](../development/ci.md). Use an isolated Compose project and unused loopback ports for integration work.

On schedule and manual dispatch, CI additionally runs `cargo-llvm-cov` 0.9.2 with the toolchain's `llvm-tools-preview`:

```sh
rustup component add llvm-tools-preview
cargo llvm-cov --workspace --locked --lcov --output-path target/workspace.lcov
cargo llvm-cov report --workspace --summary-only
```

Compare coverage for affected files against the same instrumentation on the reviewed main revision. This is a test-runner measurement, not HTTP, broker or browser coverage. Explain reductions in critical exercised paths and replace lost evidence before merge; do not optimize for a global number.

## CI adoption and review-only checks

Keep existing required `audit`, `contracts`, `rust` and `compose` contexts. `contracts` already fails if infrastructure validation or PR impact analysis fails, so those checks do not depend on a rule change. The workflow's `quality-gate` requires success for always-on checks and every selected suite and accepts skips only when the event and impact result explicitly permit them. After observing a docs-only PR, UI-only PR, full PR and merge-group run, ask the maintainer to add `quality-gate` as a required status in the existing ruleset using the GitHub Actions integration. Do not remove existing checks or add a workflow-level path filter. No rule changes are made by this branch.

Review rules without a safe automated signal: authorization and database filters, migration and data ownership, frontend architecture and token use, design/accessibility, complexity and broad contract compatibility. Evaluate targeted, structured fitness checks when repeatable violations justify them. Do not add Kubernetes or Dockerfile linters, duplicate scanners, global percentage gates or a mandatory remote analyzer without an observed need. Preserve existing GitHub secret scanning and push protection; [review the baseline](baseline.md).

**Examples and counterexamples.** The isolated [validator fixtures](../../tests/quality/test_foundation.py) reject an unexpected skipped job or a dependency on another owner's package. Treating every skipped job as success, formatting source in a CI check, or relaxing an advisory policy to merge is prohibited.

**Exceptions and references.** Use the [exception process](README.md#rule-strength-and-exceptions) and preserve the [infrastructure standard](infrastructure.md). Last known performance and cache policy live in the [CI runbook](../development/ci.md), not a second workflow.
