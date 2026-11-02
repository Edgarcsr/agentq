//! Rules for JSON Web Token libraries (`jsonwebtoken` and `jose`).
//!
//! The signing algorithm is chosen with a JOSE `alg` identifier such as `RS256`.

use super::CallSite;
use crate::finding::Confidence;
use crate::modules::common::Value;
use crate::modules::common::{Detection, names};

fn detections(library: &'static str, algs: &[&Value]) -> Vec<Detection> {
    algs.iter()
        .filter_map(|alg| alg.as_str())
        .filter_map(names::jose_algorithm)
        .map(|(algorithm, purpose, key_size)| Detection {
            algorithm,
            purpose,
            key_size,
            confidence: Confidence::High,
            library,
        })
        .collect()
}

pub fn detect(site: &CallSite) -> Vec<Detection> {
    let [function] = site.path() else {
        return vec![];
    };
    match (site.resolved.module.as_str(), function.as_str()) {
        // sign(payload, secretOrPrivateKey, { algorithm })
        ("jsonwebtoken", "sign") => detections("jsonwebtoken", &[site.arg(2).get("algorithm")]),
        // verify(token, secretOrPublicKey, { algorithms: [...] })
        ("jsonwebtoken", "verify") => {
            let algorithms: Vec<&Value> = site.arg(2).get("algorithms").as_array().iter().collect();
            detections("jsonwebtoken", &algorithms)
        }
        // generateKeyPair(alg), importSPKI(spki, alg), importPKCS8(pkcs8, alg), importX509(x509, alg)
        ("jose", "generateKeyPair") => detections("jose", &[site.arg(0)]),
        ("jose", "importSPKI" | "importPKCS8" | "importX509") => detections("jose", &[site.arg(1)]),
        // jwtVerify(jwt, key, { algorithms: [...] })
        ("jose", "jwtVerify" | "compactVerify") => {
            let algorithms: Vec<&Value> = site.arg(2).get("algorithms").as_array().iter().collect();
            detections("jose", &algorithms)
        }
        _ => vec![],
    }
}
