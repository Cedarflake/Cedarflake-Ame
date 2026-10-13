# R2c-P persistent journal continuity

Status: implementation checkpoint; not accepted

## Scope

This checkpoint owns durable per-root journal capability, source-range enrollment, checkpoint
advancement, and cross-root lineage under ADR 0024. It does not authorize a real library, real SCM
or FSCTL execution, source enumeration, media reads, source mutation, or the R2c-Q baseline.

## Data and transaction invariants

- Schema v24 supersedes v23. It retains canonical decimal journal, volume, and nonnegative USN
  storage plus exact-width V2/V3 file references, and adds a complete canonical source-range batch
  payload, exact carried-lineage journal/OLD/NEW/carry identity, and SQL insert/update enforcement
  for 64-character lowercase hexadecimal source-range IDs. Durable pending rename carries remain
  bounded at 1,024.
- Catalog open compares complete canonical DDL for all nine v24 journal tables and both
  source-range identity triggers. Its quote-aware canonicalizer normalizes only formatting outside
  quoted tokens and preserves exact literal, quoted-identifier, BLOB, GLOB, `RAISE`, and escaped-
  quote bytes. Catalog open recomputes every source-range ID from its payload, verifies that
  the payload prefix exactly matches the immutable range coordinates, and
  validates the exact marker, columns, indexes, foreign keys, active root generation authority,
  checkpoint/range bounds, queue lineage, cross-root ownership, and every persisted domain value.
  Replacing an otherwise unlisted `CHECK` with `CHECK(1)` is rejected even when the total check
  count is unchanged.
- The v21-to-v22, v22-to-v23, and v23-to-v24 migrations are atomic. Active `running` or `comparing` inventory is
  terminalized as `superseded` with no absence authority, while its entries and other catalog,
  queue, foreground scan, preview, terminal-media, and handoff evidence remain durable.
  A v23 range with exact single-observation queue/carry evidence is deterministically rekeyed with
  every foreign key and both queue watermarks in the same transaction. A v23 lineage cannot prove
  the new consumed-carry identity and therefore fails closed before mutation; repeated reopen sees
  the unchanged v23 catalog.
- Migrated active roots receive `Unknown` capability and `BaselineRequired` continuity with no
  fabricated checkpoint. Executing that baseline belongs to R2c-Q.
- Journal authority is keyed by root and configuration generation. Replacing a root generation
  retires the old authority without deleting its durable source ranges or lineage, while repository
  reads and checkpoint writes admit only the currently active generation.
- Durable range and handoff IDs include complete volume GUID and serial evidence, journal/range
  identity, and every participant root ID and generation. Reusing the same USN coordinates after a
  root generation or volume identity change cannot collide with old evidence.
- One `IMMEDIATE` volume-batch transaction publishes every successful page from a shared response:
  source ranges, final-state queue intents, generic and R2c-P queue lineage, pending carry mutation,
  both cross-root owners, checkpoint compare-and-swap, and root state. Any owner capacity, stale
  generation, database error, or injected crash rolls back every page and checkpoint. A failed root
  may be omitted only when the broker's proof keeps the surviving interval unrelated to a potential
  handoff.
- Queue capacity degradation, stale generation, mismatched identity, malformed replay, or any
  transaction failure leaves the prior checkpoint unchanged.
- Checkpoint advancement occurs inside that `IMMEDIATE` publication transaction. It requires the current
  next-unread USN to equal the enrolled range start and exact root, generation, volume GUID and
  serial, journal, protocol, contract, range, checkpoint, and root-state identity. Covered/end and
  revision/time evidence cannot regress; the range, checkpoint, and root state advance through
  full-row compare-and-swap. Only exact replay is idempotent, and no implicit baseline is created.
- Journal candidates enter the existing final-state reconciler as `Reconcile` or
  `RenameCandidate` intents. Journal evidence never directly publishes a deletion.
