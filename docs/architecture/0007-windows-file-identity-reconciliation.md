# ADR 0007: Reconcile Windows locations with versioned file identity evidence

- Status: Accepted
- Date: 2026-08-07
- Last updated: 2026-09-05

## Context

An incremental rescan must distinguish four different events without hashing every source file:

- the same unchanged file at the same path;
- the same file edited in place;
- the same file renamed or moved;
- a path that now refers to a different file.

Path, size, and modification time alone cannot make those distinctions reliably. A path is a
location, not long-term asset identity. Treating every same-path file as the same `Asset` would let a
replacement inherit stale metadata, previews, and future user decisions. Treating every new path as
a new `Asset` would lose identity on an ordinary rename and create unnecessary analysis work.

Windows exposes `FILE_ID_INFO`, which combines a volume serial number and 128-bit file identifier.
Microsoft documents that the pair identifies a file on one computer at a point in time. File IDs
remain filesystem-specific and can eventually be reused, so they are reconciliation evidence rather
than a content fingerprint or permanent global identifier.

## Considered options

### Path, size, and modification time

Retained only as a fallback when platform identity is unavailable. It can prove that a known path
has not observably changed, but it cannot prove a rename or distinguish every replacement.

### Full or sampled content hashing during R1 discovery

Deferred. Full hashing belongs to the exact-duplicate engine and would multiply reads across the
large library. Sampled hashing still opens cloud and failure-policy questions and cannot replace
the platform identity needed to recognize an edit in place.

### `file-id` 0.2.3

The crate provides a small safe cross-platform API and is maintained under the notify-rs project
with MIT or Apache-2.0 licensing. Its public `FileId` fields are intentionally private, however, so
persisting a stable Ame-owned representation would require depending on its debug or optional
serialization format. Third-party formats cannot become Ame catalog contracts. The crate remains a
useful behavior reference but is not admitted for this slice.

### Windows `FILE_ID_INFO` adapter

Accepted for the Windows-first release. Ame already depends on `windows-sys` for cloud placeholder
attributes. A narrow filesystem adapter can convert the volume serial number and 128-bit file ID
immediately into an Ame-owned, versioned fixed-width value without exposing Windows structures
outside the adapter.

## Decision

Discovery records optional `FileIdentityEvidence` with an Ame-owned scheme and value. On Windows,
the scheme is `windows-file-id-128-v1`; the value is the fixed-width lowercase hexadecimal volume
serial number followed by the 128-bit identifier. A failed identity query becomes a structured,
non-fatal issue and the readable image remains indexable.

Schema v31 adds a separate `SourceRevisionEvidence`. On Windows 11 x64 the scheme is
`windows-file-change-time-100ns-v1`; the value is the raw signed `FILE_BASIC_INFO.ChangeTime`
bit-pattern encoded as 16 fixed-width lowercase hexadecimal digits. Directory inventory obtains
the same field from `FILE_ID_EXTD_DIR_INFO`. Identity and revision are read from the same opened
attribute handle when a terminal file is opened. Neither operation requests file-content access or
hydrates a cloud placeholder.

`ChangeTime` is not a content hash and cannot prove byte equality across machines or against an
adversarial timestamp restoration. It is low-cost filesystem change evidence. The catalog therefore
also allocates a catalog-wide, non-reused `source_generation` whenever Ame observes a dirty event,
replacement, or known/unknown or unequal revision transition. Locations for the same trustworthy
Windows File ID share an invalidation generation in one catalog transaction. This closes the ABA
hole where content is replaced and its size, modification time, and ChangeTime are restored after
Ame already observed the mutation. A generation does not establish byte identity.

Path and filename equality never constitute preview or metadata identity. An in-place overwrite at
the same path may retain the logical asset when its Windows File ID is retained, but it must advance
the source generation and invalidate every derived owner. Delete-and-recreate or atomic replacement
at that path receives the replacement identity and cannot inherit the previous asset's preview.

