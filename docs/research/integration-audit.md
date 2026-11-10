# Research integration audit

Audit date: **2026-11-09**. Baseline: `82becb998af5627cc1e58118ae01515d8a3559ca`
on branch `docs/research-corpus-and-rationale`. Toolchain: Rust 1.99.1,
edition 2024, one package `frost-collections` 0.4.0 (library `frost`).

This audit records what the supplied research handoff contained, what was
recovered and what was not, every correction applied in the curated corpus,
the decision matrix for the mandatory hazards, the evidence gaps that remain,
and the checks actually executed on this checkout. It is maintained by the
research-corpus lane; the "Verification updates" section is appended when the
lead's combined verification receipts arrive.

## 1. Inputs and preservation

| Item | Disposition | Hash (SHA-256) |
| --- | --- | --- |
| `Frost1.md.md` | Already tracked byte for byte as `docs/reviews/supplied/frost-collection-algebra-research-1.md` since the earlier attachment integration; not duplicated | `543d67b2…ebd8f6d` (full value in [manifest.json](manifest.json)) |
| `Frost2.md.md` | Already tracked as `docs/reviews/supplied/frost-collection-algebra-research-2.md`; not duplicated | `046cdaf9…8862ad3` |
| `Frost3.md.md` | New; copied unchanged to `docs/reviews/supplied/frost-computed-sets-summary.md` (summary-labelled name) | `6f39fdbc…de8177c` |
| `1-frost-research-handoff.zip` | New; copied unchanged to `docs/reviews/supplied/frost-research-handoff.zip` (62,661 bytes, 12 members) | `605767df…fc65ece0f` |

All three input hashes and lengths match the archive's `input-manifest.json`
(Frost1 50,768 bytes / 920 lines; Frost2 32,906 / 595; Frost3 11,501 / 115).
The hashes were recomputed with `shasum -a 256` on this checkout before and
after copying. Opaque export markers and session-local links remain only in
these raw files and the archive.

Marker inventory, extracted mechanically from the raw files (a private-use
prefix, `cite`, a separator, then `turn…search…`/`turn…view…` tokens, then a
terminator):

| Input | Marker groups | Token occurrences | Distinct tokens | Distinct `turn` prefixes |
| --- | ---: | ---: | ---: | ---: |
| Frost1 | 50 | 135 | 58 | 11 |
| Frost2 | 47 (on 36 lines) | 72 | 36 | 9 |
| Frost3 | 15 | 38 | 17 | 6 |

Group counts equal the archive manifest's `opaque_citation_count` values. The
original source mapping behind the tokens is unavailable; the
[citation map](citation-map.json) records every group as unresolved and lists
the editorial sources that independently support the claim at that location,
labelled as support, never as recovery. After the round-3 corrections, of the
112 groups 14 are fully supported by registered editorial sources, 41
partially, and 57 have no registered source; the curated reports label those
57 claims unverified or leave them in the raw files. (The first pass recorded
16/39/57; round 3 upgraded Frost3 group 6 on Opus's page-7 confirmation and,
under lead finding CORPUS-COVERAGE-9, downgraded Frost2 groups 2, 37, and 41
to partially supported because their unconditional total-input-linear and
memory-mapped-backing assertions exceed the registered evidence.) Tokens are scoped per input:
`turn338730view0` in Frost2 is not assumed to be the same source as any
similarly numbered token elsewhere.

## 2. Missing artifacts and recovery attempts

| Referenced artifact | Referenced by | Where searched | Result |
| --- | --- | --- | --- |
| `frost-collection-algebra-research.md` ("repository-ready research document") | Frost2 lines 49-51 (session-local link) | Repository tree at baseline; `docs/`, `verification/`; the handoff archive (12 members listed); the thread's attachments as reported by the lead | Not found |
| `frost-research-template.md` (`docs/research/RESEARCH_TEMPLATE.md`) | Frost2 lines 53 and 568 | Same | Not found; the handoff's `templates/research-prompt.md` is a new starter, not this file |
| `docs/research/computed-sets.md` (full computed-set report) | Frost3 line 17 | Same | Not found |
| `docs/research/templates/research-prompt.md` (original) | Frost3 line 23 | Same | Not found; the path is now occupied by the adapted handoff template, which does not derive from it |
| `frost-computed-sets-research.zip` | Frost3 line 29 | Same | Not found |
| Standalone algebraic-foundations-and-execution report | Task request | Same | Never identified; the [synthesis](synthesis/foundations-and-engine-roadmap.md) summarizes verified basics and `Q-ALG-1` specifies the deep dive |

No text of any missing artifact was inferred from the delivery note. The
summary's statement that the files "were created" describes another session
and is not inherited as evidence (hazard H11).

## 3. Hazard decision matrix

Each mandatory hazard was located in the raw inputs (line anchors confirmed
independently by a bounded read-only child audit; see section 8) and given a
disposition in the curated documents.

