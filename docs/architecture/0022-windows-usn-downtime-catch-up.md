# ADR 0022: Catch up Windows downtime through the USN change journal

- Status: Accepted
- Date: 2026-08-18

## Context

ADR 0016 through ADR 0021 provide normalized change evidence, a live Windows observer, a durable
queue, atomic incremental publication, the desktop lifecycle, and bounded authoritative recovery.
They deliberately treat every production start as an evidence gap because a filesystem change may
have happened while Ame was closed. The accepted recovery ceiling is 4,096 enumerated entries, but
the retained target catalog contains 79,013 locations. Its recorded cold scans required 106.905
seconds for `local-primary` and 154.698 seconds for `cloud-primary`, and an unresolved cloud
placeholder can prevent a full scan from claiming freshness. The roadmap trigger for conditional
R2c-G is therefore met: normal startup cannot regain trustworthy freshness within the recorded
large-library budget through bounded root enumeration alone.

Windows exposes a per-volume update sequence number change journal on supported NTFS and ReFS
volumes. The journal is useful as a bounded source of reconciliation candidates, but it is neither
a complete history nor authoritative filesystem state. Records may be trimmed, journals may be
recreated or restamped, file reference numbers are volume-local, and a record may describe only
part of the changes between opens and closes. Microsoft also requires administrator privileges for
change-journal operations. Ame does not request elevation, so a normal desktop token may be unable
to use this optimization. A trustworthy design must validate continuity, reconstruct paths
conservatively, persist progress only after durable enqueue, and fall back to ADR 0021 whenever any
relevant evidence is uncertain.

## Decision drivers

- cover changes made while Ame is not running without a routine full-root scan;
- share one bounded journal read among all configured roots on the same volume;
- preserve root generation, durable queue, and authoritative publication guarantees;
- avoid hydrating cloud placeholders or reading media content during catch-up;
- fail closed on journal recreation, truncation, unsupported filesystems, unavailable volumes,
  insufficient permissions, path reconstruction gaps, and capacity overflow;
- keep Win32 handles, buffers, file references, and errors behind an Ame-owned adapter;
- introduce the smallest reviewed `unsafe` boundary and document every safety invariant;
- keep journal work outside watcher callbacks, SQLite transactions, and the desktop polling mutex.

## Considered options

### Continue full-root recovery after every process start

Rejected for the target workload. It remains the correctness fallback, but its cost is proportional
to every root and a cloud placeholder can keep the result stale indefinitely.

### Persist paths reported by a third-party journal crate

Rejected. No admitted crate removes the need to validate journal identity, record versions,
buffer bounds, root membership, and restart ordering. Adding another platform dependency would
also move Windows types and policy closer to the application boundary without reducing the unsafe
Win32 surface materially.

### Read the journal directly through the pinned `windows-sys` adapter

Accepted. The repository already pins `windows-sys` 0.61.2. The adapter uses synchronous Win32
calls behind Ame-owned contracts and parses returned record bytes with checked offsets rather than
casting arbitrary buffers to variable-length structures.

## Decision

R2c-G adds a Windows-only downtime catch-up adapter. Production establishes the live observer
first and does not run authoritative recovery for a root whose startup boundary is unresolved. A
single cancellable background catch-up operation then groups the current roots by stable volume
GUID, validates each volume once, reads one bounded journal range, reconstructs candidate paths,
filters them into every matching root, and durably enqueues normalized `StartupCatchUp` work. The
desktop polling mutex is never held during journal I/O or path reconstruction.

The durable checkpoint is per volume and contains the stable volume GUID, unsigned journal
identifier, next USN, a deterministic fingerprint of the exact root IDs, generations, and
normalized volume-relative root paths that consumed the range, the associated catalog revision,
and update time. Unsigned journal identifiers and signed USNs are stored as canonical decimal text
so SQLite integer representation cannot truncate their Win32 range. Schema v19 adds this state and
an explicit catch-up contract marker. A prerelease v19 database without the marker fails closed. A
marker-complete prerelease v19 database created before the derived root-relative lookup index was
added repairs only that exact index atomically; an absent marker or malformed index still fails
closed.

Continuity is valid only when all of these are true:

- the stored volume GUID and current volume GUID are identical;
- the stored journal identifier equals the queried journal identifier;
- the stored next USN is at least `FirstUsn` and no greater than the queried `NextUsn`;
- the stored root-set fingerprint matches the current roots and generations on that volume;
- the stored catalog revision is not greater than the current catalog revision;
- the filesystem supports returned `USN_RECORD_V2` or `USN_RECORD_V3` records.

