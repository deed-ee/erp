import unittest
from jsonschema.exceptions import ValidationError
from check import catalog, validate


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
