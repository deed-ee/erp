# erp4us

![erp4us mark: connected modules forming a four](docs/community/erp4us-mark.svg)

erp4us aims to make business software composable: organizations should be able to run their own ERP and combine independently maintained business services without handing over their data or depending on any particular cloud or AI provider. People and agents operate on the same business commands, under the same authorization and audit rules. An open protocol and semantic models make those commands discoverable without exposing a service's database.

**Status: foundation, not an ERP release.** The current local-only slice registers an article by SKU and retrieves it from MongoDB. It does not yet have real tenant authentication, a web UI, a broker, or cross-service business workflows. Do not deploy it publicly or enter real business data. [Licensing is pending](docs/community/licensing.md).

## Run the first slice

Prerequisites: Docker Engine 28+ or Docker Desktop with Compose v2, `openssl`, and Python 3 for the smoke test. From the repository root:

```sh
sh scripts/init-dev.sh
docker compose up --build -d
python3 tests/integration/smoke.py --restart
```

The script waits for readiness, registers a synthetic article, verifies authorization and validation failures, restarts the service, and retrieves the same article. The command and response are documented in [OpenAPI](contracts/openapi/master-data-v1.yaml). The API listens on `127.0.0.1:8080`; set `ERP4US_PORT` in `.env` if needed. Stop with `docker compose down`; it preserves the local database. `docker compose down -v` **deletes** the demo data. Keep `.env` with the volume: regenerating it does not rotate existing MongoDB credentials.

Use the pinned Rust toolchain for `cargo test --locked`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`. For contracts, install `tests/contracts/requirements.txt` in a Python virtual environment and run `python tests/contracts/check.py`.

## Help shape the first workflow

- **Try the quickstart above.** Could you register and retrieve an article on your machine? If a step failed, [report where you got stuck](https://github.com/deed-ee/erp4us/issues/new?template=bug.md) with your OS, Docker version and the first failing step. Leave out credentials and use synthetic data.
- **Bring an ERP example.** Suppose a customer orders ten units but only six are available. What should the salesperson see, and who decides what happens to the remaining four? Add an anonymized example and the outcome you would expect to the [order-to-procurement discussion in issue #16](https://github.com/deed-ee/erp4us/issues/16).

Feedback is welcome now. Please hold code contributions until the [license and contribution terms](https://github.com/deed-ee/erp4us/issues/1) are settled.

## Get oriented

- [Architecture and service boundaries](docs/architecture/baseline.md) · [Threat model](docs/architecture/threat-model.md) · [Decisions](docs/adr/0001-independent-services.md)
- [Open Business Service Protocol and semantic model](docs/specifications/business-service-protocol.md) · [Roadmap](ROADMAP.md)
- [Contributing](CONTRIBUTING.md) · [Working rules for coding agents](AGENTS.md) · [Governance](GOVERNANCE.md) · [Security reporting](SECURITY.md)
