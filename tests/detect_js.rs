use agentq::analyzer::Analyzer;
use agentq::finding::{Confidence, Purpose};
use agentq::modules::javascript::JavaScriptAnalyzer;
use std::path::Path;

/// `(algorithm, purpose, key size)` of every finding in `source`, in source order.
fn detect(source: &str) -> Vec<(String, Purpose, Option<u32>)> {
    JavaScriptAnalyzer
        .analyze(Path::new("test.ts"), source)
        .into_iter()
        .map(|f| (f.algorithm, f.purpose, f.key_size))
        .collect()
}

fn found(
    algorithm: &str,
    purpose: Purpose,
    key_size: Option<u32>,
) -> (String, Purpose, Option<u32>) {
    (algorithm.to_string(), purpose, key_size)
}

#[test]
fn rsa_key_generation_reports_modulus_length() {
    let source = "
        import { generateKeyPairSync } from 'node:crypto';
        generateKeyPairSync('rsa', { modulusLength: 2048 });
    ";
    assert_eq!(
        detect(source),
        vec![found("rsa", Purpose::Unknown, Some(2048))]
    );
}

#[test]
fn rsa_pss_keys_are_for_signatures() {
    let source = "
        const crypto = require('crypto');
        crypto.generateKeyPair('rsa-pss', { modulusLength: 4096 }, cb);
    ";
    assert_eq!(
        detect(source),
        vec![found("rsa", Purpose::Signature, Some(4096))]
    );
}

#[test]
fn constants_and_aliases_are_followed() {
    let source = "
        const { generateKeyPairSync: gen } = require('crypto');
        const TYPE = 'rsa';
        const BITS = 3072;
        gen(TYPE, { modulusLength: BITS });
    ";
    assert_eq!(
        detect(source),
        vec![found("rsa", Purpose::Unknown, Some(3072))]
    );
}

#[test]
fn elliptic_curve_keys_report_curve_size() {
    let source = "
        const crypto = require('crypto');
        crypto.generateKeyPairSync('ec', { namedCurve: 'secp384r1' });
        crypto.createECDH('prime256v1');
    ";
    assert_eq!(
        detect(source),
        vec![
            found("ecdh", Purpose::KeyExchange, Some(384)),
            found("ecdh", Purpose::KeyExchange, Some(256)),
        ]
    );
}

#[test]
fn ec_key_generation_is_reported_with_medium_confidence() {
    let findings = JavaScriptAnalyzer.analyze(
        Path::new("a.js"),
        "require('crypto').generateKeyPairSync('ec', { namedCurve: 'P-256' });",
    );
    assert_eq!(findings[0].confidence, Confidence::Medium);
}

#[test]
fn modern_curves_and_dsa() {
    let source = "
        const c = require('crypto');
        c.generateKeyPairSync('ed25519');
        c.generateKeyPairSync('x25519');
        c.generateKeyPairSync('dsa', { modulusLength: 2048, divisorLength: 256 });
    ";
    assert_eq!(
        detect(source),
        vec![
            found("ed25519", Purpose::Signature, Some(256)),
            found("x25519", Purpose::KeyExchange, Some(256)),
            found("dsa", Purpose::Signature, Some(2048)),
        ]
    );
}

#[test]
fn diffie_hellman_variants() {
    let source = "
        const crypto = require('crypto');
        crypto.createDiffieHellman(2048);
        crypto.getDiffieHellman('modp14');
        crypto.createDiffieHellman(prime);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("dh", Purpose::KeyExchange, Some(2048)),
            found("dh", Purpose::KeyExchange, Some(2048)),
            found("dh", Purpose::KeyExchange, None),
        ]
    );
}

