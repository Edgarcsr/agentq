# agentq

A proposed tool for analyzing source code, identifying uses of cryptography that may be vulnerable to quantum computing, and recommending post-quantum alternatives suited to the system's context.

> **Status:** this project is in the concept and scope definition stage for an undergraduate final project (TCC, in Brazil). The features described below are planned and have not been implemented yet.

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

The source file scanner is the first piece of the identification step. It walks a directory, respects `.gitignore`, skips dependency and build folders (`node_modules`, `vendor`, `target`, `build`, `dist`, `.git`), and lists the files in the supported languages (Python, Java, JavaScript/TypeScript, and Go).

```sh
cargo run -- <directory>   # defaults to the current directory
```

Example, scanning the test fixtures:

```sh
$ cargo run -- tests/fixtures
Go       33.3%  tests/fixtures/gitignored/main.go
JS/TS    33.3%  tests/fixtures/js_with_node_modules/index.ts
Python   33.3%  tests/fixtures/python_project/pkg/app.py

3 arquivo(s) detectado(s) em tests/fixtures
```

The `node_modules` directory in `js_with_node_modules` is skipped, so only its own `index.ts` is listed.

## Next steps

- [ ] Review the literature on post-quantum cryptography and code analysis.
- [ ] Define the research problem and evaluation criteria for the final project.
- [ ] Define the languages, libraries, and algorithms within the initial scope.
- [ ] Choose the analysis approach and technologies for the prototype.
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
