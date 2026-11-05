const { generateKeyPairSync, publicEncrypt, privateDecrypt } = require('node:crypto');

// Every merchant gets a key pair; their API secret is stored encrypted with it.
const { publicKey, privateKey } = generateKeyPairSync('rsa', {
  modulusLength: 2048,
});

function sealSecret(secret) {
  return publicEncrypt(publicKey, Buffer.from(secret));
}

function openSecret(sealed) {
  return privateDecrypt(privateKey, sealed).toString();
}

module.exports = { sealSecret, openSecret };