#[test]
fn rsa_encryption_and_signing_apis() {
    let source = "
        const crypto = require('crypto');
        crypto.publicEncrypt(key, buf);
        crypto.privateDecrypt(key, buf);
        crypto.createSign('RSA-SHA256');
        crypto.createVerify('sha256');
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Encryption, None),
            found("rsa", Purpose::Encryption, None),
            found("rsa", Purpose::Signature, None),
            // A bare digest leaves the key type to the key, so only the hash is known.
            found("sha2-256", Purpose::Hash, None),
        ]
    );
}

#[test]
fn symmetric_ciphers_report_key_size() {
    let source = "
        const crypto = require('crypto');
        crypto.createCipheriv('aes-256-gcm', key, iv);
        crypto.createDecipheriv('aes-128-cbc', key, iv);
        crypto.createCipheriv('des-ede3-cbc', key, iv);
        crypto.createCipheriv('chacha20-poly1305', key, iv);
        crypto.generateKeySync('aes', { length: 192 });
    ";
    assert_eq!(
        detect(source),
        vec![
            found("aes-256", Purpose::Encryption, Some(256)),
            found("aes-128", Purpose::Encryption, Some(128)),
            found("3des", Purpose::Encryption, None),
            found("chacha20", Purpose::Encryption, Some(256)),
            found("aes-192", Purpose::Encryption, Some(192)),
        ]
    );
}

#[test]
fn hash_functions() {
    let source = "
        import * as crypto from 'crypto';
        crypto.createHash('md5');
        crypto.createHash('sha1').update(x);
        crypto.createHmac('sha256', key);
        crypto.createHash('sha512');
    ";
    assert_eq!(
        detect(source),
        vec![
            found("md5", Purpose::Hash, None),
            found("sha1", Purpose::Hash, None),
            found("sha2-256", Purpose::Mac, None),
            found("sha2-512", Purpose::Hash, None),
        ]
    );
}

#[test]
fn reports_location_and_snippet() {
    let source = "const crypto = require('crypto');\n\n  crypto.createHash('md5');\n";
    let findings = JavaScriptAnalyzer.analyze(Path::new("src/a.js"), source);
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.file, Path::new("src/a.js"));
    assert_eq!((finding.line, finding.column), (3, 3));
    assert_eq!(finding.snippet, "crypto.createHash('md5')");
    assert_eq!(finding.library, "node:crypto");
    assert_eq!(finding.confidence, Confidence::High);
}

#[test]
fn ignores_lookalikes() {
    let source = "
        // crypto.createHash('md5') and RSA are only mentioned in comments
        const text = \"generateKeyPairSync('rsa')\";
        const crypto = require('crypto');
        crypto.randomUUID();
        crypto.randomBytes(16);
        const other = { createHash: (x) => x };
        other.createHash('md5');
        createHash('md5');
    ";
    assert_eq!(detect(source), vec![]);
}

#[test]
fn unresolvable_arguments_produce_no_guess() {
    let source = "
        const crypto = require('crypto');
        crypto.createHash(algorithmFromConfig);
        crypto.createCipheriv(name, key, iv);
    ";
    assert_eq!(detect(source), vec![]);
}

#[test]
fn works_in_javascript_files_too() {
    let findings = JavaScriptAnalyzer.analyze(
        Path::new("a.js"),
        "const { createHash } = require('crypto'); createHash('sha1');",
    );
    assert_eq!(findings.len(), 1);
}

