const jwt = require('jsonwebtoken');
const forge = require('node-forge');
const NodeRSA = require('node-rsa');
const nacl = require('tweetnacl');
const CryptoJS = require('crypto-js');

jwt.sign(payload, privateKey, { algorithm: 'RS256' }); // @expect rsa:signature
jwt.sign(payload, secret, { algorithm: 'HS256' });
jwt.verify(token, publicKey, { algorithms: ['ES256'] }); // @expect ecdsa:signature
forge.pki.rsa.generateKeyPair({ bits: 2048 }); // @expect rsa:unknown
new NodeRSA({ b: 2048 }); // @expect rsa:unknown
nacl.sign.keyPair(); // @expect ed25519:signature
CryptoJS.MD5('x'); // @expect md5:hash
CryptoJS.SHA256('x'); // @expect sha2-256:hash
