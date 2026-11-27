# Rehearse cutover and fallback for order entry

[Issue #110](https://github.com/erp4us-org/erp4us/issues/110) proposes moving order entry for one branch without losing accepted orders. This runbook separates two decisions: whether the evidence supports switching forward, and whether it supports falling back. It is a proposed pilot procedure, not a production routing mechanism or an approval policy.

## Agree the scope before the exercise

Write down the tenant, organization, business process, source namespace, current authoritative writer and designated approver. For Demo Trading the scope is the order-entry process of one synthetic branch. The other branches stay on the legacy ERP. Decide separately who owns shipping and invoicing; do not enable those effects as part of switching order entry. The pilot compares exact normalized amounts and quantities with zero unresolved discrepancies. Any different tolerance or routing policy needs a separate decision.

Record the release and export contract versions, starting and final checkpoint, mapping revision, agreed order selection, where the two independently produced extracts came from, and the reviewer decision. Store evidence in a restricted workspace. A matching report does not on its own prove the exports are complete or the actor authorized.

## Forward rehearsal

1. **Keep one writer.** The legacy ERP is authoritative for this branch until final pause. The erp4us shadow copy produces no orders, notifications, invoices, shipments or supplier commitments. Record the unaffected branch's route and verify it before and after the exercise.
2. **Pause at the boundary.** Stop accepting source writes for the selected scope; drain in-flight requests, including timed-out requests that may have committed. Record the final source checkpoint. Preview the last change window and apply it to the [shadow copy](legacy-order-shadow.md). An old checkpoint cannot stand in for the final checkpoint.
3. **Reconcile independent exports.** Run the [order comparison](order-reconciliation.md). Test a missed cancellation at the final checkpoint: the order is still open in one export, so hold the cutover. Investigate and repeat. A matching result removes this data blocker only.
4. **Decide explicitly.** The operator demonstrates that the old writer is disabled for the scope, no other branch changed, and the fallback decision below is understood. Only an authorized rollout decision can permit enabling the target writer. If a control cannot be demonstrated, keep the target disabled. The local tools neither pause a service nor change routing.

## Fallback rehearsal

| When fallback is requested | Required evidence before re-enabling the legacy writer |
| --- | --- |
| Before the first accepted erp4us write | Stop target admission; drain in-flight commands and demonstrate there were no accepted writes or external effects, including requests whose caller timed out. Verify the legacy state is still valid for the scope and record the decision. |
| After erp4us accepted an order | Stop new target writes and capture the exact post-switch interval and its accepted order references, including later changes or cancellations. Agree how to transfer or otherwise recover those writes. Compare the expected recovered state against an independent export of the legacy recovery area. No missing, unexpected or duplicate orders; corresponding amounts, units, quantities and status must match. If recovery cannot account for an accepted order, keep legacy writes disabled pending explicit recovery. |

In the synthetic handback check, `SO-900` was accepted only on the erp4us side. The old snapshot matches before `SO-900`, but the same snapshot is unsafe afterward: reconciliation reports `missingOrder`. Adding one recovered copy removes the data blocker; adding two makes the input invalid. The comparison's `legacy` role means *expected reference* and `candidate` means *observed result* for this handback example. Neither role changes the authoritative writer in a real system.

Run the evidence walkthrough with `python3 -m unittest discover -s tests/migration -p 'test_cutover_walkthrough.py' -v`. The tests use synthetic snapshots and a disposable local shadow database to exercise stale input, forward mismatch, and fallback accounting. They do **not** demonstrate real ingress fencing, authorization, routing isolation or suppression of external effects in a deployed ERP. Those checks and an approved operating procedure remain part of #110.
