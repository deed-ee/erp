---
name: erp4us-rust
description: Implement or review Rust crate, Cargo workspace, domain values, errors and async changes in erp4us.
---

Required reading: [Rust](../../../docs/engineering/rust.md) and [examples](../../../docs/engineering/references.md#rust).

1. Inspect the closest crate, existing types, lint inheritance and dependency definitions.
2. Represent outcomes explicitly, map errors at the boundary, and find timeout/cancellation/retry paths.
3. Run `./scripts/quality/check.sh --rust` and any feature-specific checks.

Do not panic on external errors, use binary floating point for authoritative money, or suppress a lint across a crate.

**Done:** positive and negative invariants are tested, and the chosen toolchain and feature checks are recorded.
