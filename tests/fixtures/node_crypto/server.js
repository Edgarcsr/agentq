const crypto = require('node:crypto');

const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
  modulusLength: 2048,
});

function fingerprint(data) {
  return crypto.createHash('sha1').update(data).digest('hex');
}

function wrap(buffer) {
  return crypto.publicEncrypt(publicKey, buffer);
}

module.exports = { fingerprint, wrap, privateKey };
