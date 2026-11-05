import { generateKeyPairSync, sign, verify } from 'node:crypto';

const { publicKey, privateKey } = generateKeyPairSync('ed25519');

export function signWebhook(body: Buffer): Buffer {
  return sign(null, body, privateKey);
}

export function checkWebhook(body: Buffer, signature: Buffer): boolean {
  return verify(null, body, publicKey, signature);
}
