#!/usr/bin/env python3
"""Scores agentq on the recall corpus of qScan (quantakrypto/pqc-tools).

The corpus was written by the qScan authors, "blind to the detector source",
so it is independent of agentq. It is downloaded at a pinned commit and not
vendored here (Apache-2.0). Matching follows qScan's own recall benchmark
(packages/core/test/recall.test.ts): per file, greedy multiset on the crypto
family; a secp256k1 use counts when caught as ECDSA or ECDH; a finding that
could not be classified counts for a non-structural family.

Usage: python3 eval/external/qscan_recall.py [js|python] [corpus_dir]
Needs the GitHub CLI (`gh`) to download the corpus, and runs agentq through
`cargo run`.
"""
import collections, json, os, subprocess, sys, tempfile

REPO = "quantakrypto/pqc-tools"
COMMIT = "6a81b5537eae"
BASE = "packages/core/test/benchmark/"

FAMILY = {"rsa": "RSA", "dh": "DH", "ecdh": "ECDH", "ecdsa": "ECDSA", "dsa": "DSA",
          "ed25519": "EDDSA", "ed448": "EDDSA", "x25519": "X25519", "x448": "X448"}
# TLS configuration, SSH keys and dependency manifests: out of agentq's scope.
STRUCTURAL = {"TLS", "SSH", "DEP"}


def gh_raw(path):
    return subprocess.run(
        ["gh", "api", "-H", "Accept: application/vnd.github.raw",
         f"repos/{REPO}/contents/{path}?ref={COMMIT}"],
        check=True, capture_output=True).stdout


def download(lang, target):
    tree = json.loads(subprocess.run(
        ["gh", "api", f"repos/{REPO}/git/trees/{COMMIT}?recursive=1"],
        check=True, capture_output=True).stdout)
    prefix = f"{BASE}recall/{lang}/"
    for item in tree["tree"]:
        if item["type"] == "blob" and item["path"].startswith(prefix):
            out = os.path.join(target, item["path"][len(prefix):])
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with open(out, "wb") as handle:
                handle.write(gh_raw(item["path"]))
    return json.loads(gh_raw(f"{BASE}recall-labels.json"))


def main():
    lang = sys.argv[1] if len(sys.argv) > 1 else "js"
    root = sys.argv[2] if len(sys.argv) > 2 else tempfile.mkdtemp(prefix=f"qscan-{lang}-")
    labels = download(lang, root)
    report = json.loads(subprocess.run(
        ["cargo", "run", "-q", "--", root, "-f", "json"],
        check=True, capture_output=True).stdout)

    found = collections.defaultdict(list)
    for item in report["findings"]:
        family = FAMILY.get(item["finding"]["algorithm"])
        if family:
            found[os.path.relpath(item["finding"]["file"], root)].append(family)
    for item in report.get("unassessed", []):
        found[os.path.relpath(item["file"], root)].append("*")

    total = detected = in_scope = in_scope_detected = 0
    by_difficulty, by_difficulty_ok, misses = collections.Counter(), collections.Counter(), []
    for key, value in labels.items():
        if not key.startswith(f"recall/{lang}/"):
            continue
        relative = key[len(f"recall/{lang}/"):]
        pool = list(found.get(relative, []))
        expected = value["expected"]
        ok = [False] * len(expected)
        for i, e in enumerate(expected):
            if e["family"] in pool:
                pool.remove(e["family"]); ok[i] = True
        for i, e in enumerate(expected):
            if not ok[i] and e["family"] == "SECP256K1":
                for sibling in ("ECDSA", "ECDH"):
                    if sibling in pool:
                        pool.remove(sibling); ok[i] = True; break
        for i, e in enumerate(expected):
            if not ok[i] and e["family"] not in STRUCTURAL and "*" in pool:
                pool.remove("*"); ok[i] = True
        for i, e in enumerate(expected):
            total += 1; detected += ok[i]
            by_difficulty[e["difficulty"]] += 1; by_difficulty_ok[e["difficulty"]] += ok[i]
            if e["family"] not in STRUCTURAL:
                in_scope += 1; in_scope_detected += ok[i]
            if not ok[i]:
                misses.append(f"- `{relative}`: {e['family']} ({e['difficulty']}) {e['note']}")

    print(f"# agentq on the qScan recall corpus ({lang}, {REPO}@{COMMIT})\n")
    print(f"- Detection recall: {detected}/{total} ({detected / total:.3f})")
    print(f"- Without TLS, SSH and dependency manifests: {in_scope_detected}/{in_scope} "
          f"({in_scope_detected / in_scope:.3f})")
    print("- By difficulty: " + ", ".join(
        f"{k} {by_difficulty_ok[k]}/{by_difficulty[k]}" for k in sorted(by_difficulty)))
    print("\n## Missed\n")
    print("\n".join(misses) or "None.")


if __name__ == "__main__":
    main()
