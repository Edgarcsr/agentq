use agentq::analyzer::Analyzer;
use agentq::finding::{Confidence, Finding, Purpose, QuantumRisk};
use agentq::knowledge::KnowledgeBase;
use agentq::modules::java::JavaAnalyzer;
use std::path::Path;

fn analyze(source: &str) -> Vec<Finding> {
    JavaAnalyzer.analyze(Path::new("Example.java"), source)
}

fn body(source: &str) -> Vec<Finding> {
    analyze(&format!(
        "import java.security.*; import java.security.spec.*; import javax.crypto.*; \
         import javax.crypto.spec.*; class Example {{ void run() throws Exception {{ {source} }} }}"
    ))
}

fn ids(findings: &[Finding]) -> Vec<&str> {
    findings.iter().map(|f| f.algorithm.as_str()).collect()
}

#[test]
fn recognizes_jca_digests_and_hmac_with_provider_overloads() {
    let findings = body(
        r#"
        MessageDigest.getInstance("MD5");
        MessageDigest.getInstance("SHA-1", "SUN");
        MessageDigest.getInstance("sha-256");
        MessageDigest.getInstance("SHA-384");
        MessageDigest.getInstance("SHA-512");
        MessageDigest.getInstance("SHA3-256");
        Mac.getInstance("HmacSHA256");
    "#,
    );
    assert_eq!(
        ids(&findings),
        [
            "md5", "sha1", "sha2-256", "sha2-384", "sha2-512", "sha3", "sha2-256"
        ]
    );
    assert!(findings[..6].iter().all(|f| f.purpose == Purpose::Hash));
    assert_eq!(findings[6].purpose, Purpose::Mac);
    assert_eq!(findings[0].library, "java.security");
    assert_eq!(findings[6].library, "javax.crypto");
}

#[test]
fn signatures_use_the_signature_scheme_not_the_digest_name() {
    let findings = body(
        r#"
        Signature.getInstance("SHA256withRSA");
        Signature.getInstance("RSASSA-PSS");
        Signature.getInstance("SHA1withDSA");
        Signature.getInstance("SHA384withECDSAinP1363Format");
        Signature.getInstance("Ed25519");
        Signature.getInstance("SHA256withRSAandMGF1", "BC");
    "#,
    );
    assert_eq!(
        ids(&findings),
        ["rsa", "rsa", "dsa", "ecdsa", "ed25519", "rsa"]
    );
    assert!(findings.iter().all(|f| f.purpose == Purpose::Signature));
}

#[test]
fn key_pair_sizes_are_associated_with_the_right_instance() {
    let findings = body(
        r#"
        KeyPairGenerator a = KeyPairGenerator.getInstance("RSA");
        KeyPairGenerator b = KeyPairGenerator.getInstance("DSA");
        var alias = a;
        b.initialize(1024);
        alias.initialize(2048);
    "#,
    );
    assert_eq!(ids(&findings), ["rsa", "dsa"]);
    assert_eq!(findings[0].key_size, Some(2048));
    assert_eq!(findings[0].purpose, Purpose::Unknown);
    assert_eq!(findings[1].key_size, Some(1024));
}

#[test]
fn rsa_key_generation_parameters_and_pss_are_supported() {
    let findings = body(
        r#"
        var generator = KeyPairGenerator.getInstance("RSASSA-PSS");
        generator.initialize(new RSAKeyGenParameterSpec(3072, RSAKeyGenParameterSpec.F4));
    "#,
    );
    assert_eq!(findings[0].purpose, Purpose::Signature);
    assert_eq!(findings[0].key_size, Some(3072));
}

#[test]
fn ec_named_parameters_and_existing_purpose_inference_are_reused() {
    let findings = body(
        r#"
        var generator = KeyPairGenerator.getInstance("EC");
        var curve = new ECGenParameterSpec("secp384r1");
        generator.initialize(curve);
        KeyPair pair = generator.generateKeyPair();
        var agreement = KeyAgreement.getInstance("ECDH");
        agreement.init(pair.getPrivate());
    "#,
    );
    assert_eq!(ids(&findings), ["ecdh", "ecdh"]);
    assert_eq!(findings[0].key_size, Some(384));
    assert_eq!(findings[0].purpose, Purpose::KeyExchange);
    // The key reaches `agreement.init` through `pair`.
    assert_eq!(findings[0].confidence, Confidence::Medium);
}

