# Contributing

Start with the [README](README.md), [roadmap](ROADMAP.md) and [architecture](docs/architecture/baseline.md). Reproduce bugs using synthetic data; open a focused issue with steps, expected behavior and version. Keep public discussion and code in English. No AI tooling is required.

Follow the [engineering standards](docs/engineering/README.md) and [Definition of Done](docs/engineering/agent-workflow.md#definition-of-done). Keep changes focused, add tests for behavior, run the applicable [local checks](docs/engineering/quality-gates.md) and document migration/rollback and contract impacts. Use a pull request and request review rather than pushing to the default branch. For architecture or protocol changes, follow the RFC process in [GOVERNANCE.md](GOVERNANCE.md). Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md). Follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Sign off your contributions

Read the [Developer Certificate of Origin 1.1](https://developercertificate.org/). By signing off, you certify that you have the right to submit the contribution under the project's [Apache-2.0 license](LICENSE). Use `git commit -s` on each new contribution commit to add `Signed-off-by: Your Name <your-email>` to its message. Only sign for yourself. For a missing sign-off on your own latest commit, use `git commit --amend --no-edit -s`.

## CI workflow

See the [CI guide](docs/development/ci.md) for required checks, change detection, cache policy and full-system benchmark instructions.
