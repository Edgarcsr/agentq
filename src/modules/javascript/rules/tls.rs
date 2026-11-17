//! Rules for TLS configuration in Node: `tls`, `https` and `http2`.
//!
//! What matters for the quantum threat is not a call but the options that go
//! into it: which cipher suites are allowed (each names a key exchange and an
//! authentication algorithm), which curves are offered, and which signature
//! algorithms. Every option is read only when it is a literal string.

use super::CallSite;
use crate::finding::{Confidence, Purpose};
use crate::modules::common::{Detection, Value, names};

const LIBRARY: &str = "node:tls";

/// Adds a detection unless the same one is already there: a list of twenty
/// cipher suites says "ECDHE" twenty times and should be one finding.
fn push(
    out: &mut Vec<Detection>,
    algorithm: &'static str,
    purpose: Purpose,
    key_size: Option<u32>,
) {
    if out
        .iter()
        .any(|d| d.algorithm == algorithm && d.purpose == purpose && d.key_size == key_size)
    {
        return;
    }
    out.push(Detection {
        algorithm,
        purpose,
        key_size,
        confidence: Confidence::High,
        library: LIBRARY,
    });
}

/// Whether the call is one that takes TLS options.
fn takes_options(site: &CallSite) -> bool {
    match (site.resolved.module.as_str(), site.path()) {
        ("tls", [f]) => matches!(
            f.as_str(),
            "createServer" | "connect" | "createSecureContext"
        ),
        ("https", [f]) => match f.as_str() {
            "createServer" | "request" | "get" => !site.is_new,
            "Agent" => site.is_new,
            _ => false,
        },
        ("http2", [f]) => matches!(f.as_str(), "createSecureServer" | "connect"),
        _ => false,
    }
}

pub fn detect(site: &CallSite) -> Vec<Detection> {
    if !takes_options(site) {
        return vec![];
    }
    let mut out = Vec::new();
    // The options object can be any argument: `connect(port, host, options)`.
    for options in site.args.iter().filter(|a| matches!(a, Value::Object(_))) {
        if let Some(ciphers) = options.get("ciphers").as_str() {
            for suite in ciphers.split(':') {
                cipher_suite(suite, &mut out);
            }
        }
        if let Some(curves) = options.get("ecdhCurve").as_str() {
            for group in curves.split(':') {
                curve_group(group, &mut out);
            }
        }
        if let Some(algorithms) = options.get("sigalgs").as_str() {
            for algorithm in algorithms.split(':') {
                signature_algorithm(algorithm, &mut out);
            }
        }
        // Finite-field Diffie-Hellman parameters, whatever their value.
        if let Value::Object(pairs) = options
            && pairs.iter().any(|(key, _)| key == "dhparam")
        {
            push(&mut out, "dh", Purpose::KeyExchange, None);
        }
    }
    out
}

/// A cipher suite such as `ECDHE-RSA-AES128-GCM-SHA256` (OpenSSL name) or
/// `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256` (IANA name).
fn cipher_suite(name: &str, out: &mut Vec<Detection>) {
    let name = name.trim().to_ascii_uppercase().replace('_', "-");
    // Exclusions and OpenSSL keywords (`!aNULL`, `HIGH`, `DEFAULT`) name no suite.
    if name.is_empty() || name.starts_with(['!', '-', '+', '@']) || !name.contains('-') {
        return;
    }
    // IANA names put the key exchange before WITH: TLS-ECDHE-RSA-WITH-AES-128-...
    let name = match name.strip_prefix("TLS-") {
        Some(rest) => rest.replace("-WITH-", "-"),
        None => name,
    };
    let parts: Vec<&str> = name.split('-').collect();
    let has = |part: &str| parts.contains(&part);

    // TLS 1.3 suites (`AES-256-GCM-SHA384`) leave the key exchange to the
    // groups; older ones name it in front.
    let bulk = bulk_cipher(&parts);
    let key_exchange = if has("ECDHE") || has("ECDH") {
        push(out, "ecdh", Purpose::KeyExchange, None);
        true
    } else if has("DHE") || has("EDH") {
        push(out, "dh", Purpose::KeyExchange, None);
        true
    } else {
        false
    };
    if has("RSA") {
        push(out, "rsa", Purpose::Signature, None);
    } else if has("ECDSA") {
        push(out, "ecdsa", Purpose::Signature, None);
    } else if has("DSS") {
        push(out, "dsa", Purpose::Signature, None);
    } else if !key_exchange && bulk.is_some() && !is_tls13(&name) {
        // No ECDHE or DHE: the session key is sent encrypted with the server's
        // RSA key, so a recorded session can be opened once RSA is broken.
        push(out, "rsa", Purpose::KeyExchange, None);
    }
    if let Some(id) = bulk {
        push(out, id, Purpose::Encryption, None);
    }
    match parts.last().copied() {
        Some("SHA") => push(out, "sha1", Purpose::Mac, None),
        Some("MD5") => push(out, "md5", Purpose::Mac, None),
        _ => {}
    }
}

fn is_tls13(name: &str) -> bool {
    matches!(
        name,
        "AES-128-GCM-SHA256"
            | "AES-256-GCM-SHA384"
            | "CHACHA20-POLY1305-SHA256"
            | "AES-128-CCM-SHA256"
            | "AES-128-CCM-8-SHA256"
    )
}

/// The bulk cipher a suite names, as a knowledge base id.
fn bulk_cipher(parts: &[&str]) -> Option<&'static str> {
    let joined = parts.join("-");
    if parts.iter().any(|p| matches!(*p, "3DES" | "CBC3")) {
        Some("3des")
    } else if parts.contains(&"RC4") {
        Some("rc4")
    } else if parts.contains(&"DES") {
        Some("des")
    } else if joined.contains("AES256") || joined.contains("AES-256") {
        Some("aes-256")
    } else if joined.contains("AES128") || joined.contains("AES-128") {
        Some("aes-128")
    } else if parts.iter().any(|p| p.starts_with("CHACHA20")) {
        Some("chacha20")
    } else {
        None
    }
}

/// One entry of `ecdhCurve`: a curve, `X25519`, or a post-quantum hybrid.
fn curve_group(name: &str, out: &mut Vec<Detection>) {
    let lower = name.trim().to_ascii_lowercase();
    if lower.contains("mlkem") || lower.contains("kyber") {
        // Hybrids such as X25519MLKEM768 keep the classical half only as a fallback.
        push(out, "ml-kem", Purpose::KeyExchange, None);
    } else if lower == "x25519" {
        push(out, "x25519", Purpose::KeyExchange, Some(256));
    } else if lower == "x448" {
        push(out, "x448", Purpose::KeyExchange, Some(448));
    } else if let Some(bits) = names::curve_bits(&lower) {
        push(out, "ecdh", Purpose::KeyExchange, Some(bits));
    }
}

/// One entry of `sigalgs`: `RSA+SHA256`, `ecdsa_secp256r1_sha256`, `ed25519`.
fn signature_algorithm(name: &str, out: &mut Vec<Detection>) {
    let lower = name.trim().to_ascii_lowercase();
    if lower.contains("mldsa") {
        push(out, "ml-dsa", Purpose::Signature, None);
    } else if lower.contains("ecdsa") {
        push(out, "ecdsa", Purpose::Signature, None);
    } else if lower.contains("ed25519") {
        push(out, "ed25519", Purpose::Signature, None);
    } else if lower.contains("ed448") {
        push(out, "ed448", Purpose::Signature, None);
    } else if lower.contains("rsa") {
        push(out, "rsa", Purpose::Signature, None);
    }
}
