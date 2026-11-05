const crypto = require('crypto');

// The bank gateway from 2009 still wants triple DES and SHA-1.
const ALGORITHM = 'des-ede3-cbc';

function encryptCardPayload(payload, key, iv) {
  const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
  return Buffer.concat([cipher.update(payload), cipher.final()]);
}

function checksum(payload) {
  return crypto.createHash('sha1').update(payload).digest('hex');
}

module.exports = { encryptCardPayload, checksum };
