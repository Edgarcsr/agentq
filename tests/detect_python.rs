use agentq::analyzer::Analyzer;
use agentq::finding::{Confidence, Purpose};
use agentq::modules::python::PythonAnalyzer;
use std::path::Path;

type Found = (String, Purpose, Option<u32>);

/// `(algorithm, purpose, key size)` of every finding in `source`, in source order.
fn detect(source: &str) -> Vec<Found> {
    PythonAnalyzer
        .analyze(Path::new("test.py"), source)
        .into_iter()
        .map(|f| (f.algorithm, f.purpose, f.key_size))
        .collect()
}

fn found(algorithm: &str, purpose: Purpose, key_size: Option<u32>) -> Found {
    (algorithm.to_string(), purpose, key_size)
}

#[test]
fn cryptography_rsa_key_generation_reports_key_size() {
    let source = "
from cryptography.hazmat.primitives.asymmetric import rsa
key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
other = rsa.generate_private_key(65537, 4096)
";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Unknown, Some(2048)),
            found("rsa", Purpose::Unknown, Some(4096)),
        ]
    );
}

#[test]
fn constants_are_followed_into_arguments() {
    let source = "
from cryptography.hazmat.primitives.asymmetric import rsa
BITS = 3072
rsa.generate_private_key(public_exponent=65537, key_size=BITS)
";
    assert_eq!(
        detect(source),
        vec![found("rsa", Purpose::Unknown, Some(3072))]
    );
}

#[test]
fn rsa_purpose_is_inferred_from_padding() {
    let source = "
from cryptography.hazmat.primitives.asymmetric import rsa, padding
key = rsa.generate_private_key(65537, 2048)
padding.OAEP(mgf=None, algorithm=None, label=None)
";
    assert_eq!(
        detect(source),
        vec![found("rsa", Purpose::Encryption, Some(2048))]
    );

    let signing = "
from cryptography.hazmat.primitives.asymmetric import rsa, padding
key = rsa.generate_private_key(65537, 2048)
key.sign(data, padding.PSS(mgf=None, salt_length=0), None)
";
    assert_eq!(
        detect(signing),
        vec![found("rsa", Purpose::Signature, Some(2048))]
    );
}

#[test]
fn elliptic_curve_keys_take_curve_size_and_purpose_from_context() {
    let source = "
from cryptography.hazmat.primitives.asymmetric import ec
private = ec.generate_private_key(ec.SECP384R1())
shared = private.exchange(ec.ECDH(), peer)
";
    assert_eq!(
        detect(source),
        vec![found("ecdh", Purpose::KeyExchange, Some(384))]
    );

    let signing = "
from cryptography.hazmat.primitives.asymmetric import ec
private = ec.generate_private_key(curve=ec.SECP256R1())
private.sign(data, ec.ECDSA(hashes.SHA256()))
";
    let findings = detect(signing);
    assert_eq!(
        findings,
        vec![found("ecdsa", Purpose::Signature, Some(256))]
    );
}

#[test]
fn modern_curves_dsa_and_dh() {
    let source = "
from cryptography.hazmat.primitives.asymmetric import ed25519, x25519, dsa, dh
ed25519.Ed25519PrivateKey.generate()
x25519.X25519PrivateKey.generate()
dsa.generate_private_key(key_size=2048)
dh.generate_parameters(generator=2, key_size=2048)
";
    assert_eq!(
        detect(source),
        vec![
            found("ed25519", Purpose::Signature, Some(256)),
            found("x25519", Purpose::KeyExchange, Some(256)),
            found("dsa", Purpose::Signature, Some(2048)),
            found("dh", Purpose::KeyExchange, Some(2048)),
        ]
    );
}

#[test]
fn hashlib_and_cryptography_hashes() {
    let source = "
import hashlib
from cryptography.hazmat.primitives import hashes
hashlib.md5(b'x')
hashlib.sha256(b'x').hexdigest()
hashlib.new('sha1')
hashlib.pbkdf2_hmac('sha512', pw, salt, 1000)
hashes.SHA1()
hashes.Hash(hashes.SHA384())
hashlib.blake2b(b'x')
";
    assert_eq!(
        detect(source),
        vec![
            found("md5", Purpose::Hash, None),
            found("sha2-256", Purpose::Hash, None),
            found("sha1", Purpose::Hash, None),
            found("sha2-512", Purpose::Hash, None),
            found("sha1", Purpose::Hash, None),
            found("sha2-384", Purpose::Hash, None),
        ]
    );
}

