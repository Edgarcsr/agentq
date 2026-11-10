const crypto = require('crypto');
const tls = require('tls');
const https = require('https');
const jwt = require('jsonwebtoken');
const bcrypt = require('bcrypt');
const { hashIt } = require('./util');

class Signer {
  constructor(alg) { this.alg = alg; }
  run(d) { return crypto.createHash(this.alg).update(d).digest(); }          // L10 dynamic field
}
new Signer('md5').run('x');                                                   // L12
crypto.pbkdf2Sync(pw, salt, 1000, 32, 'sha1');                                // L13 KDF
crypto.createHmac('md5', k);                                                  // L14 HMAC-MD5
tls.createServer({ ciphers: 'ECDHE-RSA-AES128-GCM-SHA256', minVersion: 'TLSv1' }); // L15 TLS
https.createServer({ key: fs.readFileSync('server.key'), cert }, app);        // L16 cert
crypto.createSign('SHA256').update(d).sign(ecPrivateKey);                     // L17 ECDSA via createSign
const h = crypto.createHash; h('md5');                                        // L18 alias of member
hashIt('md5', data);                                                          // L19 cross-file
crypto.generateKeyPairSync('ec', { namedCurve: 'P-256' });                    // L20 lone EC
crypto.createCipheriv('rc4', key, '');                                        // L21 RC4
crypto.createCipheriv('des-cbc', key, iv);                                    // L22 DES
crypto.createCipheriv('bf-cbc', key, iv);                                     // L23 blowfish
crypto.sign(null, data, crypto.createPrivateKey(pem));                        // L24 keyed sign
jwt.sign(payload, privateKey, opts);                                          // L25 opts variable
jwt.sign(payload, secret);                                                    // L26 default HS256
bcrypt.hashSync(pw, 10);                                                      // L27
crypto.scryptSync(pw, salt, 32);                                              // L28
const PEM = `-----BEGIN RSA PRIVATE KEY-----\nMIIE...`;                       // L29 embedded key
crypto.subtle.importKey('spki', der, { name: 'RSA-OAEP', hash: 'SHA-256' }, false, ['encrypt']); // L30
crypto.createHash('sha256').update(password).digest('hex');                   // L31 password hashing
crypto.generateKeyPairSync('rsa', { modulusLength: 1024 });                   // L32 weak classical
crypto.generateKeyPairSync('ed448'); crypto.generateKeyPairSync('x448');      // L33
crypto.createHash('ripemd160'); crypto.createHash('sha224');                  // L34
const algo = cond ? 'md5' : 'sha256'; crypto.createHash(algo);                // L35 ternary