#[test]
fn every_detected_algorithm_exists_in_the_knowledge_base() {
    let source = "
        const c = require('crypto');
        c.generateKeyPairSync('rsa', { modulusLength: 2048 });
        c.generateKeyPairSync('rsa-pss', { modulusLength: 2048 });
        c.generateKeyPairSync('dsa', { modulusLength: 2048 });
        c.generateKeyPairSync('ec', { namedCurve: 'P-256' });
        c.generateKeyPairSync('ed25519');
        c.generateKeyPairSync('x25519');
        c.generateKeyPairSync('dh');
        c.generateKeySync('aes', { length: 128 });
        c.generateKeySync('aes', { length: 192 });
        c.generateKeySync('aes', { length: 256 });
        c.createDiffieHellman(2048);
        c.createECDH('prime256v1');
        c.createSign('RSA-SHA256');
        c.publicEncrypt(k, b);
        c.createCipheriv('des-ede3-cbc', k, iv);
        c.createCipheriv('chacha20-poly1305', k, iv);
        c.createHash('md5');
        c.createHash('sha1');
        c.createHash('sha256');
        c.createHash('sha384');
        c.createHash('sha512');
        c.createHash('sha3-256');
    ";
    let knowledge = agentq::knowledge::KnowledgeBase::embedded();
    let findings = JavaScriptAnalyzer.analyze(Path::new("a.js"), source);
    assert!(findings.len() >= 20);
    for finding in findings {
        assert!(
            knowledge.get(&finding.algorithm).is_some(),
            "`{}` is not in the knowledge base",
            finding.algorithm
        );
    }
}

#[test]
fn web_crypto_rsa_and_elliptic_curve_keys() {
    let source = "
        await crypto.subtle.generateKey(
          { name: 'RSA-OAEP', modulusLength: 4096, publicExponent: e, hash: 'SHA-256' },
          true, ['encrypt']);
        await crypto.subtle.generateKey(
          { name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, hash: 'SHA-256' }, true, ['sign']);
        await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-384' }, true, ['sign']);
        await crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, true, ['deriveKey']);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Encryption, Some(4096)),
            found("rsa", Purpose::Signature, Some(2048)),
            found("ecdsa", Purpose::Signature, Some(384)),
            found("ecdh", Purpose::KeyExchange, Some(256)),
        ]
    );
}

#[test]
fn web_crypto_through_global_object_and_node_exports() {
    let source = "
        import { webcrypto } from 'node:crypto';
        import nodeCrypto from 'node:crypto';
        globalThis.crypto.subtle.digest('SHA-1', data);
        window.crypto.subtle.generateKey({ name: 'Ed25519' }, true, ['sign']);
        webcrypto.subtle.sign('RSA-PSS', key, data);
        nodeCrypto.subtle.deriveBits({ name: 'X25519', public: k }, priv, 256);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("sha1", Purpose::Hash, None),
            found("ed25519", Purpose::Signature, Some(256)),
            found("rsa", Purpose::Signature, None),
            found("x25519", Purpose::KeyExchange, Some(256)),
        ]
    );
}

#[test]
fn web_crypto_reads_the_algorithm_from_the_right_argument() {
    let source = "
        const subtle = crypto.subtle;
        await subtle.importKey('spki', der, { name: 'RSA-OAEP', hash: 'SHA-256' }, true, ['encrypt']);
        await subtle.importKey('raw', raw, 'AES-GCM', false, ['encrypt']);
        await subtle.wrapKey('raw', key, wrapKey, { name: 'RSA-OAEP' });
        await subtle.generateKey({ name: 'AES-GCM', length: 256 }, true, ['encrypt']);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Encryption, None),
            found("rsa", Purpose::Encryption, None),
            found("aes-256", Purpose::Encryption, Some(256)),
        ]
    );
}

#[test]
fn web_crypto_ignores_non_crypto_subtle_lookalikes() {
    let source = "
        const subtle = { sign: () => 1 };
        subtle.sign('RSA-PSS');
        crypto.getRandomValues(new Uint8Array(8));
        crypto.randomUUID();
        crypto.subtle.sign(algorithmFromConfig, key, data);
    ";
    assert_eq!(detect(source), vec![]);
}

#[test]
fn jsonwebtoken_algorithms_for_signing_and_verifying() {
    let source = "
        import jwt from 'jsonwebtoken';
        jwt.sign(payload, privateKey, { algorithm: 'RS256', expiresIn: '1h' });
        jwt.sign(payload, secret);
        jwt.sign(payload, secret, { algorithm: 'HS256' });
        jwt.verify(token, publicKey, { algorithms: ['ES384', 'PS256'] });
        jwt.verify(token, secret);
        jwt.decode(token);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Signature, None),
            found("ecdsa", Purpose::Signature, Some(384)),
            found("rsa", Purpose::Signature, None),
        ]
    );
}

