"""Exercise independent Inventory processes against a disposable authenticated replica set."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
from uuid import uuid4


def output(*args, **kwargs):
    return subprocess.check_output(args, text=True, timeout=120, **kwargs).strip()


line = json.loads(Path('contracts/fixtures/confirmed-stock-line-v1.json').read_text())
image = re.search(r'image: (mongo:\S+)', Path('compose.yaml').read_text())[1]
tenant, org, password = str(uuid4()), str(uuid4()), secrets.token_hex(32)
container = output('docker', 'run', '-d', '-p', '127.0.0.1::27017', '--entrypoint', 'sh', image,
                   '-ec', 'umask 077; openssl rand -base64 48 > /tmp/key; mkdir /tmp/inv-db; chown mongodb /tmp/inv-db /tmp/key; '
                   'exec gosu mongodb mongod --dbpath /tmp/inv-db --replSet inv --keyFile /tmp/key --bind_ip_all')


def mongo(js):
    return output('docker', 'exec', '-i', container, 'mongosh', '--quiet', '--nodb', '--file', '/dev/stdin', input=js)


try:
    port = output('docker', 'port', container, '27017/tcp').split(':')[-1]
    # Only localhost inside the new container may bootstrap its first user.
    mongo('''const c = new Mongo("mongodb://127.0.0.1/?directConnection=true&serverSelectionTimeoutMS=10000");
        const admin=c.getDB("admin"); if (!admin.runCommand({replSetInitiate:{_id:"inv",members:[{_id:0,host:"localhost:27017"}]}}).ok) throw Error("init failed");
        const deadline=Date.now()+20000; while (!admin.runCommand({hello:1}).isWritablePrimary) { if(Date.now()>deadline) throw Error("no primary"); sleep(100); }
        const bootstrap="''' + secrets.token_hex(32) + '''";
        admin.createUser({user:"bootstrap",pwd:bootstrap,roles:["root"]});
        admin.auth("bootstrap",bootstrap);
        admin.createUser({user:"fixture",pwd:"''' + password + '''",roles:[{role:"readWrite",db:"inventory"}]});''')
    uri = f'mongodb://fixture:{password}@127.0.0.1:{port}/?authSource=admin&directConnection=true&serverSelectionTimeoutMS=5000'
    env = dict(os.environ, INVENTORY_MONGO_URI=uri, INVENTORY_TENANT_ID=tenant, INVENTORY_ORGANIZATION_ID=org)
    db = f'const dbi=new Mongo("{uri.replace(":"+port+"/", ":27017/")}").getDB("inventory");\n'
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
    print('Partial allocation, competing orders, scope and restart replay passed')
finally:
    subprocess.run(['docker', 'rm', '-fv', container], check=True, stdout=subprocess.DEVNULL)
