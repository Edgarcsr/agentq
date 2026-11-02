const crypto = require('crypto');

function digest(algorithm, data) {
  return crypto.createHash(algorithm).update(data).digest();
}

digest('md5', 'payload'); // @expect md5:hash

(async () => {
  const pair = await import('crypto').then((c) => c.generateKeyPairSync('rsa', { modulusLength: 2048 })); // @expect rsa:unknown
})();
