# Product

<!-- impeccable:product-schema 1 -->

## Platform

desktop

Native macOS-first desktop app written in Rust with GPUI / gpui-kit (feature `gui`, binary `agentq-gui`). Not web, iOS or Android. The CLI and the ratatui TUI remain the primary interfaces; the GUI is an additional front end over the same analysis library. Interface language: English and pt-BR.

## Users
Developers and application-security engineers who point the tool at a project folder (today JavaScript/TypeScript on Node.js) to take an inventory of cryptography that quantum computers can break, and to plan a migration to post-quantum alternatives. They open the app while auditing a repository, return to projects they scanned before, and need to see which ones are done, running or still waiting. A secondary audience is the academic board evaluating the prototype (undergraduate thesis, TCC), which is why results must be explainable, not just listed.

## Product Purpose
agentq scans source code, finds uses of cryptography that are vulnerable to quantum attacks (Shor, Grover) or already weak, explains the risk, and recommends post-quantum alternatives that fit the purpose of each use. Success is a developer who can open a project, see the urgent findings first, understand why each one matters, and know what to migrate to.

## Positioning
Detection is done on the syntax tree (tree-sitter), following imports, aliases, constants and string building, and it infers the purpose of each key (signature, key exchange, encryption…) from the calls that receive it. The advice is contextual: urgency and NIST post-quantum alternatives depend on that purpose. Quality is measured against an annotated corpus (`eval/corpus/node`) with precision, recall and F1, compared with keyword search and with the external qScan tool. A keyword scanner cannot truthfully claim any of this.

## Operating Context
- Input is a local directory; `.gitignore` is respected and dependency/build folders are skipped.
- Results are findings with file, line, algorithm, purpose, key size and confidence, assessed against a knowledge base of 28 algorithms (`src/knowledge/algorithms.toml`).
- Reports are also available as text, JSON and Markdown from the CLI.
- Settings live in `~/.agentq/config.toml`; recent projects and their last scan summary in `~/.agentq/recent.toml`.
- A project has a status: pending (not scanned yet), running (scan in progress) or done.

## Capabilities and Constraints
- Only JS/TS is analyzed; Python, Java and Go files are listed but not analyzed yet. The UI must not imply coverage that does not exist.
- Risk classes: broken by quantum, weakened, safe. Urgency: high, medium, low, none. Confidence: low, medium, high.
- The GUI is opt-in (`--features gui`) so the CLI and TUI stay light.
- Platform and distribution beyond macOS are undecided.

## Brand Commitments
Name is "agentq". The repository has a banner (`assets/banner`) and 3D padlock and key logos in the TUI; the GUI currently uses a plain "q" mark. No voice or identity guidelines were stated beyond a dark interface.

## Evidence on Hand
- Thesis draft in Portuguese: `docs/tcc.md`; devlogs in `docs/`.
- Annotated corpus and results: `eval/corpus`, `eval/results/node.md`; qScan comparison in `eval/external/`.
- Sample project: `samples/node-shop`; test fixtures in `tests/fixtures`.
- No customers, testimonials or usage numbers exist; none may be invented.

## Product Principles
1. Urgent findings come first; the user should see what to migrate before anything else.
2. Every finding is explainable: why it matters, what replaces it, how sure the tool is.
3. Be honest about coverage and confidence; never present a heuristic guess as a certainty.
4. Project state (pending, running, done) is always visible at a glance.
5. The GUI adds no capability the library does not have; CLI, TUI and GUI share one analysis.

## Accessibility & Inclusion
Bilingual (English, pt-BR). No other product-specific accessibility requirement was established; status must not rely on colour alone (pair it with a label or icon).
