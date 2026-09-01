# ADR 0007: Reconcile Windows locations with versioned file identity evidence

- Status: Accepted
- Date: 2026-08-07

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

Reconciliation applies these rules in order:

1. matching platform identity preserves `Asset` identity across the same path, rename, move, or
   in-place edit;
2. derived metadata and previews are reused only when size and modification time also match;
3. an unchanged same path may backfill identity for a pre-v9 or identity-unavailable catalog row;
4. a changed same path without matching identity receives a new `Asset` identity;
5. when identity is unavailable on both scans, only an unchanged same path is reused;
6. a completed scan atomically replaces the active root snapshot, so absent locations disappear
   from the current library without touching source media;
7. current-scan identity candidates are deterministic, while historical identity lookup is limited
   to active snapshots so staged rows do not require a per-file database reconciliation query;
8. location staging uses bounded 128-row transactions, and schema v10 indexes location identity,
   active file identity, and asset references used by terminal cleanup;
9. exact byte identity remains a separate future `ContentFingerprint` and can supersede weak
   reconciliation evidence without changing this contract.

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
final-path string returned by a handle. The Win32 adapter opens the actual local DOS volume root
(`C:\` for a C-drive root) with `FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`; it does not
open a long-DOS volume spelling. Each descendant prefix is then opened by name relative to the
already pinned parent through the ADR 0024 `NtCreateFile` adapter with pure `FILE_TRAVERSE`.
Volume and descendant guards use read/write sharing but no delete sharing plus backup/open-reparse
semantics. Directory guard opens deliberately do not request no-recall. A separate root-relative
`FILE_READ_ATTRIBUTES | SYNCHRONIZE` handle validates the child's final canonical path,
volume, complete 128-bit identity, directory and reparse attributes, local availability, NTFS
volume, and the parent directory's real case semantics before the next prefix is admitted. The
entire RAII chain remains owned until the catalog transaction commits or rolls back. Failure to
expand a configured path to its long DOS form, open any prefix with those minimal rights, or
reproduce a persisted root-generation identity is a capability failure and closes publication;
path expansion is never accepted as identity or authority.

## Validation gates

- identity is stable across a same-volume rename of a controlled Windows fixture;
- a different file at the prior path does not inherit the old asset identity;
- an in-place edit preserves asset identity but invalidates metadata and preview reuse;
- a renamed unchanged file preserves asset identity and rebuildable evidence;
- an absent location disappears only after atomic publication of a complete replacement snapshot;
- v8 migration preserves every active location and marks its file identity unknown;
- v9 migration preserves file-identity evidence and adds every schema v10 reconciliation index;
- revalidation detects identity replacement in addition to size and modification-time changes;
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
  access, sharing, and flags. An ordinary-user disposable root below Public Documents proves the
  complete volume-to-root chain: the `C:\` volume handle uses
  `FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`, each root-relative descendant guard uses
  pure `FILE_TRAVERSE`, and all open no-follow without delete sharing; a held root
  rename fails with Win32 32, and rename succeeds after the guard drops. Temporary owned ancestors
  separately prevent rename and deletion while held. The workspace sandbox denies access to one
  otherwise valid profile ancestor with Win32 5; this is retained as environment-specific negative
  capability evidence, not represented as a Windows or positive profile-chain result. A controlled
  guard-capability failure fixture proves preservation of catalog, `Current`, checkpoint, and
  authority state and projects `RecoveryRequired`.
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
- Cross-volume moves receive a new platform identity and therefore a new `Asset` until exact content
  evidence is available.
- Full scans can determine removals. A deliberately limited validation scan must not claim a
  complete removal count.

## Replacement strategy

Add another identity scheme behind the filesystem adapter or replace platform evidence with a
stronger reconciliation engine. Scheme and value remain versioned, old rows remain traceable, and
the catalog can reanalyze them without exposing platform types to application or presentation code.
