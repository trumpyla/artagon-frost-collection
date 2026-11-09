# Raw research inputs

Status: **immutable historical inputs, unreviewed as project policy**. Nothing
in this directory is a Frost contract. The raw files are evidence of what was
supplied; the curated derivatives under [`../reports/`](../reports/) carry the
corrections, portable citations, and current context.

## Where the original bytes live

The repository keeps one byte-exact copy of each supplied report. To avoid a
second raw copy, this directory holds no duplicates; it points at the retained
originals under `docs/reviews/supplied/`.

| Input | Retained original | SHA-256 | Bytes / lines |
| --- | --- | --- | --- |
| `Frost1.md.md` | [`frost-collection-algebra-research-1.md`](../../reviews/supplied/frost-collection-algebra-research-1.md) | `543d67b2f7c3724f9c96849ea77c8dd24c3a36792395c46e6050d1383ebd8f6d` | 50,768 / 920 |
| `Frost2.md.md` | [`frost-collection-algebra-research-2.md`](../../reviews/supplied/frost-collection-algebra-research-2.md) | `046cdaf91f7f2912ce966835951d00dd42ae230f5a76914b5eeb5b77f8862ad3` | 32,906 / 595 |
| `Frost3.md.md` | [`frost-computed-sets-summary.md`](../../reviews/supplied/frost-computed-sets-summary.md) | `6f39fdbcf2d7816a3ea33eb40e371be7115b1b452cf040a7f8fe84a60de8177c` | 11,501 / 115 |
| Complete handoff archive | [`frost-research-handoff.zip`](../../reviews/supplied/frost-research-handoff.zip) | `605767df09f7a631f7c27951c38054216e63de8ece55f9ca4d3f6c7fc65ece0f` | 62,661 bytes, 12 members |

Hashes were recomputed on this checkout on 2026-11-09 and match the archive's
`input-manifest.json`. The [machine manifest](../manifest.json) repeats them so
a script can re-verify them.

The archive retains the task specification (`INTEGRATION-INSTRUCTIONS.md`),
the README rationale draft, `START-HERE.md`, the input manifest, the three
inputs, and the five template starters. It is kept so the original task text
and template originals stay recoverable after the curated versions diverge.

## What the raw files are not

- They are not current project policy, and their API sketches are not
  implemented or compile-tested. The implemented library is one package,
  `frost-collections` (Rust library name `frost`), on Rust 1.99.1, edition 2024.
- `Frost1` and `Frost2` overlap substantially; they are two exports of related
  research, not independent confirmations of each other.
- `Frost3` is a summary and delivery note. The full computed-set report, the
  research template, and the ZIP it links to were **not** supplied and have
  not been recovered. See the [integration audit](../integration-audit.md).
- Their citations are opaque export markers (a private-use prefix, the word
  `cite`, and `turn…search…`/`turn…view…` tokens). The original source mapping
  behind each token is unavailable. The [citation map](../citation-map.json)
  scopes every marker to its input and records its unresolved status; the
  [source catalog](../SOURCES.md) lists the editorial sources that
  independently support specific claims. Independent support is never a
  recovered mapping.
- `Frost2` and `Frost3` contain session-local `sandbox` links to files that
  were not supplied. Those links are recovery targets, not repository paths.

Raw citation markers and session-local links are permitted only in these
three files and the archive. Curated documents must not contain them.

## Reading the raw files

Read them for the questions they raise and the designs they sketch. For each
claim, check the curated report's per-claim status table before relying on
it. When a raw passage conflicts with the curated derivative, the curated
derivative records why; the raw text stays unchanged.
