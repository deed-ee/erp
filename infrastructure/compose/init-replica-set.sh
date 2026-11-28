#!/bin/sh
set -eu
mongo() {
    mongosh --quiet --norc \
        'mongodb://mongo:27017/admin?directConnection=true&serverSelectionTimeoutMS=1000' \
        --username "$MONGO_INITDB_ROOT_USERNAME" --password "$MONGO_INITDB_ROOT_PASSWORD" "$@"
}

for _attempt in $(seq 1 30); do
    if mongo --eval 'quit(db.adminCommand({ping: 1}).ok === 1 ? 0 : 1)' >/dev/null 2>&1; then
        exec mongosh --quiet --norc \
            'mongodb://mongo:27017/admin?directConnection=true&serverSelectionTimeoutMS=5000' \
            --username "$MONGO_INITDB_ROOT_USERNAME" --password "$MONGO_INITDB_ROOT_PASSWORD" \
            --eval '
                try {
                    rs.status();
                } catch (error) {
                    if (error.codeName !== "NotYetInitialized") throw error;
                    const result = rs.initiate({
                        _id: "erp4us", members: [{_id: 0, host: "mongo:27017"}]
                    });
                    if (result.ok !== 1) throw new Error("Replica-set initialization failed");
                }
            '
    fi
    sleep 1
done
echo "MongoDB did not become available for replica-set initialization" >&2
exit 1