The adapter requests record major versions 2 through 3 with `READ_USN_JOURNAL_DATA_V1`. It reads
at most 65,536 records and 64 MiB of retained parsed-plus-reconstructed evidence per volume, and
produces at most 4,096 normalized candidate observations per root. The byte ceiling counts owned
record names, parsed record storage, and reconstructed full paths before another record is retained;
an overflow falls back instead of allowing long names or deep parent chains to turn the count bound
into multi-gigabyte memory use. It treats file reference numbers and USNs only as transient
reconstruction and ordering evidence. A directory record becomes subtree reconciliation; a file
record becomes path reconciliation. Rename old-name and new-name records with the same file
reference inside one bounded result become one reliably paired rename candidate, including any
later same-path record needed to recheck final state. Unpaired or cross-root names remain
independent path candidates. Every candidate is rechecked through the existing final-state
reconciliation pipeline; journal reasons never authorize a removal or derived-evidence decision
directly.

The adapter snapshots the queried `NextUsn` as the exclusive end boundary. It reads from the stored
USN to that boundary, builds a bounded file-reference map for deleted or renamed parent chains,
opens still-live parents by file ID when necessary, converts the resulting path to a stable volume
GUID path, and then filters by normalized root containment. An unclassifiable record that may
intersect a configured root is an evidence gap, not an ignored event. Root containment and candidate
coalescing preserve exact normalized path spelling because NTFS directories may opt into
case-sensitive names. When child delete records precede their deleted or renamed parent record, the
bounded history may use only a later parent `FILE_DELETE` or `RENAME_OLD_NAME` record; a later new
name or unrelated reused file reference is never accepted as the child's historical parent path.

Checkpoint publication follows durable enqueue. The application first enqueues every resulting
plan and records `catch_up_source = windows_usn_v1` plus the exclusive volume watermark on the
retained queue work. Only after all affected roots commit does it advance the per-volume
checkpoint. A crash between those steps replays an idempotent range; it cannot skip evidence.
Queue coalescing retains the newest compatible watermark without using it as asset identity.

One journal range may describe moves in either or both directions between configured roots on the
same volume. A dependency graph cannot safely order that work because two authoritative roots may
both contain removals and destinations. Before any catch-up delta removes or replaces a location,
the publication transaction copies its file identity, asset identity, compatible metadata, and
preview expectation into a durable handoff snapshot keyed by catch-up source and watermark. Asset
and preview cleanup treat that snapshot as a temporary owner. A later destination first checks the
active catalog and then the matching handoff snapshot, so it can adopt the original asset and
preview identity even after the source location was removed. Once every queue row for the watermark
is terminal, the same publication transaction removes the snapshots and reclaims only artifacts or
assets that still have no active owner. This protocol has no cross-root wait edge and therefore
supports source-first, destination-first, and bidirectional authoritative moves without starvation
or a dependency cycle.

When no trustworthy checkpoint exists, or any continuity, permission, support, parsing, capacity,
containment, or reconstruction check fails, Ame enqueues the existing root-level
`FreshnessUnknown` work for every affected root while the live observer is already healthy. The
current journal identity and `NextUsn` may become a new baseline only after that fallback work is
durable. The root remains `Updating` or `NeedsReconciliation` until catch-up or the fallback
authoritative recovery publishes. Unsupported platforms use the same explicit fallback and never
claim journal coverage.

One background catch-up operation and the existing authoritative recovery worker are mutually
exclusive. Shutdown requests cancellation and retains the worker handle in the same explicit
stopping lifecycle used by ADR 0021; restart is rejected until the previous worker is joined.
Catch-up readiness is evaluated per root: one unavailable or unhealthy root cannot block a healthy
root whose catch-up evidence is already durable from running its authoritative recovery. Subtree
catalog containment and ordering use the same exact-case semantics as USN candidate distribution,
so a case-sensitive `Album` scope cannot consume or remove an `album` sibling.

Per-volume checkpoints are derived coordination evidence. After successfully enqueuing all work and
saving the current checkpoints, Ame deletes at most 128 checkpoints older than seven days per run,
excluding every volume returned by the current catch-up. Cleanup is disabled while any durable
`FreshnessUnknown` row remains pending, leased, or waiting to retry, so retention cannot erase the
watermark context of an unresolved gap. The same bounded cleanup removes terminal handoff snapshots
that were stranded by retirement or interruption. Schema v19 owns the handoff table and its asset
and preview lookup indexes; a marker-complete prerelease v19 database may add that empty contract or
repair a missing derived index atomically, while a malformed named object still fails closed.

