const crypto = require('node:crypto');

const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 3072 }); // @expect rsa:encryption
const sealed = crypto.publicEncrypt(publicKey, Buffer.from('secret')); // @expect rsa:encryption
const opened = crypto.privateDecrypt(privateKey, sealed); // @expect rsa:encryption
