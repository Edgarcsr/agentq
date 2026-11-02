const crypto = require('crypto');
const { generateKeyPairSync } = crypto;

generateKeyPairSync('rsa', { modulusLength: 2048 }); // @expect rsa:unknown
crypto.generateKeyPairSync('dsa', { modulusLength: 2048 }); // @expect dsa:signature
crypto.generateKeyPairSync('ed25519'); // @expect ed25519:signature
crypto.generateKeyPairSync('x25519'); // @expect x25519:key_exchange
crypto.createECDH('secp256k1'); // @expect ecdh:key_exchange
crypto.createDiffieHellman(2048); // @expect dh:key_exchange
crypto.getDiffieHellman('modp14'); // @expect dh:key_exchange
