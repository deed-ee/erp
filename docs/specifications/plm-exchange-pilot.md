# Proposed CAD/PLM exchange pilot

**Status: review draft for [#111](https://github.com/erp4us-org/erp4us/issues/111).** This is a language-neutral, versioned fixture contract with executable examples, not an ERP import endpoint or decision about product-data ownership. The pilot refines [BOM-001](../requirements/catalog/BOM.yaml), [PLM-001](../requirements/catalog/PLM.yaml), [INT-006](../requirements/catalog/INT.yaml) and [MDM-002](../requirements/catalog/MDM.yaml) using SC-01.

## Three distinct references

| Reference | Meaning in the fixture |
| --- | --- |
| Article key | A mapped Master Data reference, such as `ENG-MACHINE`. It defines a kind of product, not a physical instance. |
| Assembly and revision | In the named source and organization, `machine` at revision `A` has its own immutable component positions, quantities, units and explicitly pinned nested revisions. It is not a pointer to the latest release. |
| Physical unit | A stable source-unit reference (`unit-438`) and serial issued as `(maker-a, machine-series, 00042)`. The receiver would allocate its own internal ID independently of these values. Serial strings preserve leading zeros. |

The [draft schema](../../contracts/plm/released-assembly-exchange-draft-v1.json) assumes an adapter has mapped source part IDs to authorized target article keys. The fixture receiver has a small synthetic catalog only; no Master Data query, broker connection or database write is performed. A future receiver must bind tenant, organization and source to verified credentials before reading or writing any records. Payload fields are assertions to check, not permissions.

## Runnable examples

The [first bundle](../../contracts/plm/examples/released-assembly-a.json) supplies machine `A`, frame `A` and the first unit. The [second bundle](../../contracts/plm/examples/released-assembly-b.json) supplies machine `B`, still referring to frame `A`, and a second unit bearing the same printed serial from a different issuer. The first unit continues pointing to machine `A` and its recorded nested revision.

With the dependencies in `tests/contracts/requirements.txt` installed:

```sh
python3 -m unittest discover -s tests/contracts/plm -v
```

The example tests require that reordering and replaying a bundle changes neither unit identity nor revision count, that conflicting bindings and invalid component structures are rejected without applying part of the bundle, and that a foreign scope cannot access the example unit. Quantities are exact positive decimal strings in an explicit unit; no conversion or rounding is performed.

**Design is not build evidence.** The unit points to an engineering revision. Without measured build or installation records, its as-built state is unknown. An eventual product dossier can reference independent build, delivery, maintenance and document evidence without overwriting the earlier records. That work is outside this fixture.

## Decisions still needed before implementation

- Decide how the release authority is verified and how the target records approvals and corrections. A file claiming to contain a released revision is not proof that its issuer may release it.
- Review the example serial key (tenant, organization, issuer, namespace, printed value), including manufacturer-wide versus per-product serial namespaces, character encoding, aliases and cross-source mapping. The pilot demonstrates collision cases; it does not choose a company-wide uniqueness policy.
- Review who may map or retire article bindings, how import provenance is retained, and how release changes and unmatched parts are presented to operators. Define production authorization, durable replay and transfer size limits before shipping.

A real adapter and importer, production API or messaging contracts, and the full product dossier remain open. An adapter can start with file fixtures; exposing internal NATS is not required.
