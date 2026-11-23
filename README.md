# erp4us

![erp4us mark: connected modules forming a four](docs/community/erp4us-mark.svg)

erp4us aims to make business software composable: organizations should be able to run their own ERP and combine independently maintained business services without handing over their data or depending on any particular cloud or AI provider. People and agents operate on the same business commands, under the same authorization and audit rules. An open protocol and semantic models make those commands discoverable without exposing a service's database.

**Status: foundation, not an ERP release.** Master Data registers and retrieves scoped articles from MongoDB. It verifies OAuth access tokens and enforces operator-managed tenant and organization grants. It does not yet have a tenant administration service, web UI, broker, or cross-service business workflows. Do not deploy the Compose fixture publicly or enter real business data. [Licensing is pending](docs/community/licensing.md).

## Run the first slice

Prerequisites: Docker Engine 28+ or Docker Desktop with Compose v2, `openssl`, and Python 3 for the smoke test. From the repository root:

```sh
sh scripts/init-dev.sh
python3 tests/integration/identity_fixture.py
docker compose up --build -d
python3 tests/integration/smoke.py --restart
```

The fixture generates a machine-local test signing key and synthetic grants, not an identity provider. The smoke test exercises token verification, scope isolation and service restart. The request contract is in [OpenAPI](contracts/openapi/master-data-v1.yaml); see [Identity integration](docs/development/identity.md) for a self-hosted provider configuration. The API listens on `127.0.0.1:8080`; export `ERP4US_PORT` before starting Compose and running the smoke test to use a different port. `docker compose down` preserves the database; `docker compose down -v` **deletes** it. Keep `.env` with the volume: regenerating it does not rotate existing MongoDB credentials.

Use the pinned Rust toolchain for `cargo test --locked`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`. For contracts, install `tests/contracts/requirements.txt` in a Python virtual environment and run `python tests/contracts/check.py`.

## Help shape the first workflow

- **Try the quickstart above.** Could you register and retrieve an article on your machine? If a step failed, [report where you got stuck](https://github.com/deed-ee/erp4us/issues/new?template=bug.md) with your OS, Docker version and the first failing step. Leave out credentials and use synthetic data.
- **Bring an ERP example.** Suppose a customer orders ten units but only six are available. What should the salesperson see, and who decides what happens to the remaining four? Add an anonymized example and the outcome you would expect to the [order-to-procurement discussion in issue #16](https://github.com/deed-ee/erp4us/issues/16).

Feedback is welcome now. Please hold code contributions until the [license and contribution terms](https://github.com/deed-ee/erp4us/issues/1) are settled.

## Get oriented

- [Architecture and service boundaries](docs/architecture/baseline.md) · [Threat model](docs/architecture/threat-model.md) · [Decisions](docs/adr/0001-independent-services.md)
- [Open Business Service Protocol and semantic model](docs/specifications/business-service-protocol.md) · [Roadmap](ROADMAP.md)
- [Contributing](CONTRIBUTING.md) · [Governance](GOVERNANCE.md) · [Security reporting](SECURITY.md)