#[test]
fn a_key_used_by_no_operation_only_gets_a_low_confidence_guess() {
    // `KeyAgreement` lives in `javax.crypto` and the generator in
    // `java.security`; both are JCA, so the file-level guess still pairs them.
    let findings = body(
        r#"
        KeyPairGenerator.getInstance("EC");
        KeyAgreement.getInstance("ECDH");
    "#,
    );
    assert_eq!(findings[0].purpose, Purpose::KeyExchange);
    assert_eq!(findings[0].confidence, Confidence::Low);
}

#[test]
fn keys_follow_variables_through_pairs_casts_and_aliases() {
    let findings = body(
        r#"
        var generator = KeyPairGenerator.getInstance("RSA");
        KeyPair pair = generator.generateKeyPair();
        var same = pair;
        PrivateKey key = (PrivateKey) same.getPrivate();
        Signature signer = Signature.getInstance("SHA256withRSA");
        signer.initSign(key);
    "#,
    );
    assert_eq!(findings[0].purpose, Purpose::Signature);
    assert_eq!(findings[0].confidence, Confidence::Medium);
}

#[test]
fn same_named_variables_in_different_methods_do_not_mix() {
    let findings = analyze(
        r#"
        import java.security.*;
        import javax.crypto.*;
        class Example {
            void sign() throws Exception {
                var generator = KeyPairGenerator.getInstance("RSA");
                KeyPair pair = generator.generateKeyPair();
                var signer = Signature.getInstance("SHA256withRSA");
                signer.initSign(pair.getPrivate());
            }
            void seal() throws Exception {
                var generator = KeyPairGenerator.getInstance("RSA");
                KeyPair pair = generator.generateKeyPair();
                var cipher = Cipher.getInstance("RSA/ECB/OAEPWithSHA-256AndMGF1Padding");
                cipher.init(Cipher.ENCRYPT_MODE, pair.getPublic());
            }
        }
    "#,
    );
    assert_eq!(ids(&findings), ["rsa", "rsa", "rsa", "rsa"]);
    assert_eq!(findings[0].purpose, Purpose::Signature);
    assert_eq!(findings[2].purpose, Purpose::Encryption);
    assert_eq!(findings[0].confidence, Confidence::Medium);
    assert_eq!(findings[2].confidence, Confidence::Medium);
}

#[test]
fn an_operation_with_an_unresolved_algorithm_still_tells_the_purpose() {
    let findings = analyze(
        r#"
        import java.security.*;
        class Example {
            void run(String configured) throws Exception {
                var generator = KeyPairGenerator.getInstance("EC");
                var signer = Signature.getInstance(configured);
                signer.initSign(generator.generateKeyPair().getPrivate());
            }
        }
    "#,
    );
    assert_eq!(ids(&findings), ["ecdsa"]);
    assert_eq!(findings[0].purpose, Purpose::Signature);
    assert_eq!(findings[0].confidence, Confidence::Medium);
}

#[test]
fn pbkdf2_is_a_key_derivation_over_its_hmac_hash() {
    let findings = body(
        r#"
        SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256");
        SecretKeyFactory.getInstance("PBKDF2WithHmacSHA1");
        SecretKeyFactory.getInstance("AES");
    "#,
    );
    assert_eq!(ids(&findings), ["sha2-256", "sha1"]);
    assert!(
        findings
            .iter()
            .all(|f| f.purpose == Purpose::KeyDerivation && f.library == "javax.crypto")
    );
}

#[test]
fn ambiguous_key_purposes_remain_unknown() {
    let findings = body(
        r#"
        KeyPairGenerator.getInstance("RSA");
        Signature.getInstance("SHA256withRSA");
        Cipher.getInstance("RSA/ECB/OAEPWithSHA-256AndMGF1Padding");
    "#,
    );
    assert_eq!(findings[0].purpose, Purpose::Unknown);
}

