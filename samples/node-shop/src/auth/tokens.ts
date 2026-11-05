import jwt from 'jsonwebtoken';
import { readFileSync } from 'node:fs';

const privateKey = readFileSync('keys/jwt-private.pem');
const publicKey = readFileSync('keys/jwt-public.pem');

export function issueToken(userId: string): string {
  return jwt.sign({ sub: userId }, privateKey, { algorithm: 'RS256', expiresIn: '15m' });
}

export function readToken(token: string) {
  return jwt.verify(token, publicKey, { algorithms: ['RS256', 'ES256'] });
}

export function issueInternalToken(userId: string, secret: string): string {
  return jwt.sign({ sub: userId }, secret, { algorithm: 'HS256' });
}
