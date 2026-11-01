pub mod syntax;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classifica a finalidade criptográfica para orientar a avaliação posterior.
pub enum Operation {
    KeyGeneration,
    KeyAgreement,
    DigitalSignature,
    AsymmetricEncryption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Distingue uma API suspeita de um uso associado a uma operação no código.
pub enum Confidence {
    Candidate,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Evidência rastreável que o analisador encontrou em uma linha do fonte.
pub struct Finding {
    pub line: usize,
    pub library: &'static str,
    pub algorithm: &'static str,
    pub operation: Operation,
    pub confidence: Confidence,
    pub evidence: String,
}

struct Rule {
    pattern: &'static str,
    library: &'static str,
    algorithm: &'static str,
    operation: Operation,
}

// Estas chamadas identificam APIs relevantes, mas isoladamente não provam uso em execução.
const RULES: &[Rule] = &[
    Rule {
        pattern: "rsa.generate_private_key(",
        library: "cryptography",
        algorithm: "RSA",
        operation: Operation::KeyGeneration,
    },
    Rule {
        pattern: "ec.generate_private_key(",
        library: "cryptography",
        algorithm: "Elliptic Curve",
        operation: Operation::KeyGeneration,
    },
    Rule {
        pattern: "dh.generate_parameters(",
        library: "cryptography",
        algorithm: "Diffie-Hellman",
        operation: Operation::KeyAgreement,
    },
    Rule {
        pattern: "RSA.generate(",
        library: "PyCryptodome",
        algorithm: "RSA",
        operation: Operation::KeyGeneration,
    },
    Rule {
        pattern: "ECC.generate(",
        library: "PyCryptodome",
        algorithm: "Elliptic Curve",
        operation: Operation::KeyGeneration,
    },
    Rule {
        pattern: "DSS.new(",
        library: "PyCryptodome",
        algorithm: "DSA/ECDSA",
        operation: Operation::DigitalSignature,
    },
];

struct TrackedObject {
    library: &'static str,
    algorithm: &'static str,
}

// Métodos de uso ajudam a diferenciar uma chave apenas criada de uma operação observável.
const OPERATIONS: &[(&str, Operation)] = &[
    (".sign(", Operation::DigitalSignature),
    (".verify(", Operation::DigitalSignature),
    (".exchange(", Operation::KeyAgreement),
    (".encrypt(", Operation::AsymmetricEncryption),
    (".decrypt(", Operation::AsymmetricEncryption),
];

fn assignment_target(line: &str) -> Option<&str> {
    let (target, _) = line.split_once('=')?;
    let target = target.trim();
    let mut characters = target.chars();
    let first = characters.next()?;
    // Só nomes simples são rastreados; interpretar desempacotamento exigiria uma análise sintática.
    (first == '_' || first.is_ascii_alphabetic())
        .then_some(target)
        .filter(|_| {
            characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
        })
}

fn has_method_call(line: &str, variable: &str, method_pattern: &str) -> bool {
    let call_pattern = format!("{variable}{method_pattern}");
    line.match_indices(call_pattern.as_str()).any(|(start, _)| {
        // Evita associar, por exemplo, `key.sign()` a uma ocorrência dentro de `monkey.sign()`.
        !line[..start].chars().next_back().is_some_and(|character| {
            character == '.' || character == '_' || character.is_ascii_alphanumeric()
        })
    })
}

fn supports_operation(algorithm: &str, operation: Operation) -> bool {
    // Filtra combinações incompatíveis para não afirmar, por exemplo, que RSA faz ECDH.
    match operation {
        Operation::KeyGeneration => false,
        Operation::KeyAgreement => algorithm == "Diffie-Hellman" || algorithm == "Elliptic Curve",
        Operation::DigitalSignature => {
            algorithm == "RSA" || algorithm == "Elliptic Curve" || algorithm == "DSA/ECDSA"
        }
        Operation::AsymmetricEncryption => algorithm == "RSA",
    }
}

/// Finds candidate APIs and operations on objects assigned from those APIs.
///
/// This first version is lexical and only tracks simple assignments to variable names.
pub fn analyze_source(source: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut tracked_objects = BTreeMap::new();

    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        // Comentários isolados não são evidência de uso; comentários inline ainda são uma limitação.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let has_crypto_api = RULES.iter().any(|rule| line.contains(rule.pattern));
        if let Some(target) = assignment_target(line) {
            if !has_crypto_api {
                // Uma reatribuição desconhecida invalida a evidência anterior para evitar confirmar o objeto errado.
                tracked_objects.remove(target);
            }
        }

        for rule in RULES {
            if line.contains(rule.pattern) {
                if let Some(target) = assignment_target(line) {
                    // A associação simples permite procurar usos nas linhas seguintes sem rastrear fluxo de dados.
                    tracked_objects.insert(
                        target.to_string(),
                        TrackedObject {
                            library: rule.library,
                            algorithm: rule.algorithm,
                        },
                    );
                }

                findings.push(Finding {
                    line: line_index + 1,
                    library: rule.library,
                    algorithm: rule.algorithm,
                    operation: rule.operation,
                    confidence: Confidence::Candidate,
                    evidence: trimmed.to_string(),
                });
            }
        }

        for (variable, object) in &tracked_objects {
            for (pattern, operation) in OPERATIONS {
                // A operação só é confirmada quando o receptor foi associado a uma API reconhecida.
                if has_method_call(line, variable, pattern)
                    && supports_operation(object.algorithm, *operation)
                {
                    findings.push(Finding {
                        line: line_index + 1,
                        library: object.library,
                        algorithm: object.algorithm,
                        operation: *operation,
                        confidence: Confidence::Confirmed,
                        evidence: trimmed.to_string(),
                    });
                }
            }
        }
    }