A non-conflicting precise path reconciliation produced by a live notification or startup journal
replay is durable dirty evidence. Root, subtree, and rename compaction must not absorb that evidence
while the bounded queue can represent it. A later precise path that invalidates an already leased
broad rename remains independently runnable beside conservative root recovery. Mutually
contradictory rename lineages still fail closed to root recovery because neither removal edge is
safe to publish independently. If the hard queue bound is genuinely exceeded, the surviving root-scoped
`FreshnessUnknown` owner must promote to recovery-authorized metadata inventory. Gap-authorized
inventory treats otherwise matching existing files as bounded path candidates, and those candidates
advance the source generation before derived evidence can be reused. This deliberately sacrifices
cache reuse rather than correctness when exact continuity has been lost; ordinary inventory without
one of the recorded gap authorities does not invalidate unchanged sources.

Reconciliation applies these rules in order:

1. matching platform identity preserves `Asset` identity across the same path, rename, move, or
   in-place edit;
2. derived metadata and previews are reused only when size, modification time, and a known source
   revision match and no live/startup dirty event overrides that evidence;
3. an unchanged same path may backfill identity for a pre-v9 or identity-unavailable catalog row;
4. a changed same path without matching identity receives a new `Asset` identity;
5. an unknown source revision never proves an unchanged file; legacy v30 rows establish a revision
   baseline only during a later bounded, on-demand source read, not through a startup full scan;
6. a completed scan atomically replaces the active root snapshot, so absent locations disappear
   from the current library without touching source media;
7. current-scan identity candidates are deterministic, while historical identity lookup is limited
   to active snapshots so staged rows do not require a per-file database reconciliation query;
8. location staging uses bounded 128-row transactions, and schema v10 indexes location identity,
   active file identity, and asset references used by terminal cleanup;
9. exact byte identity remains a separate future `ContentFingerprint` and can supersede weak
   reconciliation evidence without changing this contract.
10. a causally complete watcher/journal rename with matching identity and revision preserves the
    source generation; metadata-inventory rename inference remains conservative and invalidates.

The source generation is allocated from one catalog-wide monotonic sequence and is never a
per-path counter. When a dirty observation affects one member of a trustworthy File ID hardlink
set, reconciliation invalidates every active location in that set in the same transaction and
assigns the shared new generation. This prevents per-location generation reuse from creating an ABA
match. A generation remains observation history, not proof that two files contain equal bytes.

A running full scan owns a staged snapshot, not the active projection. Its File ID observations are
compared-and-set against the complete active identity state captured at staging, but a changed
staged observation never fans out into active rows before final source revalidation and atomic scan
publication. Multiple aliases inside the same staged scan share one provisional observation. At
publication, a compatible observation adopts the active generation; a changed observation receives
one newer generation and invalidates active aliases in the same catalog transaction that switches
the root snapshot. Abandoning a scan therefore cannot leave its stale metadata, failure state, or
preview invalidation in the last published gallery.

Phase 34 adds the opposite, change-priority direction without weakening that boundary. A revalidated
P0 Live delta may update the current active projection while a replacement scan is running, but the
same `BEGIN IMMEDIATE` transaction must mirror the resulting source generation, revision, physical
and media state, preview invalidation, removal, and terminal evidence into that scan's staging.
Identity-wide valid and terminal hardlink fan-out covers both active aliases and aliases staged by
the exact running scan. Scan publication then keeps the greater compatible generation and lets a
newer active generation replace older incompatible staged evidence; incompatible equal-generation
state fails closed. An abandoned scan still removes only staged state and cannot roll back the live
delta or its completed queue row.

Terminal decode or format evidence follows the same physical identity rule. The current path stays
in the active projection as a zero-geometry, failed-preview placeholder instead of disappearing.
Every active hardlink alias receives the same revision, generation, terminal issue, and derived-data
invalidation. A later valid observation through any alias restores the complete active identity
group to pending-preview state and removes the identity-wide terminal evidence. Source bytes are
never changed by either transition.

