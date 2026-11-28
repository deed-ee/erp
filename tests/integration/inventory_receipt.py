"""Run Inventory's persistence proofs on separate authenticated disposable replica sets."""

import os
import subprocess
from urllib.parse import urlsplit
from uuid import uuid4

from fixture_mongo import fixture_replica_set


def main():
    module = 'receipt::mongo::tests::'
    tests = tuple(module + case for case in (
        'mongo_competing_lots_and_rollback_keep_all_effects_atomic',
        'mongo_recovers_the_same_lot_until_deliberately_advancing_the_current_intent',
        'mongo_application_replays_before_contacting_the_owner',
    ))
    cargo = ['cargo', 'test', '--locked', '-p', 'inventory']
    # Check the exact names before starting Docker: libtest succeeds if zero match.
    ignored = subprocess.check_output(
        [*cargo, '--', '--ignored', '--list'], text=True, timeout=120,
    ).splitlines()
    for test in tests:
        if f'{test}: test' not in ignored:
            raise RuntimeError(f'expected ignored Inventory test not found: {test}')
    for test in tests:
        database = f'inventory_receipt_test_{uuid4().hex}'
        with fixture_replica_set(database) as (uri, _, _):
            scoped_uri = urlsplit(uri)._replace(path=f'/{database}').geturl()
            env = dict(os.environ, INVENTORY_RECEIPT_TEST_MONGO_URI=scoped_uri)
            subprocess.run(
                [*cargo, '--', '--ignored', '--exact', test], env=env, check=True, timeout=120,
            )


if __name__ == '__main__':
    main()