| ID | Hazard | Input location | Repository evidence | Primary evidence | Disposition | Where applied |
| --- | --- | --- | --- | --- | --- | --- |
| H1 | `#[derive(Hash, Eq, PartialEq)]` on `Key` while the comment says equality depends only on `logical_id` | Frost1 78-84 | `docs/design/architecture.md` section 2 (user trait laws) | `SRC-RUST-HASHMAP-GET-KEY-VALUE` example with manual impls | Corrected example with lawful manual `PartialEq`/`Eq`/`Hash`; compiled and run on Rust 1.99.1 (section 7); raw example preserved | Survey section 3 |
| H2 | `try_insert(&mut self, …)` on `FrozenBiMap` | Frost1 367-376 | `src/frozen.rs` (no structural mutation), `docs/design/philosophy.md` | — | Rejected as written; insertion belongs on `MutableBiMap` or a builder; frozen algebra returns a new value | Survey section 5; `Q-FROZEN-1`, `Q-BIMAP-1` |
| H3 | Plural crate names `frost-bags`, `frost-multimaps`, `frost-bimaps` | Frost1 26-28 | `Cargo.toml`: one package | — | Preserved as an alternative; one naming policy proposed (singular domain names) for a future decision; no crate created | Synthesis section 4 |
| H4 | Singular crate names `frost-bag`, `frost-multimap`, `frost-bimap` | Frost2 28-30 | Same | — | Same as H3; third set in Frost3 line 62 also recorded | Synthesis section 4 |
| H5 | FST n-ary operations "total linear stream complexity" / "linear total input complexity" | Frost1 7, 686 | `docs/reviews/lazy-views-source-audit.md` F2 | `SRC-FST-0_4_7-OPBUILDER-DOCS`, `SRC-FST-0_4_7-RAW-OPS` | Qualified: documented bound counts elements; implementation is a binary heap (section 6) | Survey section 6; synthesis CL-NARY-1 |
| H6 | Same claim as `O(n₁+n₂+…)` with memory proportional to stream count | Frost2 9, 369; Frost3 54 | Same | Same | Same qualification | Implementation notes section 5; computed-sets summary C7 |
| H7 | Proposed `[features]` TOML (`set-algebra`, `map-algebra`, `bags`, `roaring`, …) | Frost1 572-605 | `Cargo.toml` features (`roaring-0_12`, `int-set`, no `set-algebra`) | — | Historical proposal; not copied; nonexistent names noted | Survey section 7 |
| H8 | `cargo … -p frost-map-algebra`, `cargo +nightly miri`, `cargo kani -p frost-bimap` commands | Frost2 469-492 | `Makefile` targets; `docs/verification.md` | — | Historical, not runnable; current Make targets substituted | Implementation notes section 6 |
| H9 | CI skeleton with `dtolnay/rust-toolchain@stable`/`@nightly` over nonexistent packages | Frost2 496-539 | `.github/workflows/ci.yml` pins 1.99.1 | — | Historical; not copied | Implementation notes section 6 |
| H10 | "No explicit target restrictions were given" | Frost2 57 | `Cargo.toml` rust-version, `rust-toolchain.toml`, Makefile embedded targets | — | Corrected: Rust 1.99.1, edition 2024, `no_std` core, `alloc`/`std`, embedded builds with and without pointer atomics | Implementation notes section 3 |
| H11 | "The two files were created with the exact repository-relative paths requested and packaged in the ZIP" | Frost3 113 | Files absent at baseline and in the archive | — | Not inherited; recorded as a claim about an unavailable artifact | Computed-sets summary section 6; section 2 above |
| H12 | Crate placement `frost-api`, `frost-set-algebra`, `frost-set-kernels`, `frost-integer-sets`, `frost-core`, `frost-concurrent` | Frost3 62 | One package | — | Proposal; dependency direction honoured by module boundaries | Computed-sets summary section 4 |
| H13 | `MapGet<Q>` capability name and unconstrained `Q` | Frost1 183-190, 450-460 | `src/read.rs`: `LookupRef<Q>` with per-implementation query relation | — | Mapped to the existing capability; not renamed; `Q` relation stays per implementation | Survey section 5 |
| H14 | `BiMap<L, R>` trait with `try_insert(&mut self)` and `get_left<Q: ?Sized>` without a relation bound | Frost2 326-335 | `src/frozen.rs`, `src/read.rs` | — | Pseudocode; insertion belongs on `MutableBiMap` or a builder, with query relations on implementations | Implementation notes section 4 |
| H15 | `model_inner_join` returning `Vec<(&K, &A, &B)>` without lifetimes | Frost1 888-897 | — | Rust lifetime elision rules (two input references, ambiguous output) | Labelled pseudocode; not compiled | Survey section 8 |
| H16 | Invented capability names: `SortedMapEntries`, `SetGet` as named in Frost1 exists but `MapGet`, `BagCount`, `MapEntries`, and Frost2's `Get`/`Contains`/`Entries` do not all exist | Frost1 197-216, 437-466; Frost2 205-230 | `src/read.rs` | — | Each mapped to the actual name where one exists (`SetContains`, `SetGet`, `LookupRef`, `Entries`, `OrderedEntries`, `Len`); others labelled proposals | Survey section 5; implementation notes section 4 |
| H17 | Frost3's "future optional sorted/seek capabilities" presented alongside real names | Frost3 84 | `src/read.rs` | — | Real names identified; the future ones labelled as such | Computed-sets summary section 3 |
| H18 | Roaring version 0.11.5 in the ecosystem table | Frost2 171 | `Cargo.lock`: 0.12.0 | `SRC-ROARING-0_12_0-SRC` | Drift recorded; kernels verified in the locked source | Implementation notes section 3 |
| H19 | Session-local `sandbox` links to unsupplied files | Frost2 51, 53, 568; Frost3 17, 23, 29 | Archive `input-manifest.json` lists them | — | Not reproduced in curated files; recorded as recovery targets in section 2 | Raw README; this audit |
| H20 | Frost1's `live.snapshot()` sketch implies a live map yields a coherent snapshot | Frost1 746-752 | `src/concurrent`: live backends are not `Eligible` or `SnapshotValue`; `SnapshotStore::load` is the snapshot | — | Mapped to loading a published version; live-to-snapshot cloning needs its own documented mechanism | Survey section 6 |