Schema v31 propagates revision and generation through discovery, reconciliation, terminal evidence,
metadata inventory, change deltas, spool and scan/catch-up handoffs, locations, and previews. Legacy
locations receive a catalog-wide nonzero generation during migration but keep a NULL revision and
lose weak preview ownership. That NULL becomes a baseline only when bounded preview or inspection
demand opens that specific source; startup does not scan or hash the library to fill it.
If a v30 File ID group contains contradictory stored size or modification-time observations, the
migration cannot choose a truthful winner. It conservatively clears that rebuildable physical
identity evidence for the affected rows and assigns independent nonzero generations. The locations
and assets remain visible, and a later bounded observation can re-establish identity; migration does
not read source media, invent a revision, or make the catalog unopenable merely because historical
snapshots observed a hardlink at different times.

The v30-to-v31 migration also repairs only the exact legacy terminal metadata-inventory states that
the current schema already recognizes as safely derived. Before v30 contract validation, and inside
the same immediate migration transaction, it removes source spools owned by terminal runs and clears
terminal absence authority only when no matching unretired recovery owner exists. Canonical schema
shape is a precondition for this repair. Malformed DDL, an unowned active run, or any other authority
contradiction still fails closed, and the whole repair plus migration rolls back. This prevents a
legacy terminal spool or orphaned terminal flag from being misreported as
`catalog_persistent_journal_contract_unverifiable` during an otherwise valid upgrade.

The adapter calls `GetFileInformationByHandleEx` with `FileIdInfo`. This requires one focused
`unsafe` block. Its safety invariants are:

- the handle comes from a live `std::fs::File` and remains open for the complete call;
- the output pointer targets a live, correctly aligned `FILE_ID_INFO` value;
- the supplied byte count is exactly `size_of::<FILE_ID_INFO>()`;
- the output value is read only after the operating-system call reports success;
- no borrowed Windows pointer or structure crosses the adapter.

No other `unsafe` use is admitted by this decision itself. ADR 0024 later admits a separate,
adapter-only Windows 11 x64 root-relative open and directory-enumeration boundary. That boundary
uses `NtCreateFile` with an already pinned root handle, consumes `FILE_ID_INFO` as part of the root
and directory proof, and opens a second root-relative terminal-file handle only after the
attribute-only handle proves the complete volume-plus-128-bit object identity and local,
non-reparse state. It owns the native `NTSTATUS`, `OBJECT_ATTRIBUTES`, `UNICODE_STRING`, and handle
conversion entirely inside the local-filesystem adapter. Metadata and directory calls keep
`EaBuffer = null` and `EaLength = 0`, request no content-read right, and reject offline or recall
evidence. The second `NtCreateFile` call is issued from a freshly rebound configured-root handle,
uses `FILE_NON_DIRECTORY_FILE | FILE_OPEN_NO_RECALL`, omits delete sharing, and requests content
access only for the terminal file. Before any read it must still match the metadata handle's full
ID and volume, remain locally available and non-reparse, and resolve under that rebound root
identity. Raw volume and device namespaces are rejected before an operating-system open. Each
root-relative component query reads the actual parent directory's case-sensitivity flag and
applies `OBJ_CASE_INSENSITIVE` only for a case-insensitive parent; final containment compares live
root identity rather than lowercased path text. It does not broaden file identity into domain or
application code and must retain this record's fixed-width identity representation and one-owner
handle invariants.

