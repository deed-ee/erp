# ERP4us

![erp4us mark: connected modules forming a four](docs/community/erp4us-mark.svg)

**ERP by everyone, for everyone.** ERP4us is a community-driven, open-source, modular, cloud-native and agent-native ERP and business application platform. Organizations can self-host independently maintained services without depending on any particular cloud or AI provider. People and agents use the same commands, authorization and audit rules. Open contracts make those commands discoverable without exposing a service's database.

The goal is one platform for many industries, not one ERP fork per industry. A services business should not need inventory, and a process manufacturer should not have to model every transformation as an assembly. Explicit business capabilities and versioned industry packages extend shared concepts without weakening their invariants. See the [business domain model](docs/architecture/business-domain.md) and [public requirements catalog](docs/requirements/README.md) for what is committed, planned or already delivered.

**Status: foundation, not an ERP release.** The current local-only slice registers an article by SKU and retrieves it from MongoDB. It does not yet have real tenant authentication, a web UI, a broker, or cross-service business workflows. Do not deploy it publicly or enter real business data. Project materials are licensed under [Apache-2.0](LICENSE).

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

Feedback and focused pull requests are welcome. See [Contributing](CONTRIBUTING.md) for the checks and commit sign-off.

## Get oriented

- [Architecture and service boundaries](docs/architecture/baseline.md) · [Threat model](docs/architecture/threat-model.md) · [Architecture review and decisions](docs/architecture/cross-industry-review.md)
- [Open Business Service Protocol and semantic model](docs/specifications/business-service-protocol.md) · [Roadmap](ROADMAP.md)
- [Contributing](CONTRIBUTING.md) · [Governance](GOVERNANCE.md) · [Security reporting](SECURITY.md)
