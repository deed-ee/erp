# Shared terms for boundary reviews

Use these meanings when comparing proposals. This is an engineering vocabulary, not a legal definition or the introductory storytelling task in [#33](https://github.com/erp4us-org/erp4us/issues/33).

| Term | Meaning and boundary |
|---|---|
| Tenant | Top-level isolation boundary. Never infer membership from a supplied ID. A tenant can contain several legal entities. |
| Company | A business-facing name; qualify as *legal entity*, *operating unit* or *group* before using it as a scope. |
| Legal entity | Party legally responsible for a contract or posting. Organizations owns the reference; Finance owns its books. |
| Site / warehouse | Site is an operating location. Warehouse is an Inventory-controlled stock boundary, possibly linked to a site; neither implies legal ownership. |
| Product / article / item | Product is a reusable offering, physical, digital or service. Article is the current Master Data API term. Disambiguate *item* as product definition, document line or physical instance. |
| Material | A product acting as a measured input or output; not a mandatory superclass of services. |
| Product instance / asset | An instance is one identifiable realization of a definition. An asset is something managed for custody, use or value; not all instances are managed assets. |
| Lot / batch | Lot is a traceable grouping of material. Batch may mean a production run or its output lot: name which one. Splits and mixtures break any presumed one-to-one mapping. |
| Serial number | A scoped identifier assigned to one instance under an issuing scheme. It is not the internal ID or a quantity. |
| Business partner / customer / supplier | Master Data owns the party identity. Customer and supplier are separately effective roles, not mutually exclusive entity classes. |
| Order | Qualify as customer sales order, supplier purchase order or work order. Each creates a different commitment, owned by a different context. |
| Reservation | A commitment of eligible availability to demand, not a shipment, purchase or transfer of ownership. A rental time slot is a different availability invariant. |
| Transaction / posting | Distinguish a business exchange from an atomic database transaction. A posting is an attributed entry in a domain ledger; reverse it with a linked entry. |
| Valid time / recorded time | When a fact applies versus when it became known. Preserve both for backdated rates, addresses or policies that affect obligations; a draft label need not be bitemporal. |
| Capability / context / deployment | A business outcome, the boundary that owns an invariant, and an operable software unit respectively. A capability map is not an infrastructure inventory. |