#[test]
fn jose_functions_take_the_algorithm_from_their_arguments() {
    let source = "
        import { generateKeyPair, importSPKI, jwtVerify } from 'jose';
        await generateKeyPair('EdDSA');
        await importSPKI(pem, 'RS256');
        await generateKeyPair('ECDH-ES');
        await jwtVerify(token, key, { algorithms: ['ES256'] });
        await generateKeyPair('HS256');
    ";
    assert_eq!(
        detect(source),
        vec![
            found("ed25519", Purpose::Signature, Some(256)),
            found("rsa", Purpose::Signature, None),
            found("ecdh", Purpose::KeyExchange, None),
            found("ecdsa", Purpose::Signature, Some(256)),
        ]
    );
}

#[test]
fn node_forge_keys_and_hashes() {
    let source = "
        const forge = require('node-forge');
        forge.pki.rsa.generateKeyPair({ bits: 2048 });
        forge.pki.rsa.generateKeyPair(4096);
        forge.pki.ed25519.generateKeyPair();
        forge.md.sha1.create();
        forge.md5.create();
        forge.util.encode64(x);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Unknown, Some(2048)),
            found("rsa", Purpose::Unknown, Some(4096)),
            found("ed25519", Purpose::Signature, Some(256)),
            found("sha1", Purpose::Hash, None),
            found("md5", Purpose::Hash, None),
        ]
    );
}

#[test]
fn constructors_of_node_rsa_and_elliptic() {
    let source = "
        import NodeRSA from 'node-rsa';
        const EC = require('elliptic').ec;
        const elliptic = require('elliptic');
        new NodeRSA({ b: 1024 });
        new EC('secp256k1');
        new elliptic.ec('p384');
        new elliptic.eddsa('ed25519');
        NodeRSA({ b: 512 });
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Unknown, Some(1024)),
            found("ecdsa", Purpose::Unknown, Some(256)),
            found("ecdsa", Purpose::Unknown, Some(384)),
            found("ed25519", Purpose::Signature, Some(256)),
        ]
    );
}

#[test]
fn tweetnacl_signatures_and_boxes() {
    let source = "
        import nacl from 'tweetnacl';
        nacl.sign.keyPair();
        nacl.sign.detached(msg, sk);
        nacl.box.keyPair();
        nacl.box(msg, nonce, pk, sk);
        nacl.randomBytes(24);
        nacl.secretbox(msg, nonce, key);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("ed25519", Purpose::Signature, Some(256)),
            found("ed25519", Purpose::Signature, Some(256)),
            found("x25519", Purpose::KeyExchange, Some(256)),
            found("x25519", Purpose::KeyExchange, Some(256)),
        ]
    );
}

#[test]
fn noble_curves_packages() {
    let source = "
        import { secp256k1 } from '@noble/curves/secp256k1';
        import { p256 } from '@noble/curves/nist';
        import { x25519, ed25519 } from '@noble/curves/ed25519';
        import * as ed from '@noble/ed25519';
        import * as secp from '@noble/secp256k1';
        secp256k1.sign(msg, priv);
        p256.getSharedSecret(a, b);
        x25519.getSharedSecret(a, b);
        ed25519.verify(sig, msg, pub);
        ed.signAsync(msg, priv);
        secp.getSharedSecret(a, b);
        secp256k1.utils.randomPrivateKey();
    ";
    assert_eq!(
        detect(source),
        vec![
            found("ecdsa", Purpose::Signature, Some(256)),
            found("ecdh", Purpose::KeyExchange, Some(256)),
            found("x25519", Purpose::KeyExchange, Some(256)),
            found("ed25519", Purpose::Signature, Some(256)),
            found("ed25519", Purpose::Signature, Some(256)),
            found("ecdh", Purpose::KeyExchange, Some(256)),
        ]
    );
}