ADR 0024 publication adds a distinct configured-namespace guard. It normalizes the configured path
to canonical long DOS names, never a raw-volume path, NT device path, short-name alias, or a
final-path string returned by a handle. Each prefix must be an available, non-reparse directory. If
the current token can open a prefix with
`FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`, read/write sharing, no delete sharing, and
backup/open-reparse semantics, that one handle both pins the prefix and supplies its attribute,
canonical-path, volume, complete 128-bit identity, directory, reparse, availability, NTFS, and case-
semantics proof. Descendants are opened with the same rights relative to an already pinned parent
under its live case-sensitivity semantics. A prefix for which this exact open returns Win32
`ERROR_ACCESS_DENIED` is admitted only after independent access attempts prove that the token cannot
request `DELETE`, `WRITE_DAC`, or `WRITE_OWNER` on the prefix and cannot request
`FILE_DELETE_CHILD`, `WRITE_DAC`, or `WRITE_OWNER` on its parent. Any other error or mutable
permission fails closed. Directory guard opens do not request no-recall. The configured root itself
must still reproduce the durable root-generation `FILE_ID_INFO` under its held no-delete-sharing
guard. Every acquired guard and the durable root proof remains owned until the catalog transaction
commits or rolls back. Failure to prove an access-denied prefix immutable to the current token or
reproduce the persisted root identity closes publication; path expansion and ACL text are never
accepted as root identity or publication authority.

## Validation gates

- identity is stable across a same-volume rename of a controlled Windows fixture;
- a different file at the prior path does not inherit the old asset identity;
- an in-place edit preserves asset identity but invalidates metadata and preview reuse;
- a renamed unchanged file preserves asset identity and rebuildable evidence;
- an absent location disappears only after atomic publication of a complete replacement snapshot;
- v8 migration preserves every active location and marks its file identity unknown;
- v9 migration preserves file-identity evidence and adds every schema v10 reconciliation index;
- revalidation detects identity replacement in addition to size and modification-time changes;
- revalidation detects raw ChangeTime changes even when size and modification time are restored;
- non-conflicting precise live and startup dirty paths survive broader root, subtree, rename, and
  durable-queue compaction until the absolute capacity bound is reached, while contradictory rename
  lineage fails closed without publishing an unproven removal;
- capacity degradation produces one durable root gap whose authorized recovery invalidates
  otherwise matching existing source generations instead of retaining possibly stale previews;
- schema v30-to-v31 migration preserves catalog locations, leaves revision unknown, assigns
  catalog-wide nonzero generations, shares one only for compatible hardlink observations,
  quarantines contradictory legacy File ID evidence, and detaches legacy preview owners;
- schema v30-to-v31 migration transactionally repairs exact-shape orphaned terminal inventory
  authority and terminal spool state before validation, is idempotent on reopen, and rolls back
  without normalization when inventory DDL or active ownership is malformed;
- a changed running-scan observation cannot mutate the active projection before publication, and
  abandoning that scan leaves the active revision, generation, metadata, and preview state intact;
- a P0 Live change during a replacement scan atomically reaches both active and staged aliases,
  including valid and terminal hardlink fan-out, while scan cancellation preserves the active P0
  result;
- scan publication cannot downgrade a newer active source generation or accept incompatible
  equal-generation physical state;
- valid-to-corrupt-to-valid same-path transitions retain the asset, expose a failed placeholder,
  advance source generation, clear stale previews, and recover without source mutation;
- terminal hardlink evidence reaches every active alias and a valid observation through any alias
  clears it for the complete identity group;
- terminal staged rows and orphan derived asset rows do not grow without bound;
- Rust format, Clippy with warnings denied, tests, Flutter analysis and tests, Windows integration,
  and a Windows Release build pass;
- controlled source bytes and entries change only where the test itself explicitly arranges the
  rename, edit, replacement, or removal before a scan.

## Validation evidence

- A controlled Windows adapter test proves that the versioned identity remains stable across a
  rename, differs for a second file created at the old path, and catches replacement during final
  revalidation even when the expected size and timestamp are arranged to match.
- Schema v10 creation plus v8-to-v9 and v9-to-v10 migrations are covered. A v8 location remains
  intact with explicitly unknown file identity, v9 evidence survives migration, and the SQLite
  query plan uses `asset_locations_asset_id` for orphan cleanup.
- A five-scan Rust workflow covers first publication, rename, in-place edit, same-path replacement,
  and removal. Rename reuses asset and preview state, edit reuses asset but returns preview to
  pending, replacement creates a separate asset, and removal leaves one active location and one
  non-orphan asset row.