    findings
}

/// Reads and analyzes one Python source file, returning I/O errors to the caller.
pub fn analyze_file(path: impl AsRef<Path>) -> io::Result<Vec<Finding>> {
    let source = fs::read_to_string(path)?;
    Ok(analyze_source(&source))
}

#[cfg(test)]
mod tests {
    use super::{Confidence, Operation, analyze_source};

    #[test]
    fn confirms_operations_on_generated_keys() {
        let source = "private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)\n\
signature = private_key.sign(data, padding, algorithm)\n\
ec_key = ec.generate_private_key(curve)\n\
shared_key = ec_key.exchange(ec.ECDH(), peer_public_key)\n";

        let findings = analyze_source(source);

        assert_eq!(findings.len(), 4);
        assert_eq!(findings[0].line, 1);
        assert_eq!(findings[0].algorithm, "RSA");
        assert_eq!(findings[0].operation, Operation::KeyGeneration);
        assert_eq!(findings[0].confidence, Confidence::Candidate);
        assert_eq!(findings[1].line, 2);
        assert_eq!(findings[1].operation, Operation::DigitalSignature);
        assert_eq!(findings[1].confidence, Confidence::Confirmed);
        assert_eq!(findings[2].line, 3);
        assert_eq!(findings[3].line, 4);
        assert_eq!(findings[3].operation, Operation::KeyAgreement);
        assert_eq!(findings[3].confidence, Confidence::Confirmed);
    }

    #[test]
    fn ignores_comments_and_out_of_scope_hashing() {
        let source = "# rsa.generate_private_key()\n\
digest = hashlib.sha256(data).digest()\n";

        assert!(analyze_source(source).is_empty());
    }

    #[test]
    fn detects_pycryptodome_key_generation() {
        let source = "key = RSA.generate(2048)\n\
key.decrypt(ciphertext)\n\
signer = DSS.new(key, 'fips-186-3')\n\
signature = signer.sign(hash_obj)\n";
        let findings = analyze_source(source);

        assert_eq!(findings.len(), 4);
        assert_eq!(findings[0].library, "PyCryptodome");
        assert_eq!(findings[0].algorithm, "RSA");
        assert_eq!(findings[1].operation, Operation::AsymmetricEncryption);
        assert_eq!(findings[1].confidence, Confidence::Confirmed);
        assert_eq!(findings[2].confidence, Confidence::Candidate);
        assert_eq!(findings[3].operation, Operation::DigitalSignature);
        assert_eq!(findings[3].confidence, Confidence::Confirmed);
    }

    #[test]
    fn does_not_treat_signature_configuration_as_an_operation() {
        let findings = analyze_source("signature_algorithm = ec.ECDSA(hash_algorithm)\n");

        assert!(findings.is_empty());
    }

    #[test]
    fn does_not_match_a_tracked_name_inside_a_longer_identifier() {
        let findings = analyze_source("key = RSA.generate(2048)\nmonkey_key.sign(data)\n");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].confidence, Confidence::Candidate);
    }

    #[test]
    fn does_not_confirm_operations_on_untracked_variables() {
        let findings = analyze_source("unknown_key.sign(data)\n");

        assert!(findings.is_empty());
    }

    #[test]
    fn reassigning_a_tracked_variable_invalidates_its_crypto_identity() {
        let source = "key = RSA.generate(2048)\n\
key = load_key()\n\
key.decrypt(ciphertext)\n";
        let findings = analyze_source(source);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].confidence, Confidence::Candidate);
    }
}
