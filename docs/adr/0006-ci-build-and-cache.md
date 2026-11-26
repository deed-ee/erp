# ADR 0006: Reuse CI builds without changing merge gates

**Status:** proposed.

Keep `rust`, `contracts`, `compose` and `audit` as stable GitHub Actions check names. Avoid workflow-level PR path filters for these checks. Validate the synthetic merge tree against its base: editorial-only changes run contract/requirements validation, UI-only changes also run the full real-backend Compose job, and all other paths run all four checks. Shared Rust workspace inputs and unknown paths are full-suite inputs; do not approximate the transitive graph using a list of existing service names. A failed classifier or missing output runs the corresponding checks instead of skipping them. Main, merge queue, scheduled, and dispatched runs always use the full suite.

Build each release image once per Compose job and reuse it for startup, restore, scanning, and integration. Restore still uses a separate MongoDB project and tests the same HTTP behavior. Docker caches registry dependencies and intermediate release layers per image; Rust caches workspace dependencies once per runner/toolchain. Platform jobs still use published image pins and checksums. The real-broker Rust test runs beside the workspace tests to reuse the same Cargo profile and features, independently of Compose. Playwright still tests against live services; its development server does not substitute for the production Angular build check.

Use only standard GitHub-hosted runners. PRs have read-only repository tokens and can restore caches visible to their ref, not publish to the trusted main cache. Cache sharing is not a security boundary for secrets: only public dependencies and build outputs may be cached. Release publication is not part of this workflow; future release jobs must build from reviewed refs with separate permissions, provenance, and no PR artifacts or PR caches.

See the [CI guide](../development/ci.md) for measurements, operating controls, and deferred options.