- Cancellation after a location has been staged removes terminal staged locations and orphan asset
  rows without publishing the scan.
- The repeatable 10,000-file debug benchmark records 22.570-second cold and 21.030-second warm scans,
  26-millisecond pause response, 20.033-second resumed completion, 117-millisecond cancellation,
  a 27,262,976-byte catalog, and a 15,659,008-byte peak test-process working set. The completed
  snapshot contains exactly 10,000 locations and assets, the cancelled scan leaves no location, and
  sampled source bytes and the source entry count remain unchanged.
- Generated bridge hashes match. Flutter mapping and Windows integration tests preserve the
  Ame-owned identity scheme without exposing Windows structures.
- Controlled Windows 11 x64 fixtures exercise the production namespace wrapper with exact desired
  access, sharing, and flags. An ordinary-user disposable root under LocalAppData proves the
  complete volume-to-root chain: the DOS-volume and every root-relative descendant guard use
  `FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`, backup/open-reparse semantics, read/write
  sharing, and no delete sharing. The same held handle supplies each component's attribute and
  identity proof; a held root rename fails with Win32 32 and succeeds after the guard drops.
  Temporary owned ancestors separately prevent rename and deletion while held. The real ordinary
  token exercises case-sensitive component selection; an execution sandbox that denies the test-
  only `SetFileInformationByHandle` setup records an explicit skip rather than a false pass. A
  controlled guard-capability failure fixture proves preservation of catalog, `Current`,
  checkpoint, and authority state and projects `RecoveryRequired`.
- Publication-capable discovery is a separate private-field, non-cloneable
  `PublicationGuardedFileDiscovery`; there is no `From`, `Default`, public constructor, or ordinary
  `FileDiscovery` upgrade. It implements neither `Deref` nor `DerefMut` and exposes no raw file,
  directory iterator, underlying discovery reference, or detachable handle. Directory traversal is
  available only as opaque cursors whose borrow or owned guard keeps the namespace capability live;
  the durable inventory cursor privately duplicates the pinned handles and drops its directory
  iterator before that duplicate guard. An expected v29 identity is compared against the pinned
  root-relative metadata handle before the enumeration-root handle opens, so a configured-path
  replacement performs zero source enumeration and cannot reach authoritative catalog publication.
- Rust formatting, development/release check, and Clippy with warnings denied pass. The seventh-
  remediation focused reruns pass 34/34 local-file, 41/41 incremental, 11/11 authoritative, and
  35/35 metadata-inventory tests. The seventh-remediation historical snapshot recorded 831 passed,
  zero failed, and 11 intentional ignores in 243.79 seconds, followed by 3/3 binary integration
  tests in 2.09 seconds. That snapshot also recorded passing Flutter analysis and all 297
  unit/widget tests, controlled Windows scan and accessibility integrations at 2/2 each, and an
  internal Windows x64 Release build in 89.84 seconds. It is not final or current R2c-Q evidence;
  ADR 0024 and the R2c-Q implementation checkpoint own the later replacement evidence.

## Consequences and risks

- File identity reads open locally available files without reading their content. Cloud-only
  placeholders remain rejected before this adapter can run.
- Some filesystems or providers may not return `FILE_ID_INFO`. Ame records the limitation and falls
  back conservatively instead of rejecting the image.
- File IDs are not content hashes and may be reused over time. They cannot justify duplicate labels,
  destructive operations, or identity across computers.
- ChangeTime and source generations are also not content hashes. Routine startup must not read or
  hash every media file; exact fingerprints remain an explicit analysis workflow.
- Cross-volume moves receive a new platform identity and therefore a new `Asset` until exact content
  evidence is available.
- Full scans can determine removals. A deliberately limited validation scan must not claim a
  complete removal count.

## Replacement strategy

Add another identity scheme behind the filesystem adapter or replace platform evidence with a
stronger reconciliation engine. Scheme and value remain versioned, old rows remain traceable, and
the catalog can reanalyze them without exposing platform types to application or presentation code.