#[test]
fn library_detections_exist_in_the_knowledge_base() {
    let source = "
        const forge = require('node-forge');
        const NodeRSA = require('node-rsa');
        const elliptic = require('elliptic');
        const nacl = require('tweetnacl');
        const jwt = require('jsonwebtoken');
        const { secp256k1 } = require('@noble/curves/secp256k1');
        const { x25519 } = require('@noble/curves/ed25519');
        forge.pki.rsa.generateKeyPair(2048);
        forge.pki.ed25519.generateKeyPair();
        forge.md.sha256.create();
        new NodeRSA({ b: 2048 });
        new elliptic.ec('p256');
        new elliptic.eddsa('ed25519');
        nacl.sign.keyPair();
        nacl.box.keyPair();
        secp256k1.sign(m, k);
        secp256k1.getSharedSecret(a, b);
        x25519.getSharedSecret(a, b);
        jwt.sign(p, k, { algorithm: 'ES256' });
        jwt.sign(p, k, { algorithm: 'RS256' });
        jwt.sign(p, k, { algorithm: 'ES512' });
    ";
    let knowledge = agentq::knowledge::KnowledgeBase::embedded();
    let findings = JavaScriptAnalyzer.analyze(Path::new("a.js"), source);
    assert_eq!(findings.len(), 14);
    for finding in findings {
        assert!(
            knowledge.get(&finding.algorithm).is_some(),
            "{}",
            finding.algorithm
        );
    }
}

#[test]
fn crypto_js_hashes_and_triple_des() {
    let source = "
        import CryptoJS from 'crypto-js';
        import { SHA1, HmacSHA256 } from 'crypto-js';
        import md5 from 'crypto-js/md5';
        const sha512 = require('crypto-js/sha512');
        CryptoJS.MD5(message);
        SHA1(message);
        HmacSHA256(message, key);
        md5(message);
        sha512(message);
        CryptoJS.TripleDES.encrypt(message, key);
        CryptoJS.enc.Base64.stringify(x);
        CryptoJS.lib.WordArray.random(16);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("md5", Purpose::Hash, None),
            found("sha1", Purpose::Hash, None),
            found("sha2-256", Purpose::Mac, None),
            found("md5", Purpose::Hash, None),
            found("sha2-512", Purpose::Hash, None),
            found("3des", Purpose::Encryption, None),
        ]
    );
}

