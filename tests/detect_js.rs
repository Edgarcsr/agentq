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
            found("ecdsa", Purpose::Unknown, Some(384)),
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
            found("sha2-256", Purpose::Hash, None),
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
