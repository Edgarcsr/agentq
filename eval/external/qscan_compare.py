#!/usr/bin/env python3
"""Compares agentq and qScan on agentq's annotated corpus and on the probe.

qScan (quantakrypto/pqc-tools) is a third-party package, so it runs inside
Docker with no network, a read-only file system and no capabilities. Build the
image once:

    docker build -t agentq-qscan:0.12.0 eval/external/qscan-docker

Usage: python3 eval/external/qscan_compare.py > eval/external/qscan-compare.md

The comparison covers only the public-key algorithms broken by Shor's
algorithm, which is what qScan reports. Hashes, symmetric ciphers and
post-quantum algorithms in the corpus are left out on both sides.

The `// @expect` markers are stripped before either tool sees the files, since
a lexical scanner would otherwise read the answers in them. The probe's
trailing `// L12 ...` notes are stripped for the same reason.
"""
import collections
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
IMAGE = "agentq-qscan:0.12.0"

# Public-key algorithms, by the family name both tools are mapped to.
AGENTQ_FAMILY = {"rsa": "RSA", "dh": "DH", "ecdh": "EC", "ecdsa": "EC", "dsa": "DSA",
                 "ed25519": "EdDSA", "ed448": "EdDSA", "x25519": "X25519", "x448": "X448"}
QSCAN_FAMILY = {"RSA": "RSA", "DH": "DH", "ECDH": "EC", "ECDSA": "EC", "DSA": "DSA",
                "EdDSA": "EdDSA", "X25519": "X25519", "X448": "X448", "secp256k1": "EC"}
# qScan's category, as an agentq purpose.
QSCAN_PURPOSE = {"kem": "encryption", "key-exchange": "key_exchange", "signature": "signature"}
# What decides the replacement: a key encapsulation mechanism or a signature scheme.
REPLACEMENT = {"key_exchange": "ML-KEM", "encryption": "ML-KEM", "signature": "ML-DSA"}

MARKER = re.compile(r"\s*// @expect.*$")
PROBE_NOTE = re.compile(r"\s*// L\d+.*$")


def copy_stripped(source, pattern):
    target = tempfile.mkdtemp(prefix="agentq-compare-")
    expectations = []
    for name in sorted(os.listdir(source)):
        with open(os.path.join(source, name), encoding="utf-8") as handle:
            lines = handle.read().split("\n")
        for number, line in enumerate(lines, 1):
            marker = line.find("// @expect")
            if marker < 0:
                continue
            for item in line[marker + len("// @expect"):].split(","):
                algorithm, _, purpose = item.strip().partition(":")
                expectations.append((name, number, algorithm, purpose or None))
        with open(os.path.join(target, name), "w", encoding="utf-8") as handle:
            handle.write("\n".join(pattern.sub("", line) for line in lines))
    os.chmod(target, 0o755)
    return target, expectations


def run_agentq(path):
    output = subprocess.run(["cargo", "run", "-q", "--", path, "-f", "json"], cwd=ROOT,
                            check=True, capture_output=True).stdout
    hits = collections.defaultdict(list)
    for item in json.loads(output)["findings"]:
        finding = item["finding"]
        family = AGENTQ_FAMILY.get(finding["algorithm"])
        if family:
            hits[(os.path.basename(finding["file"]), finding["line"])].append(
                (family, finding["purpose"], finding["snippet"]))
    return hits


def run_qscan(path):
    out = tempfile.mkdtemp(prefix="qscan-out-")
    os.chmod(out, 0o777)
    subprocess.run(["docker", "run", "--rm", "--network", "none", "--read-only",
                    "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
                    "-v", f"{path}:/scan/in:ro", "-v", f"{out}:/out", IMAGE, "/scan/in",
                    "--format", "json", "--no-config-file", "--severity-threshold", "critical",
                    "-o", "/out/report.json"], check=True, capture_output=True)
    with open(os.path.join(out, "report.json"), encoding="utf-8") as handle:
        report = json.load(handle)
    shutil.rmtree(out)
    hits = collections.defaultdict(list)
    for finding in report["findings"]:
        location = finding.get("location", {})
        family = QSCAN_FAMILY.get(finding.get("algorithm"), "?")
        purpose = QSCAN_PURPOSE.get(finding.get("category"), "unknown")
        hits[(os.path.basename(location.get("file", "")), location.get("line"))].append(
            (family, purpose, finding.get("ruleId", "")))
    return hits