#[test]
fn rsa_key_purpose_is_inferred_from_the_rest_of_the_file() {
    let encrypting = "
        const crypto = require('crypto');
        const { publicKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
        crypto.publicEncrypt(publicKey, data);
    ";
    assert_eq!(
        detect(encrypting),
        vec![
            found("rsa", Purpose::Encryption, Some(2048)),
            found("rsa", Purpose::Encryption, None),
        ]
    );

    let signing = "
        const crypto = require('crypto');
        const { privateKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
        crypto.sign('sha256', data, privateKey);
    ";
    assert_eq!(
        detect(signing),
        vec![
            found("rsa", Purpose::Signature, Some(2048)),
            found("sha2-256", Purpose::Hash, None),
        ]
    );
}

#[test]
fn ec_key_purpose_and_algorithm_are_inferred() {
    let key_exchange = "
        const crypto = require('crypto');
        const { privateKey } = crypto.generateKeyPairSync('ec', { namedCurve: 'P-256' });
        crypto.diffieHellman({ privateKey, publicKey });
    ";
    assert_eq!(
        detect(key_exchange),
        vec![found("ecdh", Purpose::KeyExchange, Some(256))]
    );

    let signing = "
        const crypto = require('crypto');
        const { privateKey } = crypto.generateKeyPairSync('ec', { namedCurve: 'P-256' });
        crypto.createSign('SHA256').sign(privateKey);
    ";
    assert_eq!(
        detect(signing),
        vec![
            found("ecdsa", Purpose::Signature, Some(256)),
            found("sha2-256", Purpose::Hash, None),
        ]
    );
}

#[test]
fn inferred_purpose_lowers_confidence() {
    // The key is passed to the call that shows its purpose: medium.
    let linked = JavaScriptAnalyzer.analyze(
        Path::new("a.js"),
        "const c = require('crypto');
         const { publicKey } = c.generateKeyPairSync('rsa', { modulusLength: 2048 });
         c.publicEncrypt(publicKey, b);",
    );
    assert_eq!(linked[0].purpose, Purpose::Encryption);
    assert_eq!(linked[0].confidence, Confidence::Medium);
    assert_eq!(linked[1].confidence, Confidence::High);

    // Nothing links the key to the call, so it is a file-level guess: low.
    let guessed = JavaScriptAnalyzer.analyze(
        Path::new("a.js"),
        "const c = require('crypto');
         c.generateKeyPairSync('rsa', { modulusLength: 2048 });
         c.publicEncrypt(k, b);",
    );
    assert_eq!(guessed[0].purpose, Purpose::Encryption);
    assert_eq!(guessed[0].confidence, Confidence::Low);
}

#[test]
fn typescript_import_equals_require_is_followed() {
    let source = "import crypto = require('crypto');\ncrypto.createHash('md5');";
    assert_eq!(detect(source), vec![found("md5", Purpose::Hash, None)]);
}

#[test]
fn string_concatenation_and_template_literals_are_evaluated() {
    let source = "
        const crypto = require('crypto');
        const BITS = 256;
        crypto.createCipheriv('aes-' + BITS + '-gcm', k, iv);
        crypto.createCipheriv(`aes-${BITS}-cbc`, k, iv);
        crypto.createCipheriv('aes-' + `${BITS / 2}`, k, iv);
        crypto.createHash('sha' + 1);
        crypto.createHash(prefix + '1');
    ";
    assert_eq!(
        detect(source),
        vec![
            found("aes-256", Purpose::Encryption, Some(256)),
            found("aes-256", Purpose::Encryption, Some(256)),
            found("sha1", Purpose::Hash, None),
        ]
    );
}

#[test]
fn properties_of_constant_objects_are_read() {
    let source = "
        const crypto = require('crypto');
        const config = { hash: 'sha1', keys: { type: 'rsa', bits: 2048 } };
        crypto.createHash(config.hash);
        crypto.generateKeyPairSync(config.keys.type, { modulusLength: config.keys.bits });
        crypto.createHash(config.missing);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("sha1", Purpose::Hash, None),
            found("rsa", Purpose::Unknown, Some(2048)),
        ]
    );
}

#[test]
fn reassigned_variables_do_not_produce_findings() {
    let source = "
        const crypto = require('crypto');
        let alg = 'md5';
        alg = process.env.HASH;
        crypto.createHash(alg);
        var KEY_TYPE = 'rsa';
        crypto.generateKeyPairSync(KEY_TYPE, { modulusLength: 2048 });
    ";
    assert_eq!(
        detect(source),
        vec![found("rsa", Purpose::Unknown, Some(2048))]
    );
}

#[test]
fn a_parameter_named_like_the_module_is_not_the_module() {
    let source = "
        const crypto = require('crypto');
        function fake(crypto) { return crypto.createHash('md5'); }
        const arrow = (crypto) => crypto.createHash('md5');
        const single = crypto => crypto.createHash('md5');
        crypto.createHash('sha1');
    ";
    assert_eq!(detect(source), vec![found("sha1", Purpose::Hash, None)]);
}

