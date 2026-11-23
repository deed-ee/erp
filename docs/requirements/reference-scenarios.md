# Cross-industry architecture scenarios

These are manual architecture checks and future test specifications, not passing integration tests. Use anonymized or synthetic records. For an RFC, name the affected scenarios and explain which boundary owns each decision and which evidence survives changes, replay or denial. Automate only the checks supported by a delivered slice.

## SC-01

**Mechanical engineering.** Configure an engineer-to-order machine, release a design revision, build and assemble serialized components, deliver it and record a maintenance replacement. The delivered record still references the built revision and serials after a new revision is released; the maintained view identifies the replacement and its documentation without rewriting the as-built evidence.

## SC-02

**Sawmill.** Receive and classify roundwood, recording dimensions, volume, weight, moisture and quality. Transform one lot into graded timber, chips and sawdust, then split and ship a timber lot. Compare actual quantities with planned yield without treating a moisture-dependent conversion as global. Given a shipment, find contributing lots; given a source lot, find affected shipments and customers even after broker retention expires.

## SC-03

**Chemical production.** Use a synthetic formula fixture with input lots, recorded concentrations, actual quantities, process parameters and output lots. Release a revised definition after execution and verify the prior run still resolves its original revision, quality results, expiry and safety document. Failed inspection keeps an affected lot unavailable until authorized release. Verify downstream recall scope.

## SC-04

**Construction.** Track a project with a bill of quantities, work packages, subcontractors, materials, labor, equipment and multiple sites. Post a progress measurement and partial invoice against the applicable scope and contract revision. Reconcile project costs and navigate from completed work to installed assets. Site assignments do not determine the invoicing legal entity.

## SC-05

**Retail and wholesale.** Receive cases at two warehouses and sell in a different unit using a product-specific pack ratio, external identifiers and a dated price list. Reserve and ship available stock, then reverse a returned quantity without rewriting the ledger. For a shortage, verify an authorized purchase and receipt feed the waiting order without duplicate reservations on replay. The first order MVP exercises a single warehouse, not the entire scenario.

## SC-06

**Professional services.** Sell a service, assign qualified staff, record time and expenses against budget and invoice delivery against the agreement. No physical inventory balance, reservation or production order is required. The order MVP checks that its contracts leave this path open; workforce and billing delivery are later work.

## Shared checks

- Add a typed classification or package without forking Master Data; verify two packages coexist and invalid configuration does not bypass a command invariant. An old record still resolves the policy revision used at the time.
- Attempt a relationship traversal, attribute search and export as an actor outside the record's organization or tenant. Return neither hidden entities nor evidence of them via counts.
- Change a translation's source revision, retry a provider job, and verify approved text is preserved. Deny a prohibited provider transfer before execution; verify normal operation without a provider.
