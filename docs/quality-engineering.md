# Quality engineering

The [CI workflow](../.github/workflows/ci.yml) runs on pull requests and pushes to `main`. It uses read-only repository permissions and retains the existing Rust, contracts, Compose and dependency-audit jobs. Workflow lint and Rust coverage add feedback; this change does not change repository rules or impose a coverage threshold.

## Reproduce the checks

From the repository root, with the pinned Rust toolchain, `jq`, and Python 3.12:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash -o pipefail -c 'cargo metadata --format-version 1 --no-deps --locked | jq -e -f scripts/check-service-boundaries.jq'

python3.12 -m venv .venv
.venv/bin/python -m pip install -r tests/contracts/requirements.txt
.venv/bin/python tests/contracts/check.py
.venv/bin/python tests/requirements/check.py
.venv/bin/python -m unittest discover -s tests/requirements
```

The boundary check rejects direct Cargo path dependencies between distinct `services/<name>/` trees, including direct development and build dependencies. It permits packages within a service and shared crates outside `services/`. It does not establish transitive, database, or runtime isolation.

For Linux x86-64, the installer downloads pinned upstream tool archives and checks their SHA-256 digests before extracting the binaries. On other platforms, install matching versions from their upstream releases.

```sh
sh scripts/install-quality-tool.sh actionlint target/quality-tools
target/quality-tools/actionlint

rustup component add llvm-tools-preview
sh scripts/install-quality-tool.sh cargo-llvm-cov target/quality-tools
export PATH="$PWD/target/quality-tools:$PATH"
cargo llvm-cov --workspace --locked --lcov --output-path target/workspace.lcov
cargo llvm-cov report --workspace --summary-only
```

CI attaches `rust-workspace-coverage` for 14 days. The report measures instrumented Rust workspace tests, not the separately built Compose service, end-to-end flows, or Angular. A covered line is not evidence that a business scenario passed.

## Initial baseline

At source revision `8b1aa9c`, two Rust domain tests and six Python catalog tests passed locally. The Rust suite covered 49 of 299 executable lines (16.39%), 53 of 319 regions (16.61%) and 6 of 44 functions (13.64%). The HTTP, persistence and startup paths were not executed by those Rust unit tests.

The [existing CI run](https://github.com/erp4us-org/erp4us/actions/runs/45277286709) passed on that revision: Rust 77 s, contracts 17 s, Compose 146 s, audit 156 s. These are elapsed job times from one run, not performance targets. Compose independently exercises the demo API, duplicate SKU handling, restart, transactional outbox rollback and restore. Run it only against disposable synthetic data as described in the [quickstart](../README.md).

## Follow-up and evidence

Use [issue #18](https://github.com/erp4us-org/erp4us/issues/18) for preview readiness and the linked platform requirements; a passing job does not approve or complete a catalog record. Prioritize testable HTTP and persistence invariants over a percentage target. Angular remains on a separate branch; add reporting after the workspace lands rather than creating duplicate UI jobs now. Fixed demo-token checks are not tenant-isolation evidence. Coverage thresholds, broker delivery, container provenance and GitOps deployment are not implemented by this change.
