(async () => {
  await crypto.subtle.generateKey({ name: 'RSA-OAEP', modulusLength: 4096, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' }, true, ['encrypt']); // @expect rsa:encryption
  await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign']); // @expect ecdsa:signature
  await crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-384' }, true, ['deriveKey']); // @expect ecdh:key_exchange
  await crypto.subtle.digest('SHA-1', data); // @expect sha1:hash
  await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, true, ['encrypt']); // @expect aes-256:encryption
})();
