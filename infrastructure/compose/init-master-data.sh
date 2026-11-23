mongosh --quiet --norc <<'JS'
db.getSiblingDB('master_data').createUser({
  user: 'master_data_app',
  pwd: process.env.DEV_MASTER_DATA_PASSWORD,
  roles: [{role: 'readWrite', db: 'master_data'}]
});
JS
