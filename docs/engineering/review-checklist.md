# Change review

Review the actual diff and evidence, not just the summary. Record what was exercised and what remains unverified.

1. **Functional correctness:** What does the actor accomplish? Do precision, transitions, explicit permissions and adverse inputs preserve owner invariants?
2. **Architecture and scope:** Is there already a comparable implementation? Did this duplicate business logic or infrastructure, violate dependency direction, increase coupling, or introduce unrelated work?
3. **Security and isolation:** Are credentials and untrusted inputs handled safely? Can people, integrations or agents read or change another tenant or organization? Are dependencies, logs and failures redacted?
4. **Contract compatibility:** Have version bounds, operation and permission IDs, errors, rollout, migration and rollback been preserved or deliberately changed with approval?
5. **Maintainability:** Are names, responsibilities, failure paths and state transitions understandable? Is complexity necessary? Do not debate formatting already handled by tools.
6. **Operations and performance:** Are errors diagnosable without payloads? Are queues, retries, reads and work bounded? Is any cost or runtime regression measured?
7. **Verification and docs:** Do positive and negative assertions prove the behavior, including durable effects when relevant? Were related checks actually run? Are blocked checks identified and evidence linked? Are requirements, docs and references current?

Report each finding with path/line, consequence and a proposed fix or proof:

- **Blocking:** cannot merge (security/correctness boundary, breaking change without approval, or required evidence missing).
- **Major:** substantial risk to maintainability, quality or behavior; resolve or get a recorded maintainer decision before merge.
- **Minor:** focused improvement; fix or track if deliberately deferred.
- **Suggestion:** optional improvement.

Prefer independent review for large or security-critical changes. A second agent is useful for different evidence, not as approval. Accepted architecture, security, governance and release decisions still require the [existing maintainer process](../../GOVERNANCE.md).