#[test]
fn key_exchange_algorithms_and_explicit_named_curves() {
    let findings = body(
        r#"
        KeyAgreement.getInstance("DiffieHellman");
        KeyAgreement.getInstance("ECDH");
        KeyAgreement.getInstance("X25519");
        var generator = KeyPairGenerator.getInstance("XDH");
        generator.initialize(NamedParameterSpec.X25519);
        var ed = KeyPairGenerator.getInstance("EdDSA");
        ed.initialize(new NamedParameterSpec("Ed25519"));
    "#,
    );
    assert_eq!(
        ids(&findings),
        ["dh", "ecdh", "x25519", "x25519", "ed25519"]
    );
    assert_eq!(findings[3].key_size, Some(256));
}

#[test]
fn cipher_transformations_are_parsed_by_algorithm() {
    let findings = body(
        r#"
        Cipher.getInstance("RSA/ECB/OAEPWithSHA-256AndMGF1Padding");
        Cipher.getInstance("AES_128/GCM/NoPadding");
        Cipher.getInstance("AES_256/CBC/PKCS5Padding");
        Cipher.getInstance("DESede/CBC/PKCS5Padding");
        Cipher.getInstance("ChaCha20-Poly1305");
    "#,
    );
    assert_eq!(
        ids(&findings),
        ["rsa", "aes-128", "aes-256", "3des", "chacha20"]
    );
    assert!(findings.iter().all(|f| f.purpose == Purpose::Encryption));
}

#[test]
fn aes_generator_and_cipher_follow_the_same_key() {
    let findings = body(
        r#"
        var generator = KeyGenerator.getInstance("AES");
        generator.init(256);
        var key = generator.generateKey();
        var cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key);
    "#,
    );
    assert_eq!(ids(&findings), ["aes-256", "aes-256"]);
    assert!(findings.iter().all(|f| f.key_size == Some(256)));
}

#[test]
fn aes_secret_keys_use_bytes_not_gcm_tag_length() {
    let findings = body(
        r#"
        byte[] bytes = new byte[32];
        var key = new SecretKeySpec(bytes, "AES");
        var cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE, key, new GCMParameterSpec(128, nonce));
        new SecretKeySpec(bytes, 0, 16, "AES");
    "#,
    );
    assert_eq!(ids(&findings), ["aes-256", "aes-256", "aes-128"]);
    assert_eq!(findings[1].key_size, Some(256));
}

#[test]
fn unknown_aes_sizes_and_unmodeled_algorithms_are_not_guessed() {
    assert!(
        body(
            r#"
        Cipher.getInstance("AES/GCM/NoPadding");
        KeyGenerator.getInstance("AES");
        new SecretKeySpec(externalBytes, "AES");
        KeyPairGenerator.getInstance("EdDSA");
        KeyAgreement.getInstance("XDH");
        Signature.getInstance("Ed448");
        MessageDigest.getInstance("SHA-512/256");
        MessageDigest.getInstance("SHA3-224");
        Cipher.getInstance("DES");
        Signature.getInstance("ML-KEM");
    "#
        )
        .is_empty()
    );
}

#[test]
fn post_quantum_factories_map_to_existing_safe_algorithms() {
    let findings = body(
        r#"
        KeyPairGenerator.getInstance("ML-KEM-768");
        KEM.getInstance("ML-KEM");
        KeyFactory.getInstance("ML-DSA");
        Signature.getInstance("ML-DSA-65");
        Signature.getInstance("SLH-DSA-SHA2-128s", "BC");
    "#,
    );
    assert_eq!(
        ids(&findings),
        ["ml-kem", "ml-kem", "ml-dsa", "ml-dsa", "slh-dsa"]
    );
    let knowledge = KnowledgeBase::embedded();
    assert!(
        findings
            .iter()
            .all(|f| knowledge.get(&f.algorithm).unwrap().risk == QuantumRisk::Safe)
    );
}

#[test]
fn qualified_names_work_without_imports() {
    let findings = analyze(
        r#"class Example { void run() {
        java.security.MessageDigest.getInstance("MD5");
        javax.crypto.Cipher.getInstance("RSA");
    } }"#,
    );
    assert_eq!(ids(&findings), ["md5", "rsa"]);
}

#[test]
fn explicit_static_imports_and_static_wildcards_work() {
    for import in [
        "java.security.MessageDigest.getInstance",
        "java.security.MessageDigest.*",
    ] {
        let findings = analyze(&format!(
            "import static {import}; class Example {{ void run() {{ getInstance(\"MD5\"); }} }}"
        ));
        assert_eq!(ids(&findings), ["md5"]);
    }
}

