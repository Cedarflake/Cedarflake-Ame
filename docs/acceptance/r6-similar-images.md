# R6 similarity integration evidence

Status: scoped evidence; [roadmap](../roadmap.md) owns delivery disposition.

The [engine evaluation](r6-engine-evaluation.md) records selection and resource probes.
[ADR 0038](../architecture/0038-bounded-perceptual-similarity.md) owns the admitted algorithm and
group identities. This record covers their production integration.

## Persistent identities and choices — 2026-11-04

At `b178fd87`, schema 74 separates immutable analysis runs, content evidence, bounded candidate indexes and
resumable matching from durable group references, representative choices, ignores and exclusions.
The migration runs atomically after schema 73 and preserves existing catalog and exact-group choices.

Two focused migration cases passed against the final DDL and row validation. They cover retained
exact-group choices, malformed reference-projection rejection, preservation of similarity choices
after derived evidence removal, complete rollback on a rejected version update, and successful retry.
The command used the repository tool lock and `Invoke-AmeAcceptanceScratch`, with `TEMP` and `TMP`
bound to that disposable fixture directory:

```powershell
cargo test --locked --offline --manifest-path rust/Cargo.toml --lib adapters::sqlite_catalog::migrations::similar_images -j 1 -- --test-threads=1
```

The new migration facade/schema/validation have 51/130/162 source lines, including conditional
fixture helpers, no inline test cases and 153 dedicated test lines. The domain identity owner has
58 source lines. Existing migration coordination changes only dispatch: 4,181 source lines including
fixture helpers and 2,290 inline-test lines; current-schema routing has 416 source and 36 dedicated
test lines. The SQLite facade changes only the schema constant and retains 3,167 source lines with
no inline cases. The retained dedicated catalog suite is unchanged.

## Native analysis integration — 2026-11-04

At `b452ceee`, the production reader uses the shared held source and the admitted normalization/hash
pipeline.
The evaluation probe now imports those helpers instead of retaining a second implementation.
Source discovery covers unequal encoded lengths independently of classification/model state and
reuses verified content evidence. Publication retains current-source, cancellation and run-lease
checks in the same transaction as evidence admission. Candidate matching persists a continuation
after at most 128 postings; group references and direct members survive interruption.

Eight R6 cases passed: the two migration cases above, two decoder contracts, three persistence and
matching cases, and pre-dispatch cancellation. The 140-content dense fixture stops within one posting
window, reopens the actual SQLite catalog, and completes with two groups and 141 direct memberships.
A shared middle member appears in both groups; endpoints 20 bits apart never become members of the
same reference through that middle member. Other cases cover unequal-size discovery, cached evidence,
changed sources, cancellation and expired-owner rejection. The shared source guard keeps its retained
[Windows lease evidence](r6-engine-evaluation.md); these new fixtures do not use real library roots.

The generated opaque session exists before dispatch, retains cancellation and foreground suspension,
and exposes bounded progress snapshots. FRB 2.12.0 initially failed while discovering the internal
`LocalPreviewSources` adapter. Excluding the adapter module from bridge discovery preserves all 113
existing API entry names and adds only the five similarity-session entries. Rust and Dart generated
hashes agree; the repository bridge command completed an optimized native build. The new run method
is registered with the existing asynchronous-contract gate; all 32 entries passed.

`image_hasher =3.1.1` is now a runtime dependency and `image` is pinned to `=0.25.10`. MIT notices for
the selected engine/decoder and relevant dependencies are included as an asset and registered with
the existing license page. No model, native runtime or original-media changes are included.

Source / inline-test / dedicated-test line counts are 344 / 0 / 102 for the image-reader family,
842 / 0 / 284 for similarity persistence, and 381 / 0 / 18 for the session/execution family. Source
counts include module declarations. Existing SQLite coordination adds only the feature module.

The [schema-74 CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41820364101)
reported two availability checks with outdated module inventories and one schema-33 preservation
check that treated newly created similarity tables as retained rows. The inventories now admit only
the exact added module declarations and bridge-discovery annotation. The migration case explicitly
asserts empty similarity tables and the initial dispatch row before comparing every retained row.
All ten focused availability checks, including rejecting mutations, and the schema-33 migration/reopen
case passed locally. These changes add no test cases or broad exclusions. The availability facade
has 2,603 pre-test lines and 3,550 inline-test/support lines; the migration case file has 382 lines.

## Group queries and durable operations — 2026-11-04

The native review contract now provides bounded group/member pages, seek, preference, ignore/restore,
member exclusion and exclusion-specific undo. Current fingerprint compatibility owns membership;
exact-copy representative selection reuses exact-review policy. Groups below two visible contents
are omitted. Degree changes query the existing evidence. Decision revisions reject stale writes,
and unavailable preferences remain saved without selecting replacement content.

Two SQLite fixtures passed, covering exact-copy projection, degree boundaries, preference and ignored state,
member removal, stale writes/undo, reopen, current-source replacement and group-scoped exclusions.
The 70-group fixture returns 64 then six groups and clamps seeking to the last page. The retained
choice case also passed after adding derived-evidence removal and reconstruction of the same group
through production matching, with its preference, ignore and exclusion still effective.
The review persistence family has 570 source lines, no inline cases and 353 dedicated test lines;
the application review boundary has 129 source lines.

Canonical bridge generation preserves all 118 prior API entries and adds seven review operations.
The optimized native build, bridge guardrail fixtures and all 39 asynchronous contracts passed.
The [native-analysis CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41828314572)
exposed the guardrail fixture's pre-R6 roster count of 31; the count and scoped module checks now
include the analysis session and review operations without weakening synchronous-call rejection.

Automatic desktop scheduling and actual desktop interaction remain unconnected and unverified.
Real-library accuracy, constrained-client resource measurements and the complete R6 user workflow
retain their acceptance scope in the roadmap.
