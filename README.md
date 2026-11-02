<p align="center">
  <img alt="agentq — Spot quantum-vulnerable cryptography" src="assets/banner/banner.png" />
</p>

<p align="center">
  <img alt="Rust 2024" src="https://shieldcn.dev/badge/Rust-2024-ff8a3d.svg?variant=outline&amp;logo=rust" />
  <img alt="ratatui 0.30" src="https://shieldcn.dev/badge/ratatui-0.30-ff8a3d.svg?variant=outline" />
  <img alt="crossterm 0.29" src="https://shieldcn.dev/badge/crossterm-0.29-ff8a3d.svg?variant=outline" />
  <img alt="ignore 0.4" src="https://shieldcn.dev/badge/ignore-0.4-ff8a3d.svg?variant=outline" />
</p>

# agentq

A proposed tool for analyzing source code, identifying uses of cryptography that may be vulnerable to quantum computing, and recommending post-quantum alternatives suited to the system's context.

> **Status:** this project is in the concept and scope definition stage for an undergraduate final project (TCC, in Brazil). Source scanning and cryptography detection for JavaScript/TypeScript are implemented (see [Current prototype](#current-prototype)); the remaining features described below are planned.

## About the project

**agentq** started with the idea of helping developers understand how their systems use cryptography and how they can prepare for the challenges posed by quantum computing.

The proposal is to analyze another system's source code, locate cryptographic operations, assess their potential vulnerability to quantum attacks, and suggest alternatives with greater resistance to those attacks. Recommendations should take into account the purpose of each operation and the requirements of the system being analyzed.

## Motivation

Sufficiently powerful quantum computers could compromise public-key cryptographic mechanisms in use today. Post-quantum cryptography aims to provide alternatives that resist these attacks and run on conventional computers. This field is the subject of research and standardization efforts by [NIST](https://www.nist.gov/pqc).

Identifying where and how cryptography is used is an important step in planning a migration. agentq aims to support this work through code analysis and recommendations tailored to each use case.

## Goal

Research and develop a tool that helps identify uses of cryptography vulnerable to quantum computing and select appropriate post-quantum alternatives.

### Specific objectives

- Identify cryptographic algorithms, libraries, and calls in source code.
- Locate the files and code sections associated with each finding.
- Analyze the purpose of each operation, such as key establishment or digital signatures.
- Explain the risks associated with the identified mechanisms.
- Recommend alternatives considering security, compatibility, and performance.
- Present the results in a report that developers can understand.

## Proposed workflow

1. **Input:** receive a project's source code for analysis.
2. **Identification:** locate uses of cryptography in the languages and libraries supported by the tool.
3. **Assessment:** map the identified mechanisms to known risks from quantum attacks.
4. **Recommendation:** suggest post-quantum alternatives suited to the purpose of each use and explain the reasoning behind them.
5. **Report:** compile the findings, their locations, associated risks, and recommendations.

Recommendations should support the technical assessment of the team responsible for the system. The suitability of an alternative depends on the context, and the analysis does not guarantee security.

## Current prototype

The prototype covers the first two steps of the workflow for **JavaScript and TypeScript (Node.js)** projects.

1. **Scanner.** Walks a directory, respects `.gitignore`, skips dependency and build folders (`node_modules`, `vendor`, `target`, `build`, `dist`, `.git`), and lists files in the supported languages (Python, Java, JavaScript/TypeScript, and Go).
2. **Detector.** Parses each JS/TS file with [tree-sitter](https://tree-sitter.github.io) and reports cryptographic calls with their location, algorithm, purpose, and key size when it is a literal. It follows imports, destructuring, aliases, and constants, so `const { createHash: h } = require('node:crypto'); h('md5')` is found, while the same words in comments or strings are not.

Recognized APIs: Node.js `crypto`, Web Crypto (`crypto.subtle`), `jsonwebtoken`, `jose`, `node-forge`, `node-rsa`, `elliptic`, `tweetnacl`, `@noble/curves`, `@noble/ed25519`, `@noble/secp256k1`, and `crypto-js`.

Algorithms are described in a knowledge base ([`src/knowledge/algorithms.toml`](src/knowledge/algorithms.toml)) that records the quantum attack that affects each one (Shor or Grover), its typical purposes, and the NIST post-quantum alternatives. The assessment and report steps do not use it yet.

```sh
cargo run -- <directory>   # defaults to the current directory
```

Example, scanning the test fixtures:

```sh
$ cargo run -- tests/fixtures/node_crypto
JS/TS    100.0%  tests/fixtures/node_crypto/server.js
JS/TS    100.0%  tests/fixtures/node_crypto/session.ts

2 arquivo(s) detectado(s) em tests/fixtures/node_crypto

5 cryptographic use(s) found
  rsa       unknown      2048-bit tests/fixtures/node_crypto/server.js:3:35  crypto.generateKeyPairSync('rsa', {
  sha1      hash                  tests/fixtures/node_crypto/server.js:8:10  crypto.createHash('sha1')
  rsa       encryption            tests/fixtures/node_crypto/server.js:12:10  crypto.publicEncrypt(publicKey, buffer)
  ecdh      key exchange 256-bit  tests/fixtures/node_crypto/session.ts:5:16  createECDH('prime256v1')
  aes-256   encryption   256-bit  tests/fixtures/node_crypto/session.ts:8:18  createCipheriv('aes-256-gcm', secret, randomBytes(12))
```

### Known limitations

- Name resolution is file-local and ignores scoping; algorithms chosen at runtime (configuration, function parameters) are not reported.
- Calls on values returned by other calls (`key.sign(...)` on a generated key) are not tracked, so the purpose of RSA and EC key generation stays `unknown` for now.
- Only JavaScript and TypeScript are analyzed; Python, Java, and Go files are listed but not inspected.

## Next steps

- [ ] Review the literature on post-quantum cryptography and code analysis.
- [ ] Define the research problem and evaluation criteria for the final project.
- [x] Define the languages, libraries, and algorithms within the initial scope (JS/TS first).
- [x] Choose the analysis approach and technologies for the prototype (tree-sitter and a rule-based knowledge base).
- [ ] Infer the purpose of each use, then assess risk and recommend post-quantum alternatives.
- [ ] Produce the report (text, JSON, Markdown).
- [ ] Develop a prototype and evaluate it on test projects.
- [ ] Document the results, limitations, and opportunities for future development.

## Team

- Henrico Birochi
- Edgar Camacho Seabra Ribeiro
- Vítor Agostino Braghittoni
- Nicholas Birochi

## Initial references

- [NIST — Post-Quantum Cryptography](https://www.nist.gov/pqc)
- [NCCoE — Migration to Post-Quantum Cryptography](https://www.nccoe.nist.gov/applied-cryptography/migration-to-pqc)