## 4. Theory and security boundary checks

| Requirement | Finding in inputs | Disposition in curated corpus |
| --- | --- | --- |
| Distinguish set, map, bag, sequence, multimap, bimap laws; representative identity; order; multiplicity; overflow and failure; callback effects | Inputs distinguish the first five well; callback effects (resolver call count, order, panic) are not specified | Taxonomy table with all eight columns in the synthesis; open items assigned to `Q-ALG-1`, `Q-MAP-1` |
| `Fn` does not prove purity | Not stated by inputs | Stated in the master prompt, synthesis, and checklist |
| Typed domains and dictionary revisions are not interchangeable ids | Frost3 proposes interning dictionaries without a revision rule | Stated in the synthesis and the authorization prompt; the general set-algebra design already requires compatible dictionary versions |
| Frozen, persistent, query, snapshot, materialized are different axes | Inputs mostly keep them apart; Frost1's `live.snapshot()` and Frost3's "pinned snapshot" blur the live/snapshot boundary | Five-axis table in the synthesis; H20 |
| Laziness does not imply bounded work or memory | Frost3 implies no container is needed for membership; silent on scans, buffering, recursion | Qualified in the computed-sets summary and synthesis CL-CSET-2 |
| Authorization examples need full request-tuple correlation, explicit unknown/error outcomes, consistency assumptions | Not present in inputs (the README draft, owned by the other lane, contains the deny-overrides illustration) | Counterexample and outcome rule in the synthesis section 6 and the authorization prompt |
| Influences are not dependencies or supported protocols | Not present in inputs | Synthesis section 6; gap table in SOURCES.md |
| Solvers, path conditions, and sound merging stay application responsibilities | Not present in inputs | Synthesis CL-SYM-1 |

## 5. Corrections to repository-facing statements

- The curated reports replace every opaque marker with catalog ids or an
  explicit "unverified" label. No curated file contains a marker, a
  session-local link, or a chat link (checked mechanically; section 7).
- The master research prompt's "fixed project context" from the handoff
  asserted a `crates/frost-*` target layout as fact. The adapted template
  instructs the researcher to record the actual package graph and treat any
  split as a proposal, consistent with the current single-package policy.
- The handoff's suggestion of a compatibility link from an older
  `RESEARCH_TEMPLATE.md` is not applied: that file never existed in this
  repository and has no documented consumers.
- Frost2's ecosystem versions are reproduced as dated claims; where the
  repository pins a different version (roaring), the lockfile is stated.

## 6. N-ary bound derivation (from the inspected `fst` 0.4.7 source)

This is **derived reasoning from source inspection**, not a measurement and
not upstream wording. Domain: `k ≥ 0` input streams; stream `i` has `n_i ≥ 0`
keys; `N = Σ n_i`; keys are byte strings of length at most `L ≥ 0` (empty
keys allowed); each underlying stream's `next` costs `t_adv` (transducer
traversal and any I/O, including memory-mapped page faults), which is
**not** counted in the merge bound.

Observed structure (`SRC-FST-0_4_7-RAW-OPS`): `StreamHeap::new` primes every
stream once (`k` advances and up to `k` pushes, performed even when every
stream is empty). Each emitted or skipped key pops the minimum slot and
refills it (one advance, at most one push) and pops every other slot whose
key equals the minimum (`pop_if_equal`), refilling each. Every key of every
stream is therefore pushed and popped at most once beyond priming: at most
`N + k` pushes and `N + k` pops. A binary heap holding at most `k` slots
costs `O(log(k + 1))` comparisons per push or pop; the `+ 1` keeps the factor
positive for `k = 1`, and for `k = 0` there is nothing to merge beyond the
`O(k)` initialization. Each comparison compares two byte vectors (then the
output value) and each refill copies one key into its slot: both cost
`O(L + 1)`, where the `+ 1` covers empty keys, which still cost constant
work.

Resulting bounds:

| Quantity | Bound |
| --- | --- |
| Initialization | `O(k)` work and `k` stream advances, regardless of key counts |
| Heap operations | at most `N + k` pushes and `N + k` pops |
| Merge work (comparisons and key copies) | `O(k + N · (L + 1) · log(k + 1))` |
| Underlying advances | `N + k` calls, each `t_adv`, counted separately |
| Merge workspace | `O(k · (L + 1))` bytes: `k` boxed stream handles, at most `k` slots each with its own `Vec<u8>` (initial capacity 64, growing to the longest key seen), and an output `Vec<IndexedValue>` of at most `k` entries; `Difference` adds one key buffer. Each input stream's internal state is excluded |
| Output | Up to `N` keys for union; each emitted item borrows the slot buffer until the next advance (lending) |