def pct(numerator, denominator):
    return f"{numerator}/{denominator} ({numerator / denominator:.0%})" if denominator else "-"


def score(expectations, hits):
    asymmetric = [e for e in expectations if e[2] in AGENTQ_FAMILY]
    expected_lines = {(e[0], e[1]) for e in expectations}
    detected = family_right = 0
    purpose_right = purpose_total = replacement_right = 0
    unknown_total = unknown_asserted = 0
    for name, line, algorithm, purpose in asymmetric:
        # A finding with an unclassified algorithm ("?") still counts as detected.
        found = hits.get((name, line), [])
        if found:
            detected += 1
        if any(h[0] == AGENTQ_FAMILY[algorithm] for h in found):
            family_right += 1
        claimed = [h[1] for h in found if h[1] in REPLACEMENT]
        if purpose in REPLACEMENT:
            purpose_total += 1
            if purpose in claimed:
                purpose_right += 1
            if any(REPLACEMENT[c] == REPLACEMENT[purpose] for c in claimed):
                replacement_right += 1
        elif purpose == "unknown" and found:
            unknown_total += 1
            if claimed:
                unknown_asserted += 1
    spurious = sorted((key, value) for key, value in hits.items() if key not in expected_lines)
    return {
        "detected": pct(detected, len(asymmetric)),
        "family": pct(family_right, len(asymmetric)),
        "purpose": pct(purpose_right, purpose_total),
        "replacement": pct(replacement_right, purpose_total),
        "asserted": pct(unknown_asserted, unknown_total),
        "spurious": spurious,
    }


def main():
    corpus, expectations = copy_stripped(os.path.join(ROOT, "eval/corpus/node"), MARKER)
    probe, _ = copy_stripped(os.path.join(ROOT, "eval/probe/node"), PROBE_NOTE)
    results = {}
    for tool, run in (("agentq", run_agentq), ("qScan 0.12.0", run_qscan)):
        results[tool] = (score(expectations, run(corpus)), run(probe))

    print("# agentq and qScan on agentq's corpus and probe\n")
    print("Generated by `eval/external/qscan_compare.py`. Only public-key algorithms "
          "broken by Shor's algorithm are compared.\n")
    print("## Annotated corpus (`eval/corpus/node`)\n")
    print("| | " + " | ".join(results) + " |")
    print("| --- |" + " ---: |" * len(results))
    rows = [
        ("Uses detected", "detected"),
        ("Right algorithm family", "family"),
        ("Right purpose (annotated purpose known)", "purpose"),
        ("Right replacement (ML-KEM or ML-DSA)", "replacement"),
        ("Purpose asserted where the code does not say it", "asserted"),
    ]
    for label, key in rows:
        print(f"| {label} | " + " | ".join(r[0][key] for r in results.values()) + " |")
    print("| Findings on lines with no cryptography | "
          + " | ".join(str(len(r[0]["spurious"])) for r in results.values()) + " |")
    for tool, (scores, _) in results.items():
        if scores["spurious"]:
            print(f"\n{tool}, findings on lines with no cryptography:\n")
            for (name, line), found in scores["spurious"]:
                print(f"- `{name}:{line}` " + ", ".join(f"{f} {p}" for f, p, _ in found))

    print("\n## Probe (`eval/probe/node`), side by side\n")
    print("| Line | " + " | ".join(results) + " |")
    print("| ---: |" + " --- |" * len(results))
    lines = sorted({key for _, hits in results.values() for key in hits})
    for key in lines:
        cells = []
        for _, hits in results.values():
            found = hits.get(key, [])
            cells.append(", ".join(f"{f} ({p})" for f, p, _ in found) or "—")
        print(f"| `{key[0]}:{key[1]}` | " + " | ".join(cells) + " |")
    shutil.rmtree(corpus)
    shutil.rmtree(probe)


if __name__ == "__main__":
    sys.exit(main())
