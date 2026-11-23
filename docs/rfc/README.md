# Requirements decision queue

Use the existing [RFC process](../../GOVERNANCE.md#rfc-process). A research ticket requests evidence; it does not authorize implementation. Each recommendation must identify who decides, two viable policies, their effects on two contrasting companies, migration/compatibility, access and history risk, the smallest falsifying test and what would justify deferral. A reviewer records the disposition in Git; accepted architecture decisions get a linked ADR.

| Question | Example that should distinguish options | Decision owner for review (not an appointment) |
|---|---|---|
| [Sharing and overrides #54](https://github.com/erp4us-org/erp4us/issues/54) | G1 shared, company-local price/access hidden from a sibling | Organizations and Master Data maintainers with company stewards |
| [Retroactive facts #55](https://github.com/erp4us-org/erp4us/issues/55) | Address entered today, valid last week, invoice already sent | Record owner and commercial document owner |
| [Conversions #56](https://github.com/erp4us-org/erp4us/issues/56) | Fixed pack versus measured timber volume and mass | Unit steward and Inventory posting owner |
| [Translation review #57](https://github.com/erp4us-org/erp4us/issues/57) | Source revised while a translation candidate is awaiting approval | Resource owner and localization reviewers |
| [Partial reservations #58](https://github.com/erp4us-org/erp4us/issues/58) | Two buyers, six available, ten promised; service-only line mixed in | Sales and Inventory owners with sales/warehouse practitioners |
| [Business numbers #59](https://github.com/erp4us-org/erp4us/issues/59) | Cancelled draft and simultaneous confirmations in two entities | Document owner with local finance reviewer |

Do not reopen ADR 0002 to choose a different messaging platform. The outstanding question is how long replay evidence is kept and how its effects are reconciled. Likewise, ADR 0001 decides service ownership, not the master-data override policy.
