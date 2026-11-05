const { createHash } = require('crypto');

module.exports = function etag(body) {
  return createHash('sha1').update(body).digest('base64');
};