- Broker protocol v5 admits at most eight distinct registered root generations from one volume and
  journal. A bounded page issues exactly one physical `FSCTL_READ_USN_JOURNAL`; a partial native
  buffer returns its proven covered/next boundary for the following page. Post-admission
  handle/ACL/containment/resolution failures remain root outcomes and do not suppress a healthy
  sibling; only the shared FSCTL failure is volume-wide.
- Endpoint uncertainty establishes the earliest rename USN at or after the affected root's start as
  a proof barrier. A healthy sibling may publish the earlier non-rename prefix, but cannot cross
  that barrier. A known OLD before a later uncertain NEW is retained as durable carry. Pending-OLD
  ACL, identity, containment, and malformed-path failures are attributed to that owner instead of
  aborting the volume; a page with no rename still permits the healthy sibling to reach its proven
  end.
- If source endpoint authorization is complete but target translation fails, the service removes
  the failed target candidate and handoff, persists the exact source OLD as pending carry, and uses
  NEW as the barrier. Source failure or any inability to construct that carry exactly falls back to
  OLD without fabricating evidence. Multiple handoffs select the earliest safe barrier.
- Every successful query first participates in the common minimum exclusive end. Roots already at
  or beyond that boundary are successful no-ops and do not enter the
  shared read. If every checkpoint equals the end, the session issues zero FSCTL, zero enumeration,
  and zero media read.
- Protocol v5 rejects v2, v3, and v4 and explicitly carries a bounded durable pending OLD record.
  Handoffs are accepted only when their current endpoints and same-page previous endpoints exactly
  match the original request and successful proof, while a carried previous endpoint exactly matches
  its durable source range, carry ID, file reference, path, generation, and OLD USN. The codec,
  client, and session reject malformed, unknown, replayed, failed, or half-lineage evidence.
- Cross-root rename evidence is emitted only when old and new paths each have one unambiguous
  admitted owner. Both paths are rechecked through their own live root capability and containment
  boundary; raw records and absolute paths never cross the broker response.
- OLD and NEW remain one semantic event. If a page, evidence limit, or native buffer ends between
  them, the OLD is durably carried instead of rereading one fixed buffer. Per-root starts filter
  records before containment and budgeting, semantic events are USN ordered, and the first event
  that would exceed a limit establishes a strictly advancing partial proof without deterministic
  root-order starvation or livelock.
- A raw page only proves journal coverage and leaves the checkpoint `CatchingUp`. The production
  final-state reconciler publishes `Current` in its terminal transaction only after all required
  queue, range, and lineage evidence is terminal; failure and retry remain `CatchingUp`.
- A complete empty range with no queue, carry, or lineage work is terminalized in the publication
  transaction and may become `Current` at its captured boundary. An empty partial range is also
  terminal as a range, but the root remains `CatchingUp` at its strictly advancing next boundary.
- Completed lifecycle history is cleaned in bounded batches. Pending carries, unresolved queue work,
  current checkpoints, active authority, cross-root owners, the newest retired-generation proof,
  and 64 recent completed ranges for an active generation remain protected.
- The source-range ID is the durable normalized batch digest. It binds range identity and time,
  every intent field, pending carry content, all lineage endpoints and owners, and handoff state.
  Same-content crash replay is idempotent; a changed intent, observation time, added or changed
  handoff, or changed carry conflicts while retaining the old checkpoint.
- Consuming a carry requires exactly two identical carried-lineage owners. Their source range,
  source and target root IDs and generations, volume, journal, file reference, previous path,
  OLD/NEW USNs, and `previous_carry_id` must match the durable carry exactly. Carry deletion,
  lineage/owner publication, and checkpoint advancement share one transaction; omission,
  duplication, or any coordinate mismatch rolls back.
- Catalog reopen requires every pending or completed cross-root lineage to have exactly two owners,
  one previous and one current. It traverses both owners and reverse-checks role, root generation,
  source range, volume, journal, OLD/NEW coordinates, previous carry identity, and the corresponding
  child in the canonical batch payload. The production terminalizer repeats the check in its
  transaction before either root can become `Current`.
