"""Exercise independent Inventory processes against a disposable authenticated replica set."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
from uuid import uuid4

from fixture_mongo import fixture_replica_set


line = json.loads(Path('contracts/fixtures/confirmed-stock-line-v1.json').read_text())
tenant, org = str(uuid4()), str(uuid4())
with fixture_replica_set('inventory') as (uri, mongo, internal_uri):
    env = dict(os.environ, INVENTORY_MONGO_URI=uri, INVENTORY_TENANT_ID=tenant, INVENTORY_ORGANIZATION_ID=org)
    db = f'const dbi=new Mongo("{internal_uri}").getDB("inventory");\n'
    key = dict(tenantId=tenant, organizationId=org, warehouseId=line['warehouseId'], articleId=line['articleId'], unit='EA')

    def seed(stock_key):
        mongo(db + f'dbi.balances.insertOne({{_id:{json.dumps(stock_key)},available:NumberInt(6),expected:NumberInt(4)}})')

    def consume(data, environment=env, success=True):
        proc = subprocess.run(['target/debug/inventory'], text=True, input=json.dumps(data), env=environment,
                              capture_output=True, timeout=60)
        assert (proc.returncode == 0) == success, proc.stderr
        return json.loads(proc.stdout) if success else None

    seed(key)
    assert consume(line) == consume(line) == {'reserved': 6, 'outstanding': 4}
    consume(dict(line, quantity=12), success=False)
    consume(dict(line, tenantId=tenant), success=False)
    for field in ['INVENTORY_TENANT_ID', 'INVENTORY_ORGANIZATION_ID']:
        assert consume(line, dict(env, **{field: str(uuid4())})) == {'reserved': 0, 'outstanding': 10}
    race = dict(line, warehouseId=str(uuid4()), quantity=6)
    seed(dict(key, warehouseId=race['warehouseId']))
    lines = [dict(race, orderId=str(uuid4())) for _ in range(2)]
    with ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(consume, lines))
    assert sorted((r['reserved'], r['outstanding']) for r in results) == [(0, 6), (6, 0)]
    assert list(map(consume, lines)) == results
    mongo(db + '''if (dbi.reservations.countDocuments({}) !== 5) throw Error("unexpected reservation count");
        if (dbi.balances.countDocuments({available:0,expected:4}) !== 2) throw Error("unexpected stock balance");''')
    cancel = json.loads(Path('contracts/fixtures/cancel-stock-line-v1.json').read_text())
    consume(cancel, dict(env, INVENTORY_ORGANIZATION_ID=str(uuid4())), success=False)
    consume(dict(cancel, quantity=10), success=False)
    # Competing deliveries in separate processes must produce one release.
    with ThreadPoolExecutor(max_workers=2) as pool:
        assert sorted(r['released'] for r in pool.map(consume, [cancel, cancel])) == [0, 6]
    assert consume(cancel) == {'released': 0}
    assert consume(line) == {'reserved': 0, 'outstanding': 0}
    balance = f'dbi.balances.findOne({{_id:{json.dumps(key)}}})'
    assert mongo(db + f'print({balance}.available)') == '6'
    assert consume(dict(line, orderId=str(uuid4()), quantity=6)) == {'reserved': 6, 'outstanding': 0}
    # Post the four expected pieces as an Inventory-owned receipt fixture.
    mongo(db + f'dbi.balances.updateOne({{_id:{json.dumps(key)},expected:4}},{{$inc:{{expected:-4,available:4}}}})')
    assert consume(line) == {'reserved': 0, 'outstanding': 0}
    assert consume(cancel) == {'released': 0}
    assert mongo(db + f'print({balance}.available)') == '4'
    mongo(db + 'if (dbi.reservations.countDocuments({requested:10,reserved:0,outstanding:0,'
               '"cancellation.released":6,"cancellation.withdrawnDemand":4}) !== 1) throw Error("release history");')
    print('Partial allocation, cancellation, competing deliveries, scope and restart replay passed')
