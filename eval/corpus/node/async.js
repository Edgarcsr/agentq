// Asynchronous Node: callbacks, util.promisify, promises and await.
const crypto = require('node:crypto');
const { promisify } = require('node:util');

// The keys of a callback are the parameters after the error.
crypto.generateKeyPair('rsa', { modulusLength: 2048 }, (err, pub, priv) => { // @expect rsa:signature
  crypto.sign('sha256', Buffer.from('a'), priv, (e, signature) => {}); // @expect sha2-256:hash
});

// A promisified function is still the same function.
const generate = promisify(crypto.generateKeyPair);
async function sign() {
  const { privateKey } = await generate('ec', { namedCurve: 'P-256' }); // @expect ecdsa:signature
  return crypto.sign(null, Buffer.from('x'), privateKey);
}

const pbkdf2 = promisify(crypto.pbkdf2);
async function derive() {
  return pbkdf2('password', 'salt', 1000, 32, 'sha1'); // @expect sha1:key_derivation
}

// The result of a promise reaches `.then` as a parameter.
generate('rsa', { modulusLength: 3072 }) // @expect rsa:encryption
  .then(({ publicKey }) => crypto.publicEncrypt(publicKey, Buffer.from('s'))); // @expect rsa:encryption

// Dynamic import and Web Crypto under await.
async function dynamic() {
  const { createHash } = await import('node:crypto');
  createHash('md5'); // @expect md5:hash
  const kp = await crypto.subtle.generateKey( // @expect ecdsa:signature
    { name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign']);
}

// A promisified local function is not cryptography.
function local(cb) { cb(null, 'rsa'); }
const promisifiedLocal = promisify(local);
promisifiedLocal('rsa', { modulusLength: 2048 });
