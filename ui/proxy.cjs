module.exports = {
  '/v1/manufacturing/**': { target: `http://127.0.0.1:${process.env.ERP4US_MANUFACTURING_PORT || '8090'}` },
  '/v1/**': { target: `http://127.0.0.1:${process.env.ERP4US_PORT || '8080'}` }
};