#[test]
fn ambiguous_static_imports_and_local_factory_methods_do_not_match() {
    assert!(
        analyze(
            r#"
        import static java.security.MessageDigest.getInstance;
        import static another.Factory.getInstance;
        class Example { void run() { getInstance("MD5"); } }
    "#
        )
        .is_empty()
    );
    assert!(
        analyze(
            r#"
        import static java.security.MessageDigest.getInstance;
        class Example {
            void run() { getInstance("MD5"); }
            Object getInstance(String name) { return null; }
        }
    "#
        )
        .is_empty()
    );
}

#[test]
fn final_fields_constants_aliases_and_concatenation_are_resolved() {
    let findings = analyze(
        r#"
        import java.security.MessageDigest;
        class Example {
            void run() {
                final String prefix = "SHA-";
                String algorithm = prefix + 256;
                MessageDigest.getInstance(algorithm);
                MessageDigest.getInstance(Example.LEGACY);
                MessageDigest.getInstance(this.LEGACY);
            }
            static final String LEGACY = "M" + "D5";
        }
    "#,
    );
    assert_eq!(ids(&findings), ["sha2-256", "md5", "md5"]);
}

#[test]
fn same_names_in_methods_and_nested_blocks_have_separate_values() {
    let findings = analyze(
        r#"
        import java.security.MessageDigest;
        class Example {
            void a() {
                String alg = "MD5";
                MessageDigest.getInstance(alg);
                { String other = "SHA-1"; MessageDigest.getInstance(other); }
            }
            void b() { String alg = "SHA-256"; MessageDigest.getInstance(alg); }
        }
    "#,
    );
    assert_eq!(ids(&findings), ["md5", "sha1", "sha2-256"]);
}

#[test]
fn reassigned_values_and_parameters_do_not_inherit_constants() {
    let findings = analyze(
        r#"
        import java.security.MessageDigest;
        class Example {
            static final String ALGORITHM = "MD5";
            void a(String ALGORITHM) { MessageDigest.getInstance(ALGORITHM); }
            void b() {
                String alg = "MD5";
                alg = configuration();
                MessageDigest.getInstance(alg);
            }
            void c() { String alg = "SHA-1"; MessageDigest.getInstance(alg); }
        }
    "#,
    );
    assert_eq!(ids(&findings), ["sha1"]);
}

#[test]
fn ignores_comments_strings_custom_imports_and_shadowing_parameters() {
    assert!(
        analyze(
            r#"
        import application.MessageDigest;
        class Example { void run() {
            MessageDigest.getInstance("MD5");
            // java.security.MessageDigest.getInstance("MD5");
            String code = "Cipher.getInstance(\"RSA\")";
        } }
    "#
        )
        .is_empty()
    );
    assert!(
        analyze(
            r#"
        import java.security.MessageDigest;
        class Example {
            void run(Object MessageDigest) { MessageDigest.getInstance("MD5"); }
            void other() { Object MessageDigest = factory(); MessageDigest.getInstance("MD5"); }
        }
    "#
        )
        .is_empty()
    );
}

#[test]
fn lambda_parameters_shadow_imports_and_fields() {
    assert!(
        analyze(
            r#"
        import java.security.MessageDigest;
        class Example {
            static final String ALG = "MD5";
            void run() {
                use(MessageDigest -> MessageDigest.getInstance("MD5"));
                use((ALG) -> MessageDigest.getInstance(ALG));
            }
        }
    "#
        )
        .is_empty()
    );
}

