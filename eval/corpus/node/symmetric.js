const crypto = require('crypto');

crypto.createCipheriv('aes-256-gcm', key, iv); // @expect aes-256:encryption
crypto.createDecipheriv('aes-128-cbc', key, iv); // @expect aes-128:encryption
crypto.createCipheriv('aes-192-ctr', key, iv); // @expect aes-192:encryption
crypto.createCipheriv('des-ede3-cbc', key, iv); // @expect 3des:encryption
crypto.createCipheriv('chacha20-poly1305', key, iv); // @expect chacha20:encryption
