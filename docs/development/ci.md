# CI checks and build caches

The [CI workflow](../../.github/workflows/ci.yml) validates changes; it does not publish releases or deploy. On pull requests, the `impact` job classifies the tested Git merge tree. Markdown under `docs/` and known root editorial files run the contract/requirements checks only; UI changes also run Compose, including Playwright and npm audit. Everything else (including a new service, a change in a common crate, a contract or fixture, a lockfile, or a workflow) triggers the full suite. Rename detection is disabled so both the removed and added paths are classified. Keep the four existing required check names; don't add a workflow-level path filter to `CI`.

Main pushes, merge groups, manual dispatches, and weekly monitoring always run the full system suite, including restart, backup/restore, JetStream, UI, and supply-chain checks. PR branches do not have a second `push` pipeline. Concurrency cancels superseded runs for the same event and ref; a main push still validates the integrated tree. Infrastructure image advisories retain their separate digest-change and weekly checks. There is no release workflow today. Before release, dispatch a full run on the reviewed commit and treat packaging, attestations and publication as a separate protected workflow.

| Cache | Contents and invalidation | Policy |
| --- | --- | --- |
| Rust | Swatinem workspace dependency artifacts and Cargo downloads keyed by runner/toolchain, manifests, lockfile and relevant build settings. CI disables incremental builds and debug info to keep dependencies compact. No workspace binaries or Cargo-installed tools. | Restore on checked refs; save on `main` only. |
| Docker | BuildKit `gha`, one scope per image and namespace. `cargo-chef` freezes the dependency recipe ahead of source compilation. Runtime stages use a pinned base and copy the release executables, not the build tools. | Save intermediate layers on `main`; other refs only import. A cache miss still builds the image normally. |
| npm | `~/.npm/_cacache`, keyed by Node pin, OS/architecture and `ui/package-lock.json`. `npm ci` still verifies the lockfile and installs fresh. | Restore on checked refs; save on `main` only. |

All caches are disposable and may be evicted. No credentials, `.env`, private registry configuration, or deployment artifacts belong in a cache. Keep the manual approval policy for Copilot PR workflows; do not use `pull_request_target` to run PR code. For a new compatible Rust service, include its build inputs in `.dockerignore`, give its image a separate scope, and keep module dependencies in the workspace. Monitor cache quota before adding scopes; if import/export outweighs recompilation, reduce retained layers or remove the least useful cache rather than bypassing checks.

Master Data and Article Delivery temporarily install the Bookworm PAM security update during the runtime build because the published base does not yet include the fix for CVE-2025-6020. Once an official pinned base contains the fix, remove the package upgrade layer and run the same release-image advisory gate.

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

### Hosted cold/warm results

The [cold](https://github.com/erp4us-org/erp4us/actions/runs/45703792935/attempts/1) and [warm](https://github.com/erp4us-org/erp4us/actions/runs/45703792935/attempts/2) attempts ran the full suite successfully at the same reviewed SHA (`9a57f5b51925`). All used standard Ubuntu 24.04 runners. The Compose runner image changed from `20261116.376.1` to `20261122.23.1` between attempts; the baseline used the latter. The Rust runner image was `20261116.133.1` in both attempts. These are single-run observations, not an estimate of long-term hit rate.

| Metric | Matched baseline | Cold | Warm |
| --- | ---: | ---: | ---: |
| Total elapsed | 9:02 | 10:12 | 4:41 |
| `compose` | 8:58 | 10:07 | 4:36 |
| `rust` | 1:25 | 1:50 | 1:30 |
| `audit` | 2:18 | 0:21 | 0:25 |
| `contracts` | 0:22 | 0:21 | 0:18 |
| Sum of job durations | 13:03 | 12:39 | 6:49 |

The warm run saved **4:21 (48%)** in elapsed time versus baseline. The cold run took **1:10 longer**, with 121.5 seconds of BuildKit cache export on the critical path. Both Docker build steps combined fell from 353 seconds cold to 6 seconds warm, including the image load. The four warm cache hits skipped roughly 179 seconds of cold Docker compilation (the two `cook` and two final Cargo timings, excluding the separate cargo-chef installation). Cargo's reported Clippy and test build times fell from 65.35 to 53.84 seconds combined; the Rust job now also runs the broker test, previously part of Compose.

| Cache | Hits cold / warm | Observed stored size | Restore cold / warm | Save cold / warm |
| --- | --- | --- | --- | --- |
| Rust workspace | 0/1 / 1/1 exact | 196.4 MiB | 1 s miss / 5 s | 17 s / skipped |
| npm downloads | 0/1 / 1/1 exact | 76.0 MiB | <1 s miss / 1 s | 2 s / skipped |
| BuildKit | 0/4 / 4/4 compile vertices | 29.5 KiB benchmark indexes across both attempts; 1.61 GiB of shared BuildKit blobs on `main` | Manifest import 0.1 s + <0.1 s in both runs | 77.2 s + 44.3 s / 0.6 s + 0.6 s |

GitHub does not attribute shared blob bytes to individual BuildKit scopes in the cache listing. The manifest timer excludes lazy layer fetches; the warm image steps bound the combined restore/load cost. Ordinary PRs do not pay the trusted cache export cost. Track cold export cost and blob growth before adding more image scopes.

### Repeating the benchmark

Dispatch `CI` on the reviewed revision with a fresh benchmark namespace, then rerun the successful run at the same SHA. Rotate the benchmark namespace rather than deleting shared caches. Run sequentially, confirm the first run misses all three caches, and record the runner image versions from job setup.

For each run, record start-to-finish time, every job's duration, and sum of job durations. Inspect named job steps with `gh run view <id> --json jobs` and use `gh run view <id> --log` for cache and compile timings. Record exact and fallback Rust/npm restores separately; for each image count cached `cargo chef cook` and final `cargo build` vertices out of the two eligible vertices, not merely whether the cache index imported. Record the BuildKit import/export durations and the Rust/npm restore/save step durations. Use `gh cache list --ref refs/heads/main --limit 100 --json key,sizeInBytes` to inventory the namespace before and after, including BuildKit layer blobs if visible; mark any unobservable metric unavailable rather than zero. Compare matching compile vertices and Rust steps, not total wall time, to estimate compilation avoided. Report the cold run's export overhead separately from read-only PRs.

`sccache` is deferred: it would add another storage policy and cache transfer path before the existing Rust and Docker caches have been measured. `cargo-nextest` is also deferred: current evidence shows compilation and Compose dominate, and switching the runner would still require retaining doc tests, ignored broker tests, and existing semantics. Revisit both only if measurements show a remaining bottleneck they can reduce.
