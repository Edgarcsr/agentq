import { createCipheriv, createECDH, randomBytes } from 'crypto';

// Mentions of RSA or md5 in comments must not be reported.
export function open(peerKey: Buffer) {
  const ecdh = createECDH('prime256v1');
  ecdh.generateKeys();
  const secret = ecdh.computeSecret(peerKey);
  const cipher = createCipheriv('aes-256-gcm', secret, randomBytes(12));
  return cipher;
}
