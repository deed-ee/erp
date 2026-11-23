module.exports = {
  '/v1/**': { target: `http://127.0.0.1:${process.env.ERP4US_PORT || '8080'}` }
};
