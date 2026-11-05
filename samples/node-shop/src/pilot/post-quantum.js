const { generateKeyPairSync } = require('node:crypto');
const { ml_kem768 } = require('@noble/post-quantum/ml-kem');

// Pilot for the next generation of the checkout: already post-quantum.
const signingKeys = generateKeyPairSync('ml-dsa-65');

function newSessionKeys() {
  return ml_kem768.keygen();
}

module.exports = { signingKeys, newSessionKeys };
