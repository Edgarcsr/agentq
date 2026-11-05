const crypto = require('crypto');

// Browser and server agree on a session key with elliptic curve Diffie-Hellman.
function startHandshake() {
  const ecdh = crypto.createECDH('prime256v1');
  const publicKey = ecdh.generateKeys();
  return { ecdh, publicKey };
}

function finishHandshake(ecdh, peerPublicKey) {
  const shared = ecdh.computeSecret(peerPublicKey);
  return crypto.createHash('sha256').update(shared).digest();
}

function protect(sessionKey, message) {
  const iv = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv('aes-256-gcm', sessionKey, iv);
  return { iv, data: Buffer.concat([cipher.update(message), cipher.final()]) };
}

module.exports = { startHandshake, finishHandshake, protect };
