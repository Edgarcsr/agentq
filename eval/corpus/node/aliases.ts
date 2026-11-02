import * as nodeCrypto from 'node:crypto';
import { createCipheriv as makeCipher } from 'crypto';

const sub = nodeCrypto.webcrypto.subtle;
const TYPE = 'ec';

nodeCrypto.createHash('sha1'); // @expect sha1:hash
makeCipher('aes-256-cbc', key, iv); // @expect aes-256:encryption
sub.digest('SHA-256', data); // @expect sha2-256:hash
nodeCrypto.generateKeyPairSync(TYPE, { namedCurve: 'P-256' }); // @expect ecdh:key_exchange
nodeCrypto.diffieHellman({ privateKey, publicKey });
