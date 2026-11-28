"""Check the synthetic receiver policy without starting an IdP or changing any preview realm."""

import importlib.util
import json
import os
from pathlib import Path
import tempfile
from unittest.mock import patch


def identity(budget=False, issuance=False, receiving=False):
    spec = importlib.util.spec_from_file_location('preview_identity', Path('scripts/preview-identity.py'))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    with patch.dict(os.environ, {
        'ERP4US_PREVIEW_BUDGET_APPROVALS': str(int(budget)),
        'ERP4US_PREVIEW_PURCHASE_ISSUANCE': str(int(issuance)),
        'ERP4US_PREVIEW_RECEIVING': str(int(receiving)),
    }):
        spec.loader.exec_module(module)
    return module


def main():
    default = identity(budget=True, issuance=True)
    assert 'purchase-receiver' not in default.USERS
    fixture = identity(budget=True, issuance=True, receiving=True)
    assert len(set(fixture.USERS.values())) == len(fixture.USERS)
    grants = fixture.identity_policy()['principals']
    receiver = fixture.USERS['purchase-receiver']
    finance = [
        fixture.USERS[name]
        for name in ('production', 'department-approver', 'budget-approver', 'purchase-issuer')
    ]
    assert all(grants[actor] == default.identity_policy()['principals'][actor] for actor in finance)
    assert grants[receiver] == {
        'actor_id': receiver,
        'grants': [fixture.grant(fixture.NOVA, [
            'sales.order.read', 'inventory.reservation.read', 'inventory.material-plan.read',
            'procurement.purchase-order.receipt-source.read',
            'inventory.purchase-receipt.read', 'inventory.purchase-receipt.post',
        ])],
    }

    with tempfile.TemporaryDirectory() as directory:
        fixture.DEMO = Path(directory) / 'disposable-preview'
        fixture.prepare()
        credentials = fixture.DEMO / 'credentials.json'
        policy = fixture.DEMO / 'identity-policy.json'
        realm = fixture.DEMO / 'realm/erp4us-preview-realm.json'
        assert {user['id'] for user in json.loads(realm.read_text())['users']} == set(fixture.USERS.values())
        originals = [(path, path.read_bytes()) for path in (credentials, policy, realm)]
        default.DEMO = fixture.DEMO
        try:
            default.prepare()
        except SystemExit as error:
            assert 'accounts differ' in str(error)
        else:
            raise AssertionError('an existing realm must not be changed to a different role set')
        assert all(path.read_bytes() == before for path, before in originals)

        incomplete = identity(receiving=True)
        incomplete.DEMO = Path(directory) / 'missing-prerequisite'
        try:
            incomplete.prepare()
        except SystemExit as error:
            assert 'requires both' in str(error)
        else:
            raise AssertionError('receiving must not skip purchasing approval and issuance')
        assert not incomplete.DEMO.exists()


if __name__ == '__main__':
    main()
