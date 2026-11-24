#!/bin/sh
set -eu
exec mongosh --quiet --norc 'mongodb://mongo:27017/admin?replicaSet=erp4us' \
    --username "$MONGO_INITDB_ROOT_USERNAME" --password "$MONGO_INITDB_ROOT_PASSWORD" \
    --eval '
        const receipts = db.getSiblingDB("article_receipts");
        if (receipts.getUser("article_receipts_app") === null) {
            receipts.createUser({
                user: "article_receipts_app",
                pwd: process.env.DEV_RECEIPTS_MONGO_PASSWORD,
                roles: [{role: "readWrite", db: "article_receipts"}]
            });
        }
    '
