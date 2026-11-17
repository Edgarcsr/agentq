// TLS configuration: what a server or client is willing to negotiate.
const https = require('node:https');
const tls = require('node:tls');

// Cipher suites name a key exchange, an authentication algorithm and a cipher.
https.createServer({ // @expect ecdh:key_exchange, rsa:signature, aes-128:encryption, ecdsa:signature, aes-256:encryption, dh:key_exchange, sha1:mac
  ciphers: 'ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:DHE-RSA-AES256-SHA',
  minVersion: 'TLSv1.2',
}, handler);

// Without ECDHE or DHE the session key travels under the server's RSA key.
tls.createSecureContext({ ciphers: 'AES128-SHA:!aNULL' }); // @expect rsa:key_exchange, aes-128:encryption, sha1:mac

// Offered curves; a hybrid with ML-KEM is post-quantum.
tls.connect(443, 'example.com', { ecdhCurve: 'X25519:secp384r1:X25519MLKEM768' }); // @expect x25519:key_exchange, ecdh:key_exchange, ml-kem:key_exchange

// Not literals, or not TLS: nothing to report.
https.createServer({ ciphers: process.env.CIPHERS }, handler);
require('node:http').createServer({ ciphers: 'AES128-SHA' }, handler);