### Unsafe boundary and invariants

All new `unsafe` is confined to the Windows USN adapter and is reviewed as one handle-and-I/O
boundary. The following invariants are binding:

- every UTF-16 input is NUL-terminated, lives for the complete Win32 call, and is not retained;
- every output buffer is initialized storage whose byte length is checked to fit `u32` before the
  pointer is passed to Win32;
- synchronous `DeviceIoControl` calls pass a null overlapped pointer and no pointer escapes the
  call;
- returned byte counts are checked against the allocated buffer before any field is read;
- the leading next-USN value and every record length, major version, filename offset, filename
  length, alignment step, and UTF-16 boundary are validated with checked arithmetic;
- variable-length records are parsed from byte slices; no arbitrary output buffer is reinterpreted
  as an aligned Rust reference;
- `FILE_ID_DESCRIPTOR` is fully initialized with the identifier kind matching the V2 64-bit or V3
  128-bit record before `OpenFileById` is called;
- handles are represented by one RAII owner, reject both null and `INVALID_HANDLE_VALUE`, and call
  `CloseHandle` exactly once; borrowed handles never outlive that owner;
- path-result lengths are checked before UTF-16 conversion, and truncation or invalid encoding
  becomes explicit fallback;
- cancellation is checked between bounded reads and reconstruction steps; no SQLite transaction or
  shared runtime lock spans a Win32 call.

## Validation gates

- pure parser fixtures cover valid V2/V3 records, mixed versions, malformed lengths and offsets,
  invalid UTF-16, unknown versions, record, retained-byte, and candidate ceilings, and cancellation;
- adapter fixtures cover journal recreation, trimmed USNs, unsupported filesystems, permission
  errors, unavailable volumes, child-before-parent deletion and rename reconstruction,
  case-sensitive root filtering, and multiple roots on one volume with one journal read;
- migration fixtures cover fresh v19, v18 to v19 preservation, prerelease handoff-contract repair,
  and fail-closed malformed v19;
- application fixtures prove enqueue-before-checkpoint, replay after interruption, root-set and
  catalog-revision mismatch fallback, retained catch-up queue metadata, both path move orders,
  bidirectional authoritative handoff without wait cycles, exact-case subtree capacity,
  unrelated-removal progress, and bounded checkpoint retention that stops on unresolved gaps;
- runtime fixtures prove watcher-first ordering, no authoritative work before catch-up completion,
  fallback recovery, cancellation, bounded stop, and restart ownership;
- deterministic adapter fixtures prove create, modify, rename, and remove candidate coverage; a
  controlled temporary Windows root exercises direct catch-up when the process token permits it and
  otherwise proves the explicit permission fallback without source mutation;
- complete format, Clippy, Rust, Flutter, Windows integration, bridge, Daily, Windows release, and
  synthetic performance gates pass before merge;
- real-library validation remains R2c-H and requires its separately authorized, serial, read-only
  workflow.

## References

- [Microsoft: Change Journal Records](https://learn.microsoft.com/en-us/windows/win32/fileio/change-journal-records)
- [Microsoft: Using the Change Journal Identifier](https://learn.microsoft.com/en-us/windows/win32/fileio/using-the-change-journal-identifier)
- [Microsoft: FSCTL_READ_USN_JOURNAL](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_read_usn_journal)
- [Microsoft: Walking a Buffer of Change Journal Records](https://learn.microsoft.com/en-us/windows/win32/fileio/walking-a-buffer-of-change-journal-records)
- [Microsoft: OpenFileById](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-openfilebyid)
- [Microsoft: GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew)

## Consequences and risks

- Supported volumes can usually cover downtime in work proportional to changed journal records,
  rather than all library entries.
- Journal evidence remains an optimization over the established recovery ladder. Any uncertainty
  costs a bounded fallback but cannot silently mark stale data synchronized.
- Per-volume checkpoints are durable derived coordination data. They are small, but deleting or
  corrupting them intentionally forces authoritative fallback.
- Administrative policy may make raw volume journal access unavailable; Ame remains correct but
  loses the optimization on that volume.
- Path reconstruction is the highest-risk boundary. Conservative fallback may cause extra recovery,
  while optimistic skipping could lose changes, so ambiguity always falls back.

## Replacement strategy

A future platform service, snapshot API, or safer Windows wrapper may replace the adapter if it
preserves the checkpoint, root-set binding, enqueue-before-advance, bounded candidate, explicit
fallback, watcher-first, and final-state reconciliation contracts. Migration may discard only the
derived checkpoint and force fallback; it must not rewrite the catalog, user decisions, or source
media.
