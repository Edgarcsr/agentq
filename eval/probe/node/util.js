const crypto = require('crypto');
exports.hashIt = (alg, d) => crypto.createHash(alg).update(d).digest();