#[test]
fn conflicting_conditional_and_unknown_initializations_do_not_guess_sizes() {
    for initialization in [
        "g.init(128); g.init(256);",
        "g.init(256); g.init(config());",
        "if (flag) { g.init(256); }",
        "if (flag) g.init(256);",
        "g.init(256); if (flag) { g.init(128); }",
        "g.generateKey(); g.init(256);",
    ] {
        let source = format!(r#"var g = KeyGenerator.getInstance("AES"); {initialization}"#);
        assert!(body(&source).is_empty(), "{source}");
    }
}

#[test]
fn nested_types_and_type_parameters_do_not_resolve_to_jdk_imports() {
    assert!(
        analyze(
            r#"
        import java.security.*;
        class Example {
            void run() { MessageDigest.getInstance("MD5"); }
            class MessageDigest { static Object getInstance(String name) { return null; } }
        }
    "#
        )
        .is_empty()
    );
    assert!(
        analyze(
            r#"
        import java.security.MessageDigest;
        class Example<MessageDigest> {
            void run() { MessageDigest.getInstance("MD5"); }
        }
    "#
        )
        .is_empty()
    );
}

#[test]
fn class_qualified_fields_respect_the_owner_and_shadowing() {
    let findings = analyze(
        r#"
        import java.security.MessageDigest;
        class Example {
            static final String ALG = "MD5";
            class Nested {
                static final String ALG = "SHA-256";
                void run() {
                    MessageDigest.getInstance(Example.ALG);
                    MessageDigest.getInstance(this.ALG);
                }
            }
            void unknown(Object Example) { MessageDigest.getInstance(Example.ALG); }
        }
    "#,
    );
    assert_eq!(ids(&findings), ["md5", "sha2-256"]);
}

#[test]
fn named_parameter_constants_can_be_aliased() {
    let findings = body(
        r#"
        var generator = KeyPairGenerator.getInstance("XDH");
        var parameters = NamedParameterSpec.X25519;
        generator.initialize(parameters);
    "#,
    );
    assert_eq!(ids(&findings), ["x25519"]);
}

#[test]
fn array_initializers_and_comments_do_not_change_aes_key_size() {
    let findings = body(
        r#"
        byte[] key = {0, 0, 0, 0, 0, 0, 0, 0, /* middle */ 0, 0, 0, 0, 0, 0, 0, 0};
        new SecretKeySpec(key, /* algorithm */ "AES");
    "#,
    );
    assert_eq!(ids(&findings), ["aes-128"]);
}

#[test]
fn cyclic_and_excessively_expanded_constants_stop_without_a_finding() {
    assert!(
        body(
            r#"
        final String a = b;
        final String b = a;
        MessageDigest.getInstance(a);
    "#
        )
        .is_empty()
    );
    let mut source = "final String a0 = \"MD5\";".to_string();
    for n in 1..30 {
        source.push_str(&format!("final String a{n} = a{} + a{};", n - 1, n - 1));
    }
    source.push_str("MessageDigest.getInstance(a29);");
    assert!(body(&source).is_empty());
}

#[test]
fn integer_spellings_and_escapes_are_supported() {
    let findings = body(
        r#"
        MessageDigest.getInstance("\115D5");
        var g = KeyPairGenerator.getInstance("RSA");
        final int bytes = 0x100;
        g.initialize(bytes * 8);
        KeyPairGenerator.getInstance("DSA").initialize(2_048);
    "#,
    );
    assert_eq!(ids(&findings), ["md5", "rsa", "dsa"]);
    assert_eq!(findings[1].key_size, Some(2048));
    assert_eq!(findings[2].key_size, Some(2048));
}

#[test]
fn reports_exact_location_and_snippet() {
    let findings = analyze(
        "import java.security.MessageDigest;\nclass Example {\n  void run() {\n    MessageDigest.getInstance(\"MD5\");\n  }\n}\n",
    );
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.file, Path::new("Example.java"));
    assert_eq!((finding.line, finding.column), (4, 5));
    assert_eq!(finding.snippet, "MessageDigest.getInstance(\"MD5\")");
    assert_eq!(finding.confidence, Confidence::High);
}

#[test]
fn all_detected_algorithms_exist_in_the_knowledge_base() {
    let findings = body(
        r#"
        KeyPairGenerator.getInstance("RSA");
        KeyPairGenerator.getInstance("DH");
        Signature.getInstance("SHA256withDSA");
        Signature.getInstance("SHA256withECDSA");
        Signature.getInstance("Ed25519");
        Cipher.getInstance("AES_192/GCM/NoPadding");
        KeyGenerator.getInstance("DESede");
        Cipher.getInstance("ChaCha20");
        KeyFactory.getInstance("ML-KEM");
    "#,
    );
    assert_eq!(findings.len(), 9);
    for finding in findings {
        assert!(KnowledgeBase::embedded().get(&finding.algorithm).is_some());
    }
}
