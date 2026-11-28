"""Verify real Inventory-to-Procurement delivery on disposable, loopback-only fixtures."""

import os
import subprocess
from urllib.parse import urlsplit
from uuid import uuid4

from fixture_mongo import fixture_replica_set, output


def main():
    cases = (
        ('inventory', 'receipt::mongo::tests::receipt_transport_publish_committed_inventory_outbox'),
        ('procurement', 'receipt_projection::store::tests::receipt_transport_observe_inventory_facts_and_exact_replay'),
    )
    for owner, test in cases:
        ignored = output('cargo', 'test', '--locked', '-p', owner, '--', '--ignored', '--list')
        if f'{test}: test' not in ignored.splitlines():
            raise RuntimeError(f'expected ignored transport proof not found: {test}')
    image = output('docker', 'compose', '-f', 'compose.yaml', '-f', 'compose.delivery.yaml',
                   'config', '--images', 'nats')
    nats = output('docker', 'run', '-d', '--rm', '-p', '127.0.0.1::4222', image, '-js')
    try:
        address = output('docker', 'port', nats, '4222/tcp')
        host, port = address.split(':')
        assert host == '127.0.0.1' and port.isdigit()
        for owner, test in cases:
            database = f'{owner}_receipt_wire_{uuid4().hex}'
            with fixture_replica_set(database) as (uri, _, _):
                scoped_uri = urlsplit(uri)._replace(path=f'/{database}').geturl()
                env = dict(os.environ, RECEIPT_TRANSPORT_TEST_MONGO_URI=scoped_uri,
                           RECEIPT_TRANSPORT_TEST_NATS_PORT=port)
                subprocess.run(
                    ['cargo', 'test', '--locked', '-p', owner, '--', '--ignored', '--exact', test],
                    env=env, check=True, timeout=120,
                )
    finally:
        subprocess.run(['docker', 'stop', nats], stdout=subprocess.DEVNULL,
                       check=True, timeout=30)


if __name__ == '__main__':
    main()
