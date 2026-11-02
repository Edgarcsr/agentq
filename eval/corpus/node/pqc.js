const { generateKeyPairSync } = require('node:crypto');
const { ml_kem768 } = require('@noble/post-quantum/ml-kem');

generateKeyPairSync('ml-kem-768'); // @expect ml-kem:key_exchange
generateKeyPairSync('ml-dsa-65'); // @expect ml-dsa:signature
ml_kem768.keygen(); // @expect ml-kem:key_exchange
