# First Working ERP integration

Tracking: [#119](https://github.com/erp4us-org/erp4us/issues/119). Integration branch: `milestone/first-working-erp`. This work is not a release, production deployment, or claim that the full NovaMotion process is ready.

## Verified baseline

`main` at `9be1eab` already includes the scoped Master Data access-token verifier (#15), the versioned messaging library and Article relay/configuration reference (#97/#98), separately owned Inventory/Procurement fixture adapters (#95/#96/#106), and the Angular shell. The CAD/PLM example (#118) is a test contract, not a live BOM service. None of the remaining open source PRs supplied a production Sales, Inventory, Manufacturing, Shipment, or diagnostics implementation; #28/#31 are contributor/requirement work and #78 is an overlapping draft CI baseline. Leave their original review and milestone assignments intact.

## Current integration: NovaMotion manufacturing structures

Manufacturing owns its own MongoDB collection and credential. Each preview uses an explicit product and BOM revision, traverses pinned subassembly revisions, and computes exact whole-piece quantities with checked arithmetic. The primary and secondary demo structures are inserted by an explicit idempotent seed command; nothing is seeded by the normal server. Master Data and Manufacturing reuse the same static OIDC verification library with explicit owning-service grants. Browser scope selection uses verified memberships, not claims read by the frontend. No NATS event is emitted for the read-only preview. Cross-service commands will use the existing wire and outbox conventions, not this preview API as a write path.

With Docker Compose, Python, and the Node version in `ui/.nvmrc`, use an isolated test project:

```sh
sh scripts/init-dev.sh
python3 tests/integration/identity_fixture.py
docker compose -p erp4us-manufacturing -f compose.yaml -f compose.manufacturing.yaml up --build -d
docker compose -p erp4us-manufacturing -f compose.yaml -f compose.manufacturing.yaml run --rm manufacturing-seed
python3 tests/integration/manufacturing.py
```

The synthetic issuer is only for automated fixture tests, not user authentication. Run `npm ci` then `npm start` from `ui/`; `ERP4US_MANUFACTURING_E2E=1 npm run test:e2e` exercises the planning view through the running API. The [manufacturing contract](../../contracts/openapi/manufacturing-v1.yaml) documents the two scoped operations. Stop the isolated project with its same Compose files and project name. No migration touches existing Master Data, Inventory or Procurement state.

## Remaining acceptance gates

- Real provider/browser sign-in, permission-aware workflows, and editable tenant, organization, warehouse and role administration.
- Scoped draft orders and confirmation, live stock and reservations, shortage drafts, purchasing and partial receipts.
- Durable production orders/material consumption, deliveries and stored PDF, process timeline, and business recovery evidence.
- Restricted diagnostics/collector, actual health/heartbeat status and logs. Do not grant runtime access to any UI, gateway or business container.
- Generate the ten-unit quantitative evidence and full browser test on a clean launch before a final maintainer review or publishing any claims.