Treating `k` and `L` as bounded constants reduces the merge work to `O(N)`;
that fixed-fan-in, fixed-key-cost reading is this corpus's interpretation of
the upstream statement (`O(n1 + n2 + …)` with memory proportional to the
stream count). The upstream documentation does not itself say that its bound
omits fan-in, key length, or decoding, and nothing here was timed. This
derivation applies to `fst` 0.4.7 only. It does not transfer to a frontier
merge (`O(k)` per emitted key without a heap), to Frost's ordered pairwise
kernels, or to unordered probe plans, each of which needs its own model
(`Q-NARY-1`). The design document `docs/design/collection-algebra.md` carries
the same bound in its n-ary section; the two statements were aligned in
round 3.

## 7. Checks executed on this checkout

Commands were run from the repository root on 2026-11-09 unless a row says
otherwise. "rtk" prefixes are the host's output-condensing wrapper; they do
not change the commands.

| Check | Command | Result |
| --- | --- | --- |
| Input and archive hashes before copying | `shasum -a 256` on the attachment ZIP, the extracted `inputs/Frost3.md.md`, and the two tracked Frost1/Frost2 files | All four equal the archive manifest and the lead's recorded values |
| Copies unchanged | `cp` of Frost3 and the ZIP into `docs/reviews/supplied/`, then `shasum -a 256` and `wc -c` | `6f39fdbc…` (11,501 bytes) and `605767df…` (62,661 bytes) |
| Archive member list | `unzip -l` on the ZIP; `find` on the extracted directory | 12 members, 158,773 bytes uncompressed; extracted tree matches |
| Marker extraction | Node 26.13.0 script in the lane scratch directory over the three raw files (regex on the private-use prefix, `cite`, separator, and terminator) | 50 / 47 / 15 groups; distinct tokens 58 / 36 / 17 (section 1) |
| Corrected Eq/Hash example | `rustc +1.99.1 --edition 2024 -D warnings -o representative_key_1991 representative_key.rs && ./representative_key_1991` in the scratch directory | `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; compiled with no warnings; printed `representative example passed`. A first run without `+1.99.1` resolved to the host default 1.96.0 and also passed; the 1.99.1 run is the evidence used |
| `fst` 0.4.7 source inspection | Read `~/.cargo/registry/src/…/fst-0.4.7/src/raw/ops.rs` and `src/stream.rs`; `Cargo.lock` pins 0.4.7 | Section 6 |
| `roaring` 0.12.0 source inspection | `rg` for `intersection_len|union_len|difference_len|symmetric_difference_len|rank|select` in `roaring-0.12.0/src/{bitmap,treemap}` | All six present on both `RoaringBitmap` and `RoaringTreemap` |
| External source rechecks | Fetched the std `HashMap` and `HashSet` pages, the docs.rs `fst` `OpBuilder` page, the Zanzibar publication page (not the PDF), the SpiceDB schema page, RFC 9635, the W3C DID page, and the libp2p peer-ids spec; in round 3, the version-pinned docs.rs source pages for `fst` 0.4.7 `stream.rs` and `roaring` 0.12.0 `bitmap/ops.rs` and `treemap/ops.rs` | Observations recorded in SOURCES.md as concise paraphrases. The Zanzibar section numbers in the lead's preparatory note were wrong (rewrites are in 2.3.1, not 2.2; the snapshot rule is in 2.2 and 2.4.4, not 2.4.3); Opus read the PDF (pp. 2-8) and reported the correct sections and pages, which the catalog adopted in round 3 with that attribution. Preferred MCP research lanes (Tavily, Context7, DuckDuckGo, arXiv) failed to connect this session; the native fetch tool was the fallback |
| Structural verification of the corpus | `node scratch/ntm-swarm/research-handoff/verify-docs.cjs`; `check-jsonschema` 0.39.0 on the two JSON files; Ruby YAML parse of every front-matter block; `make doc-links`; `git diff --check` | Recorded in section 9 after the run |

Not executed by this lane: `make check`, the feature matrix, embedded builds,
Miri, and benchmarks. No production source, test, manifest, lockfile, CI, or
tool configuration was changed, so those gates are unaffected by this change;
the lead runs `make check` on the combined tree.

Rendering was not performed: no Markdown or Mermaid renderer is available on
this host. Fence balance and link resolution are structural checks only.

## 8. Child audit (bounded, read-only)

One native child (Claude Haiku, read-only, no delegation, no mail) was
dispatched to confirm the fifteen hazard line anchors H1-H15 against the raw
files and to count markers independently. Result, reconciled by the parent:

- All fifteen anchors **confirmed** with verbatim quotes; the child noted
  that H15 was not compiled and that its failure expectation follows from
  lifetime elision rules.
- Marker counts: Frost1 50 groups on 50 lines, 58 distinct tokens; Frost2 47
  groups on 36 lines, 36 distinct tokens; Frost3 15 groups on 15 lines, 17
  distinct tokens. These equal the parent's extraction. The child counted
  `cite` occurrences and private-use prefixes with a different regex, which
  is why the agreement is meaningful.
- Limits reported by the child: it read only the anchored ranges; nothing was
  compiled; no files were edited.

The child's report was collected and the child completed before this audit
was finalized. No other children were started.

## 9. Structural check results

Executed on 2026-11-09 after all corpus files were written. The verifier and
link checker ran over the combined working tree, which at that moment also
contained the other lane's in-progress README, index, and Frozen API guide
changes; their results therefore cover both lanes' files as they stood.

| Check | Command | Result |
| --- | --- | --- |
| Citation map generation (first pass, historical) | `node build-citation-map.cjs markers.json docs/research/citation-map.json` (scratch script over the mechanical extraction) | 112 groups written: 16 supported, 39 partially supported, 57 unsupported (Frost1 4/17/29; Frost2 5/16/26; Frost3 7/6/2). Superseded by the round-3 regeneration recorded in section 13 |
| JSON Schema validation | `check-jsonschema --schemafile docs/research/manifest.schema.json docs/research/manifest.json`; same for `citation-map.schema.json` / `citation-map.json` (check-jsonschema 0.39.0) | Both: `ok -- validation done` |
| Lead's structural verifier | `node scratch/ntm-swarm/research-handoff/verify-docs.cjs` | 31 changed files; 4 raw hashes match; 11 original README executable fences preserved; 25 Markdown and 4 JSON files checked; 418 local links and anchors resolved; `errors: []` |
| Repository link check | `bash scripts/check-doc-links.sh` (the `make doc-links` recipe) | `Verified 774 repository-local documentation links.` |
| Front matter | Ruby `-ryaml` over `docs/research/**/*.md`, loading each leading block as a mapping with an `id` | 13 blocks parsed. The host's `PATH` Ruby was 2.6.10 (the assignment also listed Ruby 4.0.9 with Psych 5.3.1 as available; it was not needed) |
| Forbidden tokens in curated files | `rg --pcre2` over `docs/research` for the private-use code points U+E200 to U+E202, the session-local `sandbox` URL scheme, and chat-session links (the same pattern the lead's verifier applies) | No match |
| Trailing whitespace in curated files | `rg ' +$' docs/research` | No match |

Not performed: Markdown or Mermaid rendering (no renderer on the host);
`make check` and the Cargo gates (no source, manifest, lockfile, CI, or tool
change in this lane; the lead runs them on the combined tree).

## 10. Evidence-gap inventory

| Gap | Effect | Owner / next step |
| --- | --- | --- |
| Original citation mappings for all 112 marker groups | Curated claims rest on editorial sources or are labelled unverified | Recovery would require the original research metadata; none available |
| Full computed-set report, original research template, computed-set ZIP | `Q-CSET-1` starts from a summary | Child research, not recovery |
| Standalone algebraic-foundations deep dive | The synthesis summarizes verified basics only | `Q-ALG-1` |
| Bag-semantics, relational, ART, succinct-dictionary, FST-construction literature | Attributions unverified | Register sources when a child reads them |
| Cross-language library claims (Guava, Java, C++, Haskell, OCaml, Clojure, Scala, PostgreSQL) | Unverified; retained in raw files only | Low priority; not needed for Frost decisions |
| OpenFGA, Differential Dataflow, Macaroons, Vanadium, OAuth, UCAN, ZCAP, Kameo | Influence only | `Q-AUTHZ-1` registers what it reads |
| Zanzibar pages | Sections and pages are as reported by Opus's PDF read; this lane did not read the PDF | A child report confirms them in its own read |
| Roaring upstream documentation and serialization-format claims | Kernels verified from source; format claims unverified | Retry upstream docs in a later session; not retried here |
| `docs/design/frozen-api.md` | Linked from this corpus; written by the other lane in this round | Lead verifies the combined tree |
| Rendering | Not performed | Lead or a later host with a renderer |

## 11. Verification updates

Appended when the lead's combined verification receipts arrive.

The lead's first combined `make check` attempt (`make-check.log`, 2026-11-09)
passed formatting and the 774-link check, then failed Clippy with three
`clippy::doc_markdown` errors in README prose (unbackticked `BiMap` and
`PeerId` tokens at README lines 49, 158, and 161); no later gate ran in that
attempt. Those lines belong to the other lane and were corrected there.

### Combined verification receipt (lead, recorded 2026-11-09T18:43:08Z)

Source: `scratch/ntm-swarm/research-handoff/r3/lead-verification.json`,
status `combined_checks_passed_editorial_correction_required`. Receipts as
recorded by the lead on the combined tree at base `82becb9`:

| Command | Exit | Result as recorded |
| --- | --- | --- |
| `rtk make check` | 0 | Covers `cargo fmt --all --check`, 774 repository-local documentation links, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo doc --all-features --no-deps --locked` with `RUSTDOCFLAGS=-D warnings`, `cargo test --all-features --locked` including doctests, and the `test-examples` target. Supersedes the earlier failed attempt. Log `r3/make-check.log` is RTK-filtered; no aggregate test count is asserted |
| `rtk proxy node scratch/ntm-swarm/research-handoff/verify-docs.cjs` | 0 | 31 changed files; 4 raw hashes; 11 original README executable fences preserved; 25 Markdown, 4 JSON; 418 local links and anchors; `errors: []` |
| `rtk proxy ruby scratch/ntm-swarm/research-handoff/verify-corpus.rb` | 0 | 21 artifacts; 13 front matter; 10 questions; 54 registered sources; groups 50/47/15; token occurrences 135/72/38; `errors: []` |
| `rtk proxy ruby scratch/ntm-swarm/research-handoff/verify-frontmatter.rb` | 0 | 13 front-matter mappings parsed safely; no duplicate keys, aliases, or errors |
| `check-jsonschema` 0.39.0 on `manifest.json` and `citation-map.json` | 0 | both `ok -- validation done` |
| `rtk git diff --check` | 0 | Tracked diff; the staged final diff check will also cover new files |
| `rtk proxy unzip -Z1 docs/reviews/supplied/frost-research-handoff.zip` | 0 | All 12 names match `manifest.archive.members`; template `input_members` are members of the retained archive |

