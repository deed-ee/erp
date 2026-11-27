from copy import deepcopy
from graphlib import CycleError, TopologicalSorter
import json
from pathlib import Path
import unittest
from uuid import uuid4

from jsonschema import Draft202012Validator, FormatChecker, ValidationError


CONTRACTS = Path(__file__).resolve().parents[3] / "contracts/plm"
SCHEMA = json.loads((CONTRACTS / "released-assembly-exchange-draft-v1.json").read_text())


class IncompatibleBundle(ValueError):
    """A semantic constraint of the proposed fixture contract is not met."""


def design_key(design):
    return design["assemblyRef"], design["revision"]


class ExampleReceiver:
    """In-memory contract oracle, not the ERP importer or an authority model."""

    def __init__(self, allowed_context, catalog):
        self.context = allowed_context
        self.catalog = catalog
        self.assemblies = {}
        self.units = {}

    def check_bundle(self, bundle):
        if self.context != tuple(bundle.get(key) for key in ("tenantId", "organizationId", "sourceSystem")):
            raise IncompatibleBundle("not the selected organization and source")
        Draft202012Validator(SCHEMA, format_checker=FormatChecker()).validate(bundle)
        assemblies, units = self.assemblies.copy(), self.units.copy()
        batch_keys = set()
        for assembly in bundle["assemblies"]:
            key = design_key(assembly)
            if key in batch_keys:
                raise IncompatibleBundle("duplicate assembly revision in batch")
            batch_keys.add(key)
            if assembly["articleRef"] not in self.catalog:
                raise IncompatibleBundle("unmapped assembly article")
            positions = {line["position"] for line in assembly["components"]}
            if len(positions) != len(assembly["components"]):
                raise IncompatibleBundle("duplicate component position")
            normalized = {**assembly, "components": sorted(assembly["components"], key=lambda row: row["position"])}
            if key in assemblies and assemblies[key] != normalized:
                raise IncompatibleBundle("released revision changed in place")
            assemblies[key] = normalized

        graph = TopologicalSorter()
        for key, assembly in assemblies.items():
            graph.add(key)
            for component in assembly["components"]:
                units_for_part = self.catalog.get(component["articleRef"])
                if units_for_part is None:
                    raise IncompatibleBundle("unmapped component")
                if component["unit"] not in units_for_part:
                    raise IncompatibleBundle("unexpected component unit")
                if "structure" in component:
                    child = design_key(component["structure"])
                    if child not in assemblies:
                        raise IncompatibleBundle("missing nested assembly revision")
                    if assemblies[child]["articleRef"] != component["articleRef"]:
                        raise IncompatibleBundle("nested article does not match structure")
                    graph.add(key, child)
        try:
            graph.prepare()
        except CycleError:
            raise IncompatibleBundle("cyclic assembly structure") from None

        serial_keys = {
            tuple(unit["serial"][key] for key in ("issuer", "namespace", "value")): ref
            for ref, (_, unit) in units.items()
        }
        batch_keys = set()
        for unit in bundle["units"]:
            ref = unit["sourceUnitRef"]
            if ref in batch_keys:
                raise IncompatibleBundle("duplicate unit in batch")
            batch_keys.add(ref)
            if design_key(unit["design"]) not in assemblies:
                raise IncompatibleBundle("missing unit design revision")
            serial = tuple(unit["serial"][key] for key in ("issuer", "namespace", "value"))
            if serial in serial_keys and serial_keys[serial] != ref:
                raise IncompatibleBundle("serial already bound to another unit")
            if ref in units:
                if units[ref][1] != unit:
                    raise IncompatibleBundle("source unit binding changed")
                continue
            units[ref] = (str(uuid4()), unit)
            serial_keys[serial] = ref

        self.assemblies, self.units = assemblies, units

    def lookup(self, context, source_unit_ref):
        if context != self.context or source_unit_ref not in self.units:
            return None
        internal_id, unit = self.units[source_unit_ref]
        return {"internalId": internal_id, "sourceUnitRef": source_unit_ref,
                "design": unit["design"], "asBuiltStatus": "unknown"}


class ReleasedAssemblyExchangeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        Draft202012Validator.check_schema(SCHEMA)

    def setUp(self):
        self.first = json.loads((CONTRACTS / "examples/released-assembly-a.json").read_text())
        self.next = json.loads((CONTRACTS / "examples/released-assembly-b.json").read_text())
        self.context = tuple(self.first[key] for key in ("tenantId", "organizationId", "sourceSystem"))
        self.receiver = ExampleReceiver(self.context, {
            "ENG-MACHINE": {"EA"}, "ENG-FRAME": {"EA"}, "ENG-FASTENER": {"EA"}, "ENG-PLATE": {"KG"},
        })

    def test_release_b_does_not_rewrite_unit_pinned_to_release_a(self):
        self.receiver.check_bundle(self.first)
        original = self.receiver.lookup(self.context, "unit-438")
        self.receiver.check_bundle(self.next)
        self.assertEqual(self.receiver.lookup(self.context, "unit-438"), original)
        self.assertEqual(original["design"], {"assemblyRef": "machine", "revision": "A"})
        self.assertEqual(original["asBuiltStatus"], "unknown")
        other = self.receiver.lookup(self.context, "unit-557")
        self.assertEqual(other["design"]["revision"], "B")
        self.assertNotEqual(other["internalId"], original["internalId"])
        self.assertNotIn(original["internalId"], ("ENG-MACHINE", "unit-438", "00042"))

    def test_replay_preserves_one_revision_and_unit_binding(self):
        self.receiver.check_bundle(self.first)
        before = self.receiver.lookup(self.context, "unit-438")
        self.first["assemblies"].reverse()
        self.first["assemblies"][1]["components"].reverse()
        self.receiver.check_bundle(self.first)
        self.assertEqual(self.receiver.lookup(self.context, "unit-438"), before)
        self.assertEqual(len(self.receiver.assemblies), 2)
        self.assertEqual(len(self.receiver.units), 1)

    def test_invalid_components_have_no_partial_effect(self):
        for change, error in (
            ({"articleRef": "UNKNOWN"}, IncompatibleBundle),
            ({"unit": "KG"}, IncompatibleBundle),
            ({"quantity": "0.000"}, ValidationError),
            ({"quantity": 1.5}, ValidationError),
        ):
            with self.subTest(change=change):
                bad = deepcopy(self.first)
                bad["assemblies"][0]["components"][1].update(change)
                with self.assertRaises(error):
                    self.receiver.check_bundle(bad)
                self.assertEqual(self.receiver.assemblies, {})
                self.assertEqual(self.receiver.units, {})

    def test_missing_reference_and_cross_revision_cycle_are_rejected(self):
        missing = deepcopy(self.first)
        missing["assemblies"][0]["components"][0]["structure"]["revision"] = "X"
        with self.assertRaisesRegex(IncompatibleBundle, "missing nested assembly revision"):
            self.receiver.check_bundle(missing)
        self.first["assemblies"][1]["components"].append({
            "position": "020", "articleRef": "ENG-MACHINE", "quantity": "1", "unit": "EA",
            "structure": {"assemblyRef": "machine", "revision": "A"},
        })
        with self.assertRaisesRegex(IncompatibleBundle, "cyclic"):
            self.receiver.check_bundle(self.first)
        self.assertEqual(self.receiver.assemblies, {})

    def test_conflicting_reuses_never_overwrite_an_existing_binding(self):
        self.receiver.check_bundle(self.first)
        altered = deepcopy(self.first)
        altered["assemblies"][0]["components"][1]["quantity"] = "7"
        with self.assertRaisesRegex(IncompatibleBundle, "changed in place"):
            self.receiver.check_bundle(altered)
        self.next["units"][0]["serial"]["issuer"] = "maker-a"
        with self.assertRaisesRegex(IncompatibleBundle, "serial already bound"):
            self.receiver.check_bundle(self.next)
        self.assertNotIn(("machine", "B"), self.receiver.assemblies)
        self.assertEqual(self.receiver.lookup(self.context, "unit-438")["design"]["revision"], "A")

    def test_foreign_organization_cannot_apply_or_lookup(self):
        self.receiver.check_bundle(self.first)
        tenant, _, source = self.context
        foreign = "40000000-0000-4000-8000-000000000002"
        self.assertIsNone(self.receiver.lookup((tenant, foreign, source), "unit-438"))
        self.next["organizationId"] = foreign
        with self.assertRaisesRegex(IncompatibleBundle, "not the selected organization"):
            self.receiver.check_bundle(self.next)
        self.assertNotIn(("machine", "B"), self.receiver.assemblies)
