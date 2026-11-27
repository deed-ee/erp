#!/usr/bin/env python3
"""Rehearse order migration in a disposable local SQLite sandbox."""
import argparse
from contextlib import closing
from hashlib import sha256
import json
import os
from pathlib import Path
import sqlite3
import sys

from migration_inputs import (
    InvalidInput, add_context_arguments, read_change_window, read_context, read_snapshot,
)


APPLICATION_ID = 0x45525034
STORE_VERSION = 1


def scope_of(expected):
    return tuple(expected[key] for key in ("tenantId", "organizationId", "sourceSystem"))


def create_file(path):
    """Reserve a private file without truncating any existing path."""
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        return False
    os.close(descriptor)
    return True


def connect(path, *, readonly):
    mode = "ro" if readonly else "rw"
    connection = sqlite3.connect(path.absolute().as_uri() + f"?mode={mode}", uri=True, timeout=5)
    connection.execute("PRAGMA trusted_schema = OFF")
    return connection


def create_tables(connection, expected):
    # Do not use executescript: it would implicitly commit the current transaction.
    connection.execute(f"PRAGMA application_id = {APPLICATION_ID}")
    connection.execute(f"PRAGMA user_version = {STORE_VERSION}")
    connection.execute("""
        CREATE TABLE migration_scope (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            tenant TEXT NOT NULL,
            organization TEXT NOT NULL,
            source TEXT NOT NULL,
            checkpoint TEXT NOT NULL
        )
    """)
    connection.execute("""
        CREATE TABLE applied_windows (
            checkpoint TEXT PRIMARY KEY,
            previous_checkpoint TEXT,
            digest TEXT NOT NULL,
            order_count INTEGER NOT NULL
        ) WITHOUT ROWID
    """)
    connection.execute("""
        CREATE TABLE orders (
            order_ref TEXT PRIMARY KEY,
            payload TEXT NOT NULL
        ) WITHOUT ROWID
    """)
    connection.execute("INSERT INTO migration_scope VALUES (1, ?, ?, ?, ?)",
                       (*scope_of(expected), expected["checkpoint"]))


def current_checkpoint(connection, expected):
    if (connection.execute("PRAGMA application_id").fetchone() != (APPLICATION_ID,)
            or connection.execute("PRAGMA user_version").fetchone() != (STORE_VERSION,)):
        raise InvalidInput("not a version 1 order rehearsal sandbox")
    scope = connection.execute(
        "SELECT tenant, organization, source, checkpoint FROM migration_scope WHERE singleton = 1"
    ).fetchone()
    if scope is None or scope[:3] != scope_of(expected):
        raise InvalidInput("sandbox scope does not match rehearsal")
    return scope[3]


def apply(path, expected, previous, orders, *, initial):
    created = initial and create_file(path)
    checkpoint = expected["checkpoint"]
    # Indexing and sorted JSON make key/row order immaterial. The original exact
    # decimal strings remain part of the batch identity, not toleranced values.
    payload = json.dumps(orders, sort_keys=True, separators=(",", ":"))
    digest = sha256(payload.encode("utf-8")).hexdigest()
    with closing(connect(path, readonly=False)) as connection, connection:
        connection.execute("BEGIN IMMEDIATE")
        if created:
            create_tables(connection, expected)
        head = current_checkpoint(connection, expected)
        known = connection.execute(
            "SELECT previous_checkpoint, digest FROM applied_windows WHERE checkpoint = ?",
            (checkpoint,),
        ).fetchone()
        if known is not None:
            if known != (previous, digest):
                raise InvalidInput("checkpoint already recorded with different input")
            status = "alreadyApplied"
        else:
            if (initial and not created) or (not initial and head != previous):
                raise InvalidInput("change window does not continue the current checkpoint")
            connection.executemany(
                "INSERT INTO orders (order_ref, payload) VALUES (?, ?) "
                "ON CONFLICT(order_ref) DO UPDATE SET payload = excluded.payload",
                ((ref, json.dumps(order, sort_keys=True)) for ref, order in sorted(orders.items())),
            )
            connection.execute("INSERT INTO applied_windows VALUES (?, ?, ?, ?)",
                               (checkpoint, previous, digest, len(orders)))
            connection.execute("UPDATE migration_scope SET checkpoint = ? WHERE singleton = 1", (checkpoint,))
            head = checkpoint
            status = "applied"
    return {"schemaVersion": 1, **expected, "previousCheckpoint": previous,
            "currentCheckpoint": head, "orderCount": len(orders), "result": status}


def export(path, expected):
    with closing(connect(path, readonly=True)) as connection, connection:
        connection.execute("BEGIN")
        if current_checkpoint(connection, expected) != expected["checkpoint"]:
            raise InvalidInput("sandbox is not at the requested checkpoint")
        rows = []
        for (payload,) in connection.execute("SELECT payload FROM orders ORDER BY order_ref"):
            order = json.loads(payload)
            order["lines"] = [order["lines"][key] for key in sorted(order["lines"])]
            rows.append(order)
    return {"schemaVersion": 1, "snapshotRole": "candidate", **expected, "orders": rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    add_context_arguments(parser)
    parser.add_argument("--sandbox", type=Path, required=True, help="path to a disposable rehearsal database")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("initial").add_argument("input", type=Path)
    commands.add_parser("apply").add_argument("input", type=Path)
    commands.add_parser("export")
    args = parser.parse_args()
    expected = read_context(parser, args)
    try:
        if args.command == "export":
            report = export(args.sandbox, expected)
        else:
            initial = args.command == "initial"
            if initial:
                previous, orders = None, read_snapshot(args.input, {**expected, "snapshotRole": "legacy"})
                if not orders:
                    raise InvalidInput("initial snapshot has no orders")
            else:
                previous, orders = read_change_window(args.input, expected)
            report = apply(args.sandbox, expected, previous, orders, initial=initial)
    except InvalidInput as error:
        print(f"shadow: {error}", file=sys.stderr)
        return 2
    except (OSError, sqlite3.Error, ValueError, KeyError, TypeError):
        # Do not echo database errors, paths or potentially sensitive payloads.
        print("shadow: unable to process local sandbox", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
