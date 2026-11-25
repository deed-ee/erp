# CI checks and build caches

The [CI workflow](../../.github/workflows/ci.yml) validates changes; it does not publish releases or deploy. On pull requests, the `impact` job classifies the tested Git merge tree. Markdown under `docs/` and known root editorial files run the contract/requirements checks only; UI changes also run Compose, including Playwright and npm audit. Everything else (including a new service, a change in a common crate, a contract or fixture, a lockfile, or a workflow) triggers the full suite. Rename detection is disabled so both the removed and added paths are classified. Keep the four existing required check names; don't add a workflow-level path filter to `CI`.

Main pushes, merge groups, manual dispatches, and weekly monitoring always run the full system suite, including restart, backup/restore, JetStream, UI, and supply-chain checks. PR branches do not have a second `push` pipeline. Concurrency cancels superseded runs for the same event and ref; a main push still validates the integrated tree. Infrastructure image advisories retain their separate digest-change and weekly checks. There is no release workflow today. Before release, dispatch a full run on the reviewed commit and treat packaging, attestations and publication as a separate protected workflow.

| Cache | Contents and invalidation | Policy |
| --- | --- | --- |
| Rust | Swatinem workspace dependency artifacts and Cargo downloads keyed by runner/toolchain, manifests, lockfile and relevant build settings. CI disables incremental builds and debug info to keep dependencies compact. No workspace binaries or Cargo-installed tools. | Restore on checked refs; save on `main` only. |
| Docker | BuildKit `gha`, one scope per image and namespace. `cargo-chef` freezes the dependency recipe ahead of source compilation. Runtime stages contain only the release executables. | Save intermediate layers on `main`; other refs only import. A cache miss still builds the image normally. |
| npm | `~/.npm/_cacache`, keyed by Node pin, OS/architecture and `ui/package-lock.json`. `npm ci` still verifies the lockfile and installs fresh. | Restore only for PRs; save on `main`. |

All caches are disposable and may be evicted. No credentials, `.env`, private registry configuration, or deployment artifacts belong in a cache. Keep the manual approval policy for Copilot PR workflows; do not use `pull_request_target` to run PR code. For a new compatible Rust service, include its build inputs in `.dockerignore`, give its image a separate scope, and keep module dependencies in the workspace. Monitor cache quota before adding scopes; if import/export outweighs recompilation, reduce retained layers or remove the least useful cache rather than bypassing checks.

## Baseline and measurement

Times below exclude the wait before Actions starts. These are complete successful runs on GitHub-hosted runners before this change.

| Run | Total | `compose` | `rust` | `audit` | `contracts` |
| --- | ---: | ---: | ---: | ---: | ---: |
| [Heartbeat on main](https://github.com/erp4us-org/erp4us/actions/runs/45699579033) | 9:02 | 8:58 | 1:25 | 2:18 | 0:22 |
| [Heartbeat PR](https://github.com/erp4us-org/erp4us/actions/runs/45696793782) | 9:09 | 9:02 | 1:33 | 2:27 | 0:21 |
| [Previous main](https://github.com/erp4us-org/erp4us/actions/runs/45694722381) | 8:00 | 7:58 | 1:36 | 3:05 | 0:12 |
| [Messaging PR](https://github.com/erp4us-org/erp4us/actions/runs/45689113509) | 10:01 | 9:54 | 1:51 | 2:36 | 0:12 |
| [Inventory PR](https://github.com/erp4us-org/erp4us/actions/runs/45559024276) | 7:07 | 7:03 | 2:10 | 2:32 | 0:20 |

The heartbeat run spent 249 seconds across the two Compose build/start steps, 165 seconds on delivery/config/broker checks, and 77 seconds on UI build and Playwright. `cargo install cargo-audit` alone took 134 seconds; CI now uses the same pinned auditor version's checksummed Linux binary and still fetches the advisory database. Historical logs show npm cache hits in all four sampled runs (about 76 MiB each). There was no persistent Rust or BuildKit cache; six npm entries used 470,480,052 bytes at inventory time. An older baseline with fewer tests is useful for locating cost, not for claiming savings.

Local BuildKit validation, repeating each build with identical inputs on the same builder: Master Data's cold dependency and final build vertices took 165.3 s and 12.5 s; Article Delivery's took 135.4 s and 19.7 s. Both vertices hit the warm cache for each image (4/4), and each complete warm rebuild took about 0.3 s. This measures local layer reuse, not remote cache transfer or end-to-end hosted speedup.

**Hosted cold/warm results are pending rollout.** Use the full heartbeat run as the coverage-matched baseline and record its runner image from the job setup. After this workflow is on `main`, manually dispatch `CI` with `benchmark-v1` once, then rerun that successful run at the same SHA. Check the first run actually misses all three cache types; if the namespace has been used already, follow the measurement issue to rotate it rather than delete shared caches. Run sequentially and avoid a runner image or dependency change between the pair.

For each run, record start-to-finish time, every job's duration, and sum of job durations. Inspect named job steps with `gh run view <id> --json jobs` and use `gh run view <id> --log` for cache and compile timings. Record exact and fallback Rust/npm restores separately; for each image count cached `cargo chef cook` and final `cargo build` vertices out of the two eligible vertices, not merely whether the cache index imported. Record the BuildKit import/export durations and the Rust/npm restore/save step durations. Use `gh cache list --ref refs/heads/main --limit 100 --json key,sizeInBytes` to inventory the namespace before and after, including BuildKit layer blobs if visible; mark any unobservable metric unavailable rather than zero. Compare matching compile vertices and Rust steps, not total wall time, to estimate compilation avoided. Report the cold run's export overhead separately from read-only PRs.

`sccache` is deferred: it would add another storage policy and cache transfer path before the existing Rust and Docker caches have been measured. `cargo-nextest` is also deferred: current evidence shows compilation and Compose dominate, and switching the runner would still require retaining doc tests, ignored broker tests, and existing semantics. Revisit both only if measurements show a remaining bottleneck they can reduce.
