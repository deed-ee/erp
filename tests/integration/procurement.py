"""Run the shortage and buyer-query fixtures as separate Procurement processes."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
from uuid import UUID, uuid4

from fixture_mongo import fixture_replica_set


shortage = json.loads(Path('contracts/fixtures/stock-shortage-v1.json').read_text())
query = {'contract': 'get-draft-proposal.v1', 'shortageId': shortage['shortageId']}

with fixture_replica_set('procurement') as (uri, mongo, internal_uri):
    env = dict(os.environ, PROCUREMENT_MONGO_URI=uri, PROCUREMENT_TENANT_ID=str(uuid4()),
               PROCUREMENT_ORGANIZATION_ID=str(uuid4()))

    def run(data, environment=env, error=None):
        result = subprocess.run(['target/debug/procurement'], input=json.dumps(data),
                                env=environment, text=True, capture_output=True, timeout=60)
        if error is None:
            assert result.returncode == 0, result.stderr
            return json.loads(result.stdout)
        assert result.returncode != 0 and result.stderr.strip() == error, result.stderr

    assert run(query) is None
    covered = dict(shortage, shortageId=str(uuid4()), missingQuantity=0)
    assert run(covered) is None
    assert run(dict(query, shortageId=covered['shortageId'])) is None

    with ThreadPoolExecutor(max_workers=2) as pool:
        first, duplicate = pool.map(run, [shortage, shortage])
    assert first == duplicate == run(query) == run(shortage)
    assert UUID(first['proposalId']).version == 4
    assert first['proposalId'] != first['shortageId']
    assert first == {
        'proposalId': first['proposalId'], 'status': 'draft', 'quantity': 4,
        **{key: shortage[key] for key in ('shortageId', 'orderId', 'lineId', 'articleId', 'unit')},
    }

    run(dict(shortage, missingQuantity=8), error='shortage reference already used with different details')
    run(dict(shortage, unit='BOX'), error='invalid Procurement input')
    run(dict(query, tenantId=env['PROCUREMENT_TENANT_ID']), error='invalid Procurement input')
    for scope_key in ('PROCUREMENT_TENANT_ID', 'PROCUREMENT_ORGANIZATION_ID'):
        assert run(query, dict(env, **{scope_key: str(uuid4())})) is None
    assert run(query) == first
    mongo(f'''const dbp = new Mongo("{internal_uri}").getDB("procurement");
        if (dbp.drafts.countDocuments({{}}) !== 1) throw Error("expected exactly one persisted draft");''')

print('Procurement draft, traceability, no-shortage, conflicting replay and scope checks passed')