Limitations recorded by the lead: research Markdown and Mermaid rendering
unavailable (README rustdoc was built); original citation mappings and the
missing reports remain unavailable; no source, dependency, feature, CI, or
build-tool change, so the feature matrix, embedded builds, Miri, and the
release `verify` target were not required or run for this documentation-only
integration. Commit and push remain with the lead; none had occurred at the
time of the receipt.

The receipt required one editorial correction, CORPUS-COVERAGE-9, applied in
this lane and recorded in sections 12 and 13. The reruns after that
correction are in section 13.

## 12. Round 3 corrections

Applied on 2026-11-09 after the lead's review notes and the other lane's
round-2 cross-review. Each item names the finding, the disposition, and the
files touched (all within `docs/research/`).

| Finding | Disposition | Files |
| --- | --- | --- |
| CORPUS-ATTRIBUTION-5 and XR-3 (unsupported project-owner attribution) | Removed. The workspace split and the protocol influences are described as content of the supplied research and its handoff, unadopted, with current policy keeping one package | `templates/research-prompt.md`; `synthesis/foundations-and-engine-roadmap.md` sections 6, 7, 9 |
| CORPUS-PROVENANCE-6 (archive member paired with ZIP digest) | Every `input_files`/`input_sha256` pair now names repository bytes: templates pair the retained archive path with the ZIP digest and record the member path in a new `input_members` field; reports, synthesis, citation map, audit, and raw index name the tracked raw files. Schema, manifest, and front matter synchronized | five templates; three reports; synthesis; `manifest.json`; `manifest.schema.json` |
| CORPUS-PARENT-7 (new parent pinned to the code baseline) | All four prompts and the handoff template set `parent_revision` to an explicit unresolved marker, add `parent_content_sha256`, keep `repo_baseline` for inspected code only, and instruct the child to resolve the parent's real commit or content digest before execution | four prompts; `templates/subtopic-handoff.md` |
| XR-1, XR-2 (Zanzibar sections) | Corrected to 2.2 and 2.4.4 (snapshot lower bound), 2.3.1 (userset rewrites), 3.2.3 (check evaluation and cancellation), 3.2.4 (Leopard), with pages, attributed to Opus's PDF read; this lane did not reread the PDF. Citation-map group 6 upgraded to supported on the skip-list detail Opus confirmed | `SOURCES.md`; `reports/computed-sets-summary.md`; two prompts; synthesis CL-CSET-1; `citation-map.json` (regenerated) |
| ALGEBRA-BOUND-8 (matching derived bounds in the corpus) | Replaced every `O(N log k)` / `O(k)` statement with the conservative `O(k + N · (L + 1) · log(k + 1))` merge bound plus decoding and I/O and `O(k · (L + 1))` byte workspace excluding stream internals; empty, single-stream, and empty-key cases covered; "counts elements only" reworded as this corpus's interpretation; no performance claim | section 6 above; survey section 6 and S1-7; implementation notes section 5; computed-sets summary C7 and S3-4; synthesis CL-NARY-1 and conflict table; computed-sets prompt |
| XR-4 (launch fields for seeded follow-ups) | Added compact blocks with parent section, decision, inherited assumptions, alternatives, primary evidence, experiments, acceptance, owning module, and downstream artifacts for `Q-NARY-1`, `Q-MAP-1`, `Q-BAG-1`, `Q-MM-1`, `Q-BIMAP-1`, `Q-PREFIX-1` | synthesis section 8.1 |
| XR-5 (output-mode table claimed in synthesis) | Table added to synthesis section 2 | synthesis |
| XR-6 (portable, version-pinned source URLs) | Catalog now cites docs.rs source pages pinned to `fst` 0.4.7 and `roaring` 0.12.0 and the pinned `OpBuilder` and `Streamer` pages; the local registry copy is mentioned as corroboration only | `SOURCES.md` |
| XR-7 (byte workspace) | Covered by the ALGEBRA-BOUND-8 rewording | synthesis; computed-sets summary |
| Lead verifier `related_ids` drift (landscape, deep-dive, subtopic templates; algebraic-foundations prompt) | Manifest entries aligned with front matter | `manifest.json` |
| Quotation trimming | Verbatim external quotations longer than a short phrase replaced by paraphrases; raw inputs untouched | `SOURCES.md` |
| CORPUS-COVERAGE-9 (lead, from the combined verification receipt) | Frost2 groups 2 (raw line 9), 37 (line 369), and 41 (line 417) changed from supported to partially supported. Groups 2 and 37 assert an unconditional total-input-linear runtime; the registered evidence supports only the documented upstream statement plus a qualified heap-based derivation, and each entry now names that gap. Group 41 also asserts memory-mapped backing; the registered scopes establish only streams, byte keys, and `u64` values, and the entry says so. `claim_context` values are unchanged. Final counts 14 supported, 41 partial, 57 unsupported (the lead's expected 13/42/57 assumed the first-pass 16/39/57 and did not include the Frost3 group-6 upgrade made earlier in round 3 on Opus's page-7 confirmation, whose scope names that evidence) | `citation-map.json` (regenerated); section 1 above |

