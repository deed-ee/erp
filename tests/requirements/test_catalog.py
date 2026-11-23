import unittest
from jsonschema.exceptions import ValidationError
from check import DOC, catalog, check_link, validate


class CatalogFailures(unittest.TestCase):
    def test_dangling_and_cyclic_dependencies(self):
        data = catalog()
        a, b = data['records'][:2]
        a['dependencies'] = ['ORG-999']
        with self.assertRaisesRegex(ValueError, 'missing dependency'):
            validate(**data)
        a['dependencies'], b['dependencies'] = [b['id']], [a['id']]
        with self.assertRaisesRegex(ValueError, 'cycle'):
            validate(**data)

    def test_unsubstantiated_verification_and_wrong_capability(self):
        data = catalog()
        r = data['records'][0]
        r.update(status='verified', evidence=[])
        with self.assertRaises(ValidationError):
            validate(**data)
        r.update(status='proposed', capability='ZZ.01')
        with self.assertRaisesRegex(ValueError, 'unknown capability'):
            validate(**data)

    def test_unknown_links_and_internal_fragment(self):
        with self.assertRaisesRegex(ValueError, 'invalid internal path'):
            check_link(DOC / 'requirements/guide.md', '../imaginary.md')
        with self.assertRaisesRegex(ValueError, 'unknown fragment'):
            check_link(DOC / 'requirements/guide.md', '../../GOVERNANCE.md#imaginary')

    def test_incomplete_process_card(self):
        data = catalog()
        process = data['processes'][0]
        for field in ('alternative', 'exception', 'objects', 'result'):
            with self.subTest(field=field):
                value = process.pop(field)
                try:
                    with self.assertRaisesRegex(ValueError, f'{process["id"]}: incomplete process card'):
                        validate(**data)
                finally:
                    process[field] = value

    def test_incomplete_scenario(self):
        data = catalog()
        scenario_id = next(iter(data['scenarios']))
        data['scenarios'][scenario_id]['then'] = []
        with self.assertRaisesRegex(ValueError, f'{scenario_id}: incomplete scenario'):
            validate(**data)

    def test_duplicate_id_unknown_phase_and_empty_acceptance(self):
        data = catalog()
        a, b = data['records'][:2]
        old_id = b['id']
        b['id'] = a['id']
        with self.assertRaisesRegex(ValueError, 'duplicate requirement'):
            validate(**data)
        b.update(id=old_id, milestone='M99')
        with self.assertRaisesRegex(ValueError, 'unknown milestone'):
            validate(**data)
        b.update(milestone='M1', acceptance=[])
        with self.assertRaises(ValidationError):
            validate(**data)
