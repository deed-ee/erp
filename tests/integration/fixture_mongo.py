"""Provision an isolated Mongo replica set with a database-local fixture account."""
from contextlib import contextmanager
from pathlib import Path
import re
import secrets
import subprocess


def output(*args, **kwargs):
    return subprocess.check_output(args, text=True, timeout=120, **kwargs).strip()


@contextmanager
def fixture_replica_set(database):
    image = re.search(r'image: (mongo:\S+)', Path('compose.yaml').read_text())[1]
    password = secrets.token_hex(32)
    container = output('docker', 'run', '-d', '-p', '127.0.0.1::27017', '--entrypoint', 'sh', image,
                       '-ec', 'umask 077; openssl rand -base64 48 > /tmp/key; '
                       'mkdir /tmp/fixture-db; chown mongodb /tmp/fixture-db /tmp/key; '
                       'exec gosu mongodb mongod --dbpath /tmp/fixture-db --replSet fixture '
                       '--keyFile /tmp/key --bind_ip_all')

    def mongo(js):
        return output('docker', 'exec', '-i', container, 'mongosh', '--quiet', '--nodb', '--file', '/dev/stdin', input=js)

    try:
        port = output('docker', 'port', container, '27017/tcp').split(':')[-1]
        # Only localhost inside the new container may bootstrap its first user.
        mongo('''const c = new Mongo("mongodb://127.0.0.1/?directConnection=true&serverSelectionTimeoutMS=10000");
            const admin=c.getDB("admin"); if (!admin.runCommand({replSetInitiate:{_id:"fixture",members:[{_id:0,host:"localhost:27017"}]}}).ok) throw Error("init failed");
            const deadline=Date.now()+20000; while (!admin.runCommand({hello:1}).isWritablePrimary) { if(Date.now()>deadline) throw Error("no primary"); sleep(100); }
            const bootstrap="''' + secrets.token_hex(32) + '''";
            admin.createUser({user:"bootstrap",pwd:bootstrap,roles:["root"]});
            admin.auth("bootstrap",bootstrap);
            admin.createUser({user:"fixture",pwd:"''' + password + '''",roles:[{role:"readWrite",db:"''' + database + '''"}]});''')
        uri = f'mongodb://fixture:{password}@127.0.0.1:{port}/?authSource=admin&directConnection=true&serverSelectionTimeoutMS=5000'
        yield uri, mongo, uri.replace(":" + port + "/", ":27017/")
    finally:
        subprocess.run(['docker', 'rm', '-fv', container], check=True, stdout=subprocess.DEVNULL)