- Every retained canonical source-range payload is a complete child manifest. Its pending carries
  must have one exact pending row or one exact consumed-lineage proof through `previous_carry_id`,
  and its lineage entries must have the parent plus exactly two matching owners. Every durable
  carry, consumed proof, lineage, and owner must also occur in its owning payload. Missing,
  duplicate, extra, or payload-only children fail closed and prevent terminalization. Cleanup may
  remove children only with their complete terminal lineage/range proof cluster in one bounded
  transaction.
- Fresh v24 and upgraded prerelease-v24 triggers reject null, non-text, non-64-character, and
  non-lowercase-hex source-range IDs for inserts and updates. Only the complete known legacy trigger
  pair is upgraded transactionally; missing, mixed, or weakened definitions fail closed.

## Controlled evidence

The injectable native Windows seam counts exactly one physical journal control call for a partial
page. It also proves per-root start filtering, request-order independence, whole semantic rename
budgeting, a carried OLD separated from NEW by 960 unrelated records and a native buffer boundary,
and long-path/evidence overflow isolated to the affected root. The controlled volume-reader fixture
performs one shared read for two roots and records zero root enumeration, media read, and source
mutation calls. Its pages cover create, modify, delete, same-root rename, same-path replacement, and
a cross-root move. Each root enrolls and checkpoints independently, and post-admission failure for
one root leaves the sibling checkpoint advancing.
Equality fixtures prove all-equal roots perform no shared read and a root already at the common end
does not block a sibling that still has work.

SQLite fixtures prove atomic enqueue-and-checkpoint across process reopen, exact full-content crash
replay, rejection of gaps, out-of-order ranges, regressions, implicit baselines, and protocol or
contract mismatches. A cross-page carry fixture reopens with OLD durable, rolls back injected crash
and per-root capacity failure, then consumes/deletes the carry while publishing both lineage owners
and the NEW checkpoint atomically. Real SQLite plus the production final reconciler proves
source-first and target-first carried moves retain one asset ID and compatible preview evidence,
first publish only the safe non-rename prefix while the sibling checkpoint remains unchanged,
remain `CatchingUp` after the first terminal endpoint, and become `Current` only after the second.
Long-running fixtures retain 64 completed active ranges, preserve unresolved retired-generation
evidence, and clean older terminal history across reopen. Migration fixtures prove v23 forward
migration and deterministic rekey, terminal and pending lifecycle backfill, queue-watermark/FK
rewrites, unprovable-lineage rollback across repeated reopen, arbitrary-ID rejection, payload and
range-coordinate tamper rejection, malformed partial-v22 rollback, and complete canonical-DDL
validation, including a same-count `CHECK(1)` replacement.

The protocol-v5 service fixture proves exactly one shared backend call, malformed root counts,
cross-volume and duplicate root-generation rejection, isolated authorization and post-admission
failure, bounded root-relative handoff translation, and no absolute-path disclosure. Raw codec,
client, and session fixtures reject request-unbound outcomes and incomplete lineage. The
session-backed reader registers and queries each live capability, captures the common exclusive end,
submits only roots with a nonempty interval, translates candidates and per-owner lineage, closes the
session, and preserves a sibling result when one root fails. Coordinator fixtures count extra and
missing roots independently.

Third-review remediation fixtures additionally exercise source- and target-side endpoint failures,
the per-root-start earliest rename barrier, durable OLD retention before an uncertain NEW,
pending-carry authorization failure, malformed pending paths, and no-rename sibling progress.
Repository negatives cover missing and duplicate owners plus wrong generation, path, reference, and
source range, with the durable carry and both checkpoints unchanged after every rejection.

Fourth-review remediation fixtures exercise target failure after verified source translation,
source failure before trustworthy carry construction, and multiple handoffs selecting the earliest
safe boundary. Production service framing, client, session, actual SQLite publication, close/reopen,
target recovery, and the real final reconciler preserve logical asset, file identity, and preview
compatibility in both source-first and target-first order. Reopen negatives delete either owner,
duplicate a role, or alter generation, range, and payload child evidence; each fails closed, and an
owner-loss terminalization attempt cannot publish the sibling `Current`. Typed trigger negatives
cover null and integer insert/update values, while the exact prerelease-v24 pair upgrades atomically.

