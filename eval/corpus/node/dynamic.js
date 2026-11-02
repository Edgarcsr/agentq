const crypto = require('crypto');
const BITS = 256;
const config = { hash: 'sha1' };

crypto.createCipheriv(`aes-${BITS}-gcm`, key, iv); // @expect aes-256:encryption
crypto.createHash(config.hash); // @expect sha1:hash

let alg = 'md5';
alg = process.env.HASH;
crypto.createHash(alg);