#[test]
fn post_quantum_usage_is_reported_as_safe_algorithms() {
    let source = "
        const crypto = require('node:crypto');
        crypto.generateKeyPairSync('ml-kem-768');
        crypto.generateKeyPairSync('ml-dsa-65');
        crypto.generateKeyPairSync('slh-dsa-sha2-128s');
        const { ml_kem768 } = require('@noble/post-quantum/ml-kem');
        const { ml_dsa44 } = require('@noble/post-quantum/ml-dsa');
        const { slh_dsa_sha2_128f } = require('@noble/post-quantum/slh-dsa');
        ml_kem768.keygen();
        ml_kem768.encapsulate(pk);
        ml_dsa44.sign(msg, sk);
        slh_dsa_sha2_128f.verify(sig, msg, pk);
        ml_kem768.lengths;
    ";
    assert_eq!(
        detect(source),
        vec![
            found("ml-kem", Purpose::KeyExchange, None),
            found("ml-dsa", Purpose::Signature, None),
            found("slh-dsa", Purpose::Signature, None),
            found("ml-kem", Purpose::KeyExchange, None),
            found("ml-kem", Purpose::KeyExchange, None),
            found("ml-dsa", Purpose::Signature, None),
            found("slh-dsa", Purpose::Signature, None),
        ]
    );
}

#[test]
fn post_quantum_usage_needs_no_migration() {
    use agentq::assess::{Urgency, assess};
    use agentq::i18n::Lang;
    use agentq::knowledge::KnowledgeBase;
    let findings = JavaScriptAnalyzer.analyze(
        Path::new("a.js"),
        "require('node:crypto').generateKeyPairSync('ml-kem-768');",
    );
    let assessment = assess(&findings[0], KnowledgeBase::embedded(), Lang::En).unwrap();
    assert_eq!(assessment.urgency, Urgency::None);
}

#[test]
fn jsrsasign_openpgp_and_libsodium() {
    let source = "
        const { KEYUTIL } = require('jsrsasign');
        const openpgp = require('openpgp');
        const sodium = require('libsodium-wrappers');
        KEYUTIL.generateKeypair('RSA', 2048);
        KEYUTIL.generateKeypair('EC', 'secp256r1');
        openpgp.generateKey({ type: 'rsa', rsaBits: 4096, userIDs: [u] });
        openpgp.generateKey({ type: 'ecc', curve: 'curve25519', userIDs: [u] });
        openpgp.generateKey({ type: 'ecc', curve: 'nistP384', userIDs: [u] });
        sodium.crypto_sign_keypair();
        sodium.crypto_box_seal(msg, pk);
        sodium.randombytes_buf(16);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Unknown, Some(2048)),
            found("ecdsa", Purpose::Unknown, Some(256)),
            found("rsa", Purpose::Unknown, Some(4096)),
            found("ed25519", Purpose::Signature, Some(256)),
            found("x25519", Purpose::KeyExchange, Some(256)),
            found("ecdsa", Purpose::Unknown, Some(384)),
            found("ed25519", Purpose::Signature, Some(256)),
            found("x25519", Purpose::KeyExchange, Some(256)),
        ]
    );
}

#[test]
fn better_auth_jwt_plugin_defaults_to_ed25519() {
    let source = "
        import { betterAuth } from 'better-auth';
        import { jwt } from 'better-auth/plugins';
        export const auth = betterAuth({ plugins: [jwt()] });
    ";
    assert_eq!(
        detect(source),
        vec![found("ed25519", Purpose::Signature, Some(256))]
    );
}

#[test]
fn better_auth_jwt_plugin_reads_the_configured_algorithm() {
    let source = "
        import { jwt } from 'better-auth/plugins';
        jwt({ jwks: { keyPairConfig: { alg: 'RS256', modulusLength: 2048 } } });
    ";
    assert_eq!(detect(source), vec![found("rsa", Purpose::Signature, None)]);
}

