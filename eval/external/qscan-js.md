# agentq on the qScan recall corpus (js, quantakrypto/pqc-tools@6a81b5537eae)

- Detection recall: 9/22 (0.409)
- Without TLS, SSH and dependency manifests: 9/14 (0.643)
- By difficulty: adversarial 0/2, aliased 3/3, canonical 3/3, config 0/8, uncommon 3/6

## Missed

- `jose-rsa-oaep.ts`: RSA (uncommon) jose alg 'RSA-OAEP-256' RSA key transport
- `jose-rsa-oaep.ts`: RSA (uncommon) jose alg 'RSA-OAEP' RSA key transport
- `jose-rsa-oaep.ts`: RSA (uncommon) jose alg 'RSA-OAEP-384' RSA key transport
- `firmware-manifest-signer.js`: EDDSA (adversarial) dynamic require('crypto') crypto.sign(null, msg, key) Ed25519 one-shot, multi-line
- `firmware-manifest-signer.js`: EDDSA (adversarial) verify(null, msg, key, sig) Ed25519 one-shot, multi-line
- `tls-server-options.ts`: TLS (config) https ServerOptions ciphers ECDHE-RSA/DHE-RSA classical suites
- `tls-server-options.ts`: TLS (config) https ServerOptions minVersion/maxVersion pinned to TLSv1.2
- `deploy/authorized_keys`: SSH (config) authorized_keys ssh-rsa public key entry
- `deploy/authorized_keys`: SSH (config) authorized_keys ssh-ed25519 public key entry
- `deploy/authorized_keys`: SSH (config) authorized_keys ecdsa-sha2-nistp256 public key entry
- `legacy-payment-gateway/package.json`: DEP (config) package.json dependency node-forge
- `legacy-payment-gateway/package.json`: DEP (config) package.json dependency elliptic
- `legacy-payment-gateway/package.json`: DEP (config) package.json dependency jsonwebtoken
