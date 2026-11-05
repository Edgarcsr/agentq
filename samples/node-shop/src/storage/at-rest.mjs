import { createCipheriv, createDecipheriv } from 'node:crypto';

// Files uploaded before 2022 were encrypted with 128-bit keys.
export function encryptOld(key, iv, data) {
  const cipher = createCipheriv('aes-128-cbc', key, iv);
  return Buffer.concat([cipher.update(data), cipher.final()]);
}

export function decryptOld(key, iv, data) {
  const decipher = createDecipheriv('aes-128-cbc', key, iv);
  return Buffer.concat([decipher.update(data), decipher.final()]);
}

export function encryptNew(key, iv, data) {
  const cipher = createCipheriv('aes-256-gcm', key, iv);
  return Buffer.concat([cipher.update(data), cipher.final()]);
}
