import { createHash, createHmac } from 'node:crypto';

createHash('md5').update('x').digest('hex'); // @expect md5:hash
createHash('sha1'); // @expect sha1:hash
createHash('sha256'); // @expect sha2-256:hash
createHash('sha384'); // @expect sha2-384:hash
createHash('sha512'); // @expect sha2-512:hash
createHash('sha3-256'); // @expect sha3:hash
createHmac('sha256', key); // @expect sha2-256:mac
