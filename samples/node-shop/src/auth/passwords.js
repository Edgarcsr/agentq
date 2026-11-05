const crypto = require('crypto');

// Old accounts still store md5 hashes; new ones use a salted sha256.
function legacyHash(password) {
  return crypto.createHash('md5').update(password).digest('hex');
}

function hashPassword(password, salt) {
  return crypto.createHmac('sha256', salt).update(password).digest('hex');
}

module.exports = { legacyHash, hashPassword };