Cross-review of the other lane's files is recorded in the round-3 lane
report, not here.

## 13. Checks after the round-3 corrections

Executed on 2026-11-09 at 18:46 UTC after the CORPUS-COVERAGE-9 regeneration
and the section 11 receipt addition, on the combined working tree.

| Check | Command | Result |
| --- | --- | --- |
| Citation map regeneration | `node build-citation-map.cjs markers.json docs/research/citation-map.json` | 112 groups: 14 supported, 41 partially supported, 57 unsupported (Frost1 4/17/29; Frost2 2/19/26; Frost3 8/5/2). Frost2 groups 2, 37, 41 confirmed `unresolved-partially-supported` with unchanged `claim_context` |
| JSON Schema | `check-jsonschema` 0.39.0 on `citation-map.json` and `manifest.json` | both `ok -- validation done` |
| Lead's corpus verifier | `ruby scratch/ntm-swarm/research-handoff/verify-corpus.rb` | exit 0; 21 artifacts; 13 front matter; 10 questions; 54 sources; groups 50/47/15; `errors: []` |
| Lead's front-matter verifier | `ruby scratch/ntm-swarm/research-handoff/verify-frontmatter.rb` | `errors: []` |
| Lead's structural verifier | `node scratch/ntm-swarm/research-handoff/verify-docs.cjs` | exit 0; 31 files; 4 raw hashes; 11 original README fences; 25 Markdown, 4 JSON; 418 links and anchors; `errors: []` |
| Repository link check | `bash scripts/check-doc-links.sh` | `Verified 774 repository-local documentation links.` |
| Forbidden tokens and trailing whitespace | `rg` scans over `docs/research` | no match |