Fifth-review remediation fixtures delete a complete lineage parent with cascading owners, an
unconsumed pending carry, and a consumed-lineage proof; each retained-payload mismatch fails catalog
reopen, and terminalization cannot publish `Current`. Valid-rekey negatives add duplicate, extra,
and payload-only child evidence without first tripping the source-range digest check. A real SQLite
cleanup fixture proves that only a closed completed lineage/range cluster is deleted in one
transaction and that the catalog reopens with newer proof intact. Canonical-DDL negatives mutate
quoted literal case or whitespace, GLOB case, `RAISE` text, and escaped quotes; every mutation is
rejected while formatting-only canonical DDL and the exact legacy trigger upgrade remain valid.

Sixth-review remediation makes the canonical payload's normalized intent set bidirectionally exact
with retained queue rows and source-range ownership. Missing, extra, or altered queue rows,
ownership, payload intents, and cross-root peer enrollment provenance fail catalog reopen and the
production terminalizer before either root can become `Current`. The production coalescer and the
validator share the same normalization path, so a valid two-observation coalesced row survives
`retry_wait`, terminal completion, bounded cluster cleanup, and reopen. Terminal queue evidence is
retained until its closed source-range cluster is removed in the same transaction. Lineage state is
also proven rather than trusted: `Completed` requires both exact owner range lifecycles completed;
`Superseded` requires durable root-generation retirement authority. Premature completion,
single-ended completion, or forged supersession fails closed, while either endpoint completion order
converges and reopens correctly.

Focused deterministic evidence for this remediation was run serially. Rust commands below were run
from `rust/`; repository scripts were run from the repository root:

```text
cargo fmt --all -- --check
  passed
cargo clippy --all-targets --all-features -- -D warnings
  passed; 0 warnings
cargo test --lib application::persistent_journal_continuity -- --test-threads=1
  15 passed; 0 failed
cargo test --lib application::incremental_library_changes -- --test-threads=1
  32 passed; 0 failed
cargo test --lib adapters::sqlite_catalog::persistent_journal::tests -- --test-threads=1
  32 passed; 0 failed
cargo test --lib adapters::sqlite_catalog::migrations::tests -- --test-threads=1
  39 passed; 0 failed
cargo test --lib adapters::sqlite_catalog::change_queue::tests -- --test-threads=1
  56 passed; 0 failed
cargo test --lib adapters::sqlite_catalog::catalog_delta::tests -- --test-threads=1
  15 passed; 0 failed
cargo test --lib journal_broker -- --test-threads=1
  142 passed; 0 failed
cargo test --test journal_broker_binary --no-fail-fast -j1
  3 passed; 0 failed
cargo test --lib --no-fail-fast -j1
  675 passed; 0 failed; 11 ignored
./tool/integration_test_windows_journal_broker.ps1
  40 of 40 exact filtered tests passed; windows_journal_broker_integration_passed
./tool/release_test_journal_broker_installer_guardrails.ps1
  journal_broker_installer_guardrails_passed
./tool/acceptance_test_windows_journal_broker_guardrails.ps1
  AME_BROKER_ACCEPTANCE_GUARDRAILS status=passed
protocol/manifest static consistency
  journal_broker_protocol_manifest_static_consistency_passed
git diff --check
  passed
```

## Acceptance boundary

This checkpoint is not R2c-P acceptance. The controlled closed-process process-level run, complete
Daily and Windows Release gates, and independent read-only audit remain outstanding before the
roadmap may mark R2c-P complete. No real library, elevated SCM operation, or real FSCTL acceptance
was run for this remediation. R2c-O's authorization-bound elevated service evidence also remains an
explicit separate gap and is not implied by deterministic or non-accessing guardrails.