#[test]
fn symmetric_ciphers() {
    let source = "
from cryptography.fernet import Fernet
from cryptography.hazmat.primitives.ciphers import algorithms
Fernet.generate_key()
algorithms.TripleDES(key)
algorithms.AES256(key)
algorithms.AES(key)
";
    assert_eq!(
        detect(source),
        vec![
            found("aes-128", Purpose::Encryption, Some(128)),
            found("3des", Purpose::Encryption, None),
            found("aes-256", Purpose::Encryption, Some(256)),
        ]
    );
}

#[test]
fn pycryptodome_with_either_package_name() {
    let source = "
from Crypto.PublicKey import RSA
from Cryptodome.Cipher import PKCS1_OAEP
from Crypto.Signature import pkcs1_15
from Crypto.Hash import SHA1
key = RSA.generate(2048)
PKCS1_OAEP.new(key)
pkcs1_15.new(key)
SHA1.new(b'x')
";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Unknown, Some(2048)),
            found("rsa", Purpose::Encryption, None),
            found("rsa", Purpose::Signature, None),
            found("sha1", Purpose::Hash, None),
        ]
    );
}

#[test]
fn python_rsa_ecdsa_and_pyjwt_libraries() {
    let source = "
import rsa, jwt, ecdsa
rsa.newkeys(1024)
rsa.sign(msg, priv, 'SHA-1')
ecdsa.SigningKey.generate(curve=ecdsa.NIST384p)
jwt.encode(payload, key, algorithm='RS256')
jwt.encode(payload, secret, algorithm='HS256')
jwt.decode(token, key, algorithms=['ES256', 'PS384'])
";
    assert_eq!(
        detect(source),
        vec![
            found("rsa", Purpose::Signature, Some(1024)),
            found("rsa", Purpose::Signature, None),
            found("ecdsa", Purpose::Signature, Some(384)),
            found("rsa", Purpose::Signature, None),
            found("ecdsa", Purpose::Signature, Some(256)),
            found("rsa", Purpose::Signature, None),
        ]
    );
}

#[test]
fn reports_location_snippet_and_library() {
    let source = "import hashlib\n\nx = hashlib.md5(b'data')\n";
    let findings = PythonAnalyzer.analyze(Path::new("app.py"), source);
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!((finding.line, finding.column), (3, 5));
    assert_eq!(finding.snippet, "hashlib.md5(b'data')");
    assert_eq!(finding.library, "hashlib");
    assert_eq!(finding.confidence, Confidence::High);
}

#[test]
fn ignores_lookalikes() {
    let source = "
# hashlib.md5 and rsa.generate_private_key appear only in comments
text = \"hashlib.md5(x)\"
import hashlib
hashlib.new(algorithm_from_config)
class hashlib2:
    def md5(self): ...
hashlib2().md5()
md5(b'x')
";
    assert_eq!(detect(source), vec![]);
}

#[test]
fn every_detected_algorithm_exists_in_the_knowledge_base() {
    let source = "
from cryptography.hazmat.primitives.asymmetric import rsa, ec, dsa, dh, ed25519, x25519
from cryptography.hazmat.primitives import hashes
from cryptography.fernet import Fernet
from cryptography.hazmat.primitives.ciphers import algorithms
from Crypto.PublicKey import RSA, ECC, DSA
from Crypto.Signature import DSS
from Crypto.Cipher import DES3, ChaCha20
import nacl.signing, nacl.public, rsa as pyrsa, ecdsa, jwt, hashlib
rsa.generate_private_key(65537, 2048)
ec.generate_private_key(ec.SECP256R1())
dsa.generate_private_key(2048)
dh.generate_parameters(2, 2048)
ed25519.Ed25519PrivateKey.generate()
x25519.X25519PrivateKey.generate()
hashes.MD5()
Fernet(key)
algorithms.TripleDES(k)
algorithms.ChaCha20(k, n)
RSA.generate(2048)
ECC.generate(curve='ed25519')
DSA.generate(2048)
DSS.new(key, 'fips-186-3')
DES3.new(k)
ChaCha20.new(key=k)
nacl.signing.SigningKey.generate()
nacl.public.PrivateKey.generate()
pyrsa.newkeys(2048)
ecdsa.SigningKey.generate(curve=ecdsa.NIST256p)
jwt.encode(p, k, algorithm='EdDSA')
hashlib.sha3_256(b'x')
";
    let knowledge = agentq::knowledge::KnowledgeBase::embedded();
    let findings = PythonAnalyzer.analyze(Path::new("a.py"), source);
    assert!(findings.len() >= 25, "{}", findings.len());
    for finding in findings {
        assert!(
            knowledge.get(&finding.algorithm).is_some(),
            "{}",
            finding.algorithm
        );
    }
}