Cargo gates were not repeated for this JSON-and-audit-only correction, as the
lead's receipt specified. Rendering remains unverified.

## 14. Round 4 closeout: final corrections and review record

Recorded on 2026-11-09 by the other documentation lane (README and design
guides). For this closeout the lead handed three corpus files to that lane:
`manifest.json`, `synthesis/foundations-and-engine-roadmap.md`, and this
audit. No other corpus or raw file was edited, and none of the round-3
corpus work was reverted.

### Corrections

| Finding | Disposition | Files |
| --- | --- | --- |
| R3-2 (seeded follow-ups had no addressable brief) | Each of the six seed blocks in synthesis section 8.1 now has its own heading. The section 8 table links each "Child brief" cell to its block (and, for `Q-NARY-1`, also to the prompt section). `manifest.json` `questions[].brief` now points `Q-MAP-1`, `Q-BAG-1`, `Q-MM-1`, `Q-BIMAP-1`, and `Q-PREFIX-1` at those headings, and `Q-NARY-1` at the "Embedded question Q-NARY-1" section of the computed-sets prompt. No research content was added. | synthesis; `manifest.json` |
| PR-1 (docs index survey attribution) | The index now says the survey derives from Frost1 and the implementation notes from Frost2 | `docs/README.md` |
| PR-2 (borrowed access row) | The Frozen API guide now separates traversal (`Elements`, `Entries`), query representatives (`LookupRef`, `SetGet`), and `ValueRef`, which lends a value only | `docs/design/frozen-api.md` |
| PR-3 (adaptive exits) | The guide now says both adaptive frozen profiles have `thaw()`, and only `AdaptiveFrozenHashMap` also has `thaw_with(policy)` | `docs/design/frozen-api.md` |

Brief location for every question after this closeout:

| Question | Brief |
| --- | --- |
| `Q-ALG-1` | [prompt](prompts/algebraic-foundations.md) |
| `Q-FROZEN-1` | [prompt](prompts/frozen-api-contracts.md) |
| `Q-NARY-1` | [computed-sets prompt section](prompts/computed-sets-and-durable-execution.md#embedded-question-q-nary-1); also a seed block in the synthesis |
| `Q-CSET-1` | [prompt](prompts/computed-sets-and-durable-execution.md) |
| `Q-AUTHZ-1` | [prompt](prompts/authorization-engine-integration.md) |
| `Q-MAP-1` | [seed block](synthesis/foundations-and-engine-roadmap.md#q-map-1-map-merge-and-join-planning) |
| `Q-BAG-1` | [seed block](synthesis/foundations-and-engine-roadmap.md#q-bag-1-bag-arithmetic-overflow-contract-and-storage) |
| `Q-MM-1` | [seed block](synthesis/foundations-and-engine-roadmap.md#q-mm-1-multimap-fiber-semantics-and-naming) |
| `Q-BIMAP-1` | [seed block](synthesis/foundations-and-engine-roadmap.md#q-bimap-1-two-index-integrity-checked-merge-and-composition) |
| `Q-PREFIX-1` | [seed block](synthesis/foundations-and-engine-roadmap.md#q-prefix-1-prefix-and-rank-indexes) |

### Final review of the round-3 corpus corrections

The review covered only bytes that changed after the reviewing lane's
round-3 snapshot: the citation map and this audit (the manifest and
synthesis changes after that snapshot are the R3-2 edits above).

- **CORPUS-COVERAGE-9: fixed.** Frost2 groups 2 (raw line 9), 37 (line 369),
  and 41 (line 417) are `unresolved-partially-supported`, and each scope names
  the part of the raw claim that no registered source establishes. Counting
  the statuses directly from `citation-map.json` gives **14 supported, 41
  partially supported, 57 unsupported** (Frost1 4/17/29, Frost2 2/19/26,
  Frost3 8/5/2). These are the current totals, and sections 1, 12, and 13
  state the same numbers. The first-pass 16/39/57 (section 9) and the lead's
  provisional 13/42/57 expectation (section 12) are historical. The
  difference from 13/42/57 is Frost3 group 6, upgraded in round 3 on the
  page-7 skip-list evidence and kept.
- **Marker coverage: unchanged and source-scoped.** All 112 groups (50/47/15)
  keep their raw line and token sequence, every supporting source id
  resolves in `SOURCES.md`, and no group claims a recovered original
  mapping.
- **Other round-3 corrections:** the reviewing lane had already confirmed
  them as fixed at its round-3 snapshot (attribution, provenance, parent
  revision, `related_ids`, Zanzibar sections and pages, seeded-question
  fields, output-mode table, pinned source URLs, byte workspace). Those
  files are unchanged since then.
- **R4-1: fixed.** Questions `Q-MAP-1`, `Q-BAG-1`, `Q-MM-1`, `Q-BIMAP-1`,
  and `Q-PREFIX-1` have launch blocks under their own headings in
  [synthesis section 8.1](synthesis/foundations-and-engine-roadmap.md#81-seeded-follow-up-blocks),
  which `manifest.json` references as their briefs; a dedicated prompt can be
  written with the subtopic handoff template once a question's dependencies
  are answered.

### Where the evidence lives

Two kinds of check appear in this audit:

- **Repository-backed checks** ship with a clone: `make check`,
  `make doc-links` (`scripts/check-doc-links.sh`), and the JSON Schemas
  [`manifest.schema.json`](manifest.schema.json) and
  [`citation-map.schema.json`](citation-map.schema.json). Any JSON Schema
  validator can apply the schemas; this integration used `check-jsonschema`
  0.39.0.
- **Session helpers and receipts** are local and untracked: `verify-docs.cjs`,
  `verify-corpus.rb`, `verify-frontmatter.rb`, `build-citation-map.cjs`, the
  lanes' own checker scripts, `lead-verification.json`, and the `make check`
  logs. They lived under a locally ignored `scratch/` directory during the
  session and are **not** part of the repository. This audit keeps their
  summarized outcomes, not the files. Re-verify with the repository-backed
  checks and the raw hashes in section 1.

### Checks on the final combined tree

Run on 2026-11-09 after the round-4 edits and after this section was added.

| Check | Result |
| --- | --- |
| `make doc-links` | `Verified 789 repository-local documentation links.` (774 before round 4; the additions are the synthesis prompt-section link and the links in this section) |
| `check-jsonschema` 0.39.0, `manifest.json` and `citation-map.json` against their schemas | both `ok -- validation done` |
| Session corpus verifier (`verify-corpus.rb`) | exit 0; 21 artifacts; 13 front-matter blocks; 10 questions; 54 sources; groups 50/47/15; tokens 135/72/38; brief anchors resolved; `errors: []` |
| Session front-matter verifier (`verify-frontmatter.rb`) | 13 files; `errors: []` |
| Session structural verifier (`verify-docs.cjs`) | exit 0; 31 changed files; 4 raw hashes; 11 original README executable fences preserved; 25 Markdown and 4 JSON files; 439 local links and anchors resolved; `errors: []` |
| Forbidden tokens and trailing whitespace (`rtk rg` over `docs/research`, `docs/README.md`, `docs/design/frozen-api.md`) | no match |
| `git diff --check` | exit 0 |

Cargo gates were not rerun: `README.md`, the only Markdown file compiled
into rustdoc (`src/lib.rs` includes it), is byte-identical to the version
the lead's combined `make check` passed (section 11). Rendering, the release
`verify` target, the feature matrix, embedded builds, Miri, Loom, Kani, and
benchmarks were not run in this closeout.