#[test]
fn better_auth_without_the_jwt_plugin_has_no_findings() {
    let source = "
        import { betterAuth } from 'better-auth';
        import { twoFactor } from 'better-auth/plugins';
        export const auth = betterAuth({ plugins: [twoFactor()] });
    ";
    assert_eq!(detect(source), vec![]);
}

#[test]
fn hmac_and_key_derivation_have_their_own_purposes() {
    let source = "
        const crypto = require('crypto');
        crypto.createHmac('md5', key);
        crypto.pbkdf2Sync(password, salt, 1000, 32, 'sha1');
        crypto.hkdfSync('sha512', ikm, salt, info, 64);
        crypto.subtle.importKey('raw', k, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
        crypto.subtle.deriveKey({ name: 'PBKDF2', salt, iterations: 1, hash: 'SHA-1' }, k, a, false, u);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("md5", Purpose::Mac, None),
            found("sha1", Purpose::KeyDerivation, None),
            found("sha2-512", Purpose::KeyDerivation, None),
            found("sha2-256", Purpose::Mac, None),
            found("sha1", Purpose::KeyDerivation, None),
        ]
    );
}

#[test]
fn signing_calls_name_the_key_type_or_only_the_digest() {
    let source = "
        const crypto = require('crypto');
        crypto.createSign('ecdsa-with-SHA1');
        crypto.createVerify('DSA-SHA1');
        crypto.sign('sha512', data, key);
        crypto.sign(null, data, key);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("ecdsa", Purpose::Signature, None),
            found("dsa", Purpose::Signature, None),
            found("sha2-512", Purpose::Hash, None),
        ]
    );
}

#[test]
fn legacy_ciphers_hashes_and_448_bit_curves() {
    let source = "
        const crypto = require('crypto');
        crypto.createCipheriv('rc4', key, '');
        crypto.createCipheriv('des-cbc', key, iv);
        crypto.createCipheriv('bf-cbc', key, iv);
        crypto.createHash('ripemd160');
        crypto.createHash('sha224');
        crypto.generateKeyPairSync('ed448');
        crypto.generateKeyPairSync('x448');
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rc4", Purpose::Encryption, None),
            found("des", Purpose::Encryption, Some(56)),
            found("blowfish", Purpose::Encryption, None),
            found("ripemd160", Purpose::Hash, None),
            found("sha2-224", Purpose::Hash, None),
            found("ed448", Purpose::Signature, Some(448)),
            found("x448", Purpose::KeyExchange, Some(448)),
        ]
    );
}

#[test]
fn algorithms_missing_from_the_knowledge_base_are_reported_as_unrecognized() {
    let source = "
        const crypto = require('crypto');
        crypto.createCipheriv('camellia-256-cbc', key, iv);
        crypto.createHash('whirlpool');
        crypto.createHash(process.env.HASH);
    ";
    // A name that cannot be read statically is still not reported.
    assert_eq!(
        detect(source),
        vec![
            found("unrecognized", Purpose::Encryption, None),
            found("unrecognized", Purpose::Hash, None),
        ]
    );
}

#[test]
fn two_keys_in_one_file_get_the_purpose_of_the_call_that_receives_each() {
    let source = "
        const crypto = require('crypto');
        const jwt = require('jsonwebtoken');
        const signing = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
        const { publicKey: sealKey } = crypto.generateKeyPairSync('rsa', { modulusLength: 4096 });
        const unused = crypto.generateKeyPairSync('rsa', { modulusLength: 3072 });
        jwt.sign(payload, signing.privateKey, { algorithm: 'RS256' });
        crypto.publicEncrypt(sealKey, secret);
    ";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Signature, Some(2048)),
            found("rsa", Purpose::Encryption, Some(4096)),
            // No call receives it, and the others are explained by their own keys.
            found("rsa", Purpose::Unknown, Some(3072)),
            found("rsa", Purpose::Signature, None),
            found("rsa", Purpose::Encryption, None),
        ]
    );
}
