// RSA, md5 and sha1 are only mentioned in comments: crypto.createHash('md5')
const crypto = require('crypto');

const help = "Use crypto.generateKeyPairSync('rsa') to make a key";
const note = `createHash('sha1') is discouraged`;
const other = { createHash: (name) => name, generateKeyPairSync: () => 1 };

other.createHash('md5');
other.generateKeyPairSync('rsa');
function wrapper(crypto) { return crypto.createHash('md5'); }
crypto.randomUUID();
crypto.randomBytes(32);
crypto.createHash(process.env.HASH_ALGORITHM);
