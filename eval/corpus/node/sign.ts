import { generateKeyPairSync, createSign, createVerify, sign } from 'crypto';

const { privateKey } = generateKeyPairSync('rsa', { modulusLength: 2048 }); // @expect rsa:signature
sign('sha256', Buffer.from('payload'), privateKey); // @expect sha2-256:hash
const signer = createSign('RSA-SHA256'); // @expect rsa:signature
const verifier = createVerify('RSA-SHA256'); // @expect rsa:signature
