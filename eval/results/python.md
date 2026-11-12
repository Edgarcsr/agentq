# Evaluation

Corpus: 3 files, 11 lines, 2 annotated cryptographic uses.

## Overall

| | TP | FP | FN | Precision | Recall | F1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| agentq | 2 | 0 | 0 | 100.0% | 100.0% | 100.0% |
| keyword search | 2 | 3 | 0 | 40.0% | 100.0% | 57.1% |
| Python lexical baseline | 2 | 1 | 0 | 66.7% | 100.0% | 80.0% |

## By algorithm (agentq)

| | TP | FP | FN | Precision | Recall | F1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| rsa | 2 | 0 | 0 | 100.0% | 100.0% | 100.0% |

## By algorithm (keyword search)

| | TP | FP | FN | Precision | Recall | F1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| rsa | 2 | 3 | 0 | 40.0% | 100.0% | 57.1% |

## By algorithm (Python lexical baseline)

| | TP | FP | FN | Precision | Recall | F1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| rsa | 2 | 1 | 0 | 66.7% | 100.0% | 80.0% |

## Purpose

Of 1 located uses with an annotated purpose, agentq assigned the right purpose to 1 (100.0%).

## Speed

Analyzing the corpus takes 0.6 ms on average over 20 runs (4656 files/s, 17072 lines/s).

## Missed by agentq (false negatives)

None.

## Reported by agentq but not annotated (false positives)

None.
