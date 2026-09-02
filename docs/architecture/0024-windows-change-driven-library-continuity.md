# ADR 0024: Drive Windows library continuity from live notifications and the USN journal

- Status: Accepted
- Date: 2026-08-22
- Last amended: 2026-09-02
- Supersedes: ADR 0023
- Historical predecessor: ADR 0022

## Context

ADR 0016 through ADR 0021 established Ame-owned change normalization, the live Windows observer,
durable queueing, final-state reconciliation, atomic catalog deltas, and desktop lifecycle. ADR 0022
attempted to cover changes made while Ame was closed by opening the Windows USN change journal from
the desktop process. The target workstation's ordinary desktop token could not open either retained
volume journal, so ADR 0023 removed production journal access and made a metadata inventory of every
available root the normal continuity authority after every process start.

That replacement is correct about source safety but wrong for the required user experience. Its
normal startup work is proportional to all entries in every root even when nothing changed. A file
created while Ame is closed cannot be discovered until enumeration reaches it, and a live event can
share polling, SQLite, queue, and filesystem capacity with an older inventory. The retained
approximately 79,000-location workload therefore remains scan-driven: complete old-library
verification is on the critical path to proving freshness, while change delivery has no reserved
execution path.

The product requirement is now limited to Windows 11 x64. For a local NTFS root, the USN change
journal is the persistent change source available while Ame is not running. The target permission
failure is a process-boundary problem, not evidence that an O(N) inventory is the correct steady-
state model. Ame can keep its ordinary desktop process unprivileged while a narrowly scoped Windows
service performs only the privileged journal operations. Installation and service update may use
one explicit UAC authorization; ordinary application startup and synchronization may not.

This decision does not revive ADR 0022 as written. Its direct desktop volume access, one global
catch-up worker, all-roots enrollment boundary, fixed record ceiling fallback, and automatic
inventory scheduling are historical implementation evidence. The accepted replacement preserves
its useful checkpoint and enqueue-before-advance invariants behind a new broker and application
contract.

## Decision drivers

- make ordinary synchronization work proportional to changes rather than total library size;
- display a running-time change with event-to-visible P95 no greater than one second even while
  catch-up or recovery work is active;
- discover changes made while Ame was closed without a routine root enumeration;
- perform zero source-root enumeration on a no-change normal startup;
- keep the desktop process at ordinary user privilege after installation;
- prevent a privileged component from reading media content, mutating source files, changing the
  journal, or disclosing records outside an authorized root;
- preserve final-state reconciliation, root generation, identity, atomic catalog publication,
  durable retry, and last-trustworthy-catalog rules;
- keep OneDrive and other Cloud Files placeholders from hydrating;
- isolate roots so one unavailable or recovering root cannot delay live work for another root;
- retain metadata inventory as a bounded, cancellable correctness recovery tool rather than a
  normal startup mechanism;
- support installation, update, protocol compatibility, diagnostics, and rollback without creating
  a second catalog or synchronization product.

## Supported capability boundary

The first complete continuity contract is for:

- Windows 11 x64;
- a local NTFS volume;
- an installed and compatible journal broker;
- an existing, queryable, continuous USN journal; and
- a configured root whose identity and caller access can be verified.

The application detects capability per root at runtime rather than inferring it only from the
Windows version. FAT, FAT32, exFAT, ReFS until separately accepted, SMB, UNC, NAS, roots whose broker
cannot be installed or contacted, and NTFS volumes without usable journal continuity do not receive
a false complete-synchronization claim. They are `LiveOnly`, `RecoveryRequired`, or `Unavailable`
as evidence requires. `LiveOnly` observes changes while Ame runs and offers explicit `更新图库`; it
does not silently enumerate the complete root on every startup.

OneDrive and other cloud-backed directories are eligible only when their selected root is on a
supported local NTFS volume. Journal and metadata processing may inspect attributes and identity
without opening placeholder content. An offline, partial, recall-on-open, or recall-on-data-access
entry remains present but unreadable and must not be hydrated.

## Considered options

### Continue startup metadata inventory and tune its pages

Rejected. Smaller batches, different polling cadence, early positive publication, or better status
copy cannot remove the O(N) discovery path or guarantee that new changes preempt old-library work.

### Keep a per-user watcher process running continuously

Rejected as the primary continuity authority. It can reduce common downtime gaps but cannot prove
coverage across logon races, crashes, upgrades, process termination, or periods when the helper is
not running. Correctness would still require a persistent platform log or routine inventory.

### Open the journal directly from the desktop process

Rejected. It repeats the target permission failure and would either require ordinary-start UAC or
mix raw-volume privilege into the UI process. ADR 0022 remains historical and is not reactivated.

### Install one broad privileged media service

Rejected. A service that scans roots, reads media, owns the catalog, or performs reconciliation
would duplicate application policy and expose irreplaceable source data to an unnecessary
privilege boundary.

### Use a constrained journal broker plus the existing live watcher

Accepted. `ReadDirectoryChangesW` remains a low-latency hint while the process is running. A small,
on-demand Windows service supplies persistent USN records across downtime. Both sources enter one
Ame-owned durable intent and final-state reconciliation model. Metadata inventory is invoked only
for an explicit baseline or a proven continuity gap.

## Decision

### Change sources and priority lanes

Production has three distinct lanes:

- **P0 Live**: normalized hints from the ADR 0017 `ReadDirectoryChangesW` observer. This lane owns
  reserved queue, worker, SQLite-publication, and filesystem-revalidation capacity.
- **P1 Journal**: bounded downtime catch-up and watcher-gap replay from the broker. The journal is
  continuity evidence and a candidate source, not final file state.
- **P2 Recovery**: metadata inventory for one-time baseline establishment or a proven continuity
  gap. It is never created merely because the application started or time elapsed.

All lanes feed the same ADR 0016 normalization, ADR 0007 identity rules, source-state inspection,
and ADR 0019 atomic catalog-delta publisher. Raw watcher kinds, USN reason flags, file references,
Win32 errors, service types, and inventory rows never become presentation or catalog domain types.

Lane priority is an execution contract, not only a persisted sort key. P0 has reserved admission and
worker capacity that P1 and P2 cannot consume. A P1 or P2 worker checks for P0 work at every bounded
page and yields before starting another page. P2 also yields to P1. A slow journal read, inventory
enumeration, media inspection, or SQLite writer never runs while holding the desktop polling mutex,
a long application-global lock, or an open catalog transaction. Work selection rotates fairly
across roots within a lane, while priority remains P0 before P1 before P2.

### Layer ownership

The domain layer owns platform-independent change origin, lane, root generation, continuity state,
and reconciliation outcomes. It does not depend on Windows services or USN structures.

The application layer owns observer-first ordering, broker requests, checkpoint validation, durable
enqueue, lane scheduling, retry, cancellation, recovery policy, final-state revalidation, atomic
publication, and root status. It consumes narrow `LiveChangeSource`, `PersistentChangeJournal`,
`MetadataInventory`, queue, catalog, and filesystem ports.

The Windows adapter owns service management and versioned IPC. The broker is a separately packaged
adapter process. SQLite owns checkpoint, queue, lineage, migration, and publication persistence.
Flutter receives only bounded Ame synchronization snapshots and catalog revisions. It does not
open volumes, call the service, interpret USN, enumerate roots, or schedule recovery.

The R2c-O `PersistentChangeJournal` boundary is an object-safe Ame-owned session. It exposes only
root registration, journal query, bounded range read, a session-bound cancellable pending read,
structured operation failures, and close. Named-pipe handles, Win32 values, transport generics,
concrete client types, and raw journal buffers remain inside the Windows adapter. The production
synchronization owner retains either that authenticated session or an exact `LiveOnly` reason and
closes an admitted session during bounded shutdown. A production-owned factory may replace a failed
session by closing the old object and authenticating a fresh one outside the synchronization runtime
mutex; reconnect does not add checkpoint or schema authority.

Starting the production process is not metadata-inventory authority. A newly observed runtime root
starts live observation without synthesizing a continuity gap merely because it entered memory.
Broker absence or protocol mismatch therefore preserves the cached catalog and exact `LiveOnly`
reason while creating no change-queue entry and no metadata-inventory run. A later live evidence
gap may still request explicit recovery under the existing bounded policy. Per-root journal
checkpoint validation, enqueue-before-advance, and the one-time migration baseline remain R2c-P and
R2c-Q work; this foundation does not add or reinterpret persistence schema.

### Journal broker boundary

The broker is an x64 Windows service installed and updated by the signed Ame installer after one
explicit administrative authorization. It is demand-started for a bounded request and need not run
while Ame is closed. Normal desktop startup never requests elevation. Service installation,
replacement, repair, and removal remain installer operations rather than application-side shell
commands.

The broker may:

- identify a local NTFS volume for a caller-authorized root;
- query the existing journal identity and `FirstUsn`/`NextUsn` bounds;
- read a caller-requested, bounded USN interval;
- reconstruct sufficient ancestry inside the service to prove root membership;
- normalize a bounded root-scoped record stream; and
- report structured capability, discontinuity, cancellation, and protocol errors.

The broker must not:

- read, decode, hash, preview, copy, move, rename, delete, or write media content;
- open a source file for content access;
- create, delete, resize, restamp, or otherwise configure a USN journal;
- open or modify Ame's catalog, cache, settings, or user decisions;
- perform final-state reconciliation or publish catalog mutations;
- return a raw volume-wide USN buffer, a root-external name, or a root-external path;
- accept an arbitrary device, volume, root, file ID, or output path without validation;
- access the network or load optional plug-ins; or
- persist caller roots, journal records, or media paths after the bounded request ends.

IPC uses a versioned, length-delimited named-pipe protocol with bounded request and response frames,
timeouts, cancellation, and explicit end-of-range evidence. The pipe DACL and service endpoint admit
only the intended local Ame client identity. The service obtains and validates the caller token,
verifies that the caller can traverse the requested configured root, resolves the root and volume
without following an attacker-controlled escape, and binds the request to an application-provided
root ID and generation. The R2c-O foundation selects an own-process, demand-start `LocalSystem`
service because direct volume journal access requires the service boundary; it does not expose
LocalSystem filesystem or media operations. SCM configures `SERVICE_SID_TYPE_RESTRICTED` and an
exact required-privileges allowlist containing only `SeManageVolumePrivilege`. The service object
grants SYSTEM and local Administrators lifecycle authority and grants Interactive Users only
query-config, start, query-status, and interrogate rights. It does not grant change-config, stop,
delete, or DACL/owner mutation. The protected install-directory and binary ACL allow SYSTEM and
Administrators full control and BUILTIN Users read/execute without write or delete; inherited
writable identities are removed. This exact account, privilege, restricted SID, owner/group, DACL,
and binary ACL plan must still pass real disposable-root SCM/FSCTL acceptance before release.
Convenience is not evidence for a broader `LocalSystem` interface.

The production pipe uses an explicit protected DACL for SYSTEM, Administrators, and local
Interactive Users, rejects remote clients, and keeps exactly two bounded listener instances so one
connected Interactive User cannot monopolize service admission. The first listener uses
`FILE_FLAG_FIRST_PIPE_INSTANCE`; the second is another instance of that same protected pipe, not a
second endpoint or name.
Pipe access is not caller authorization: the server obtains process and session IDs from the pipe,
then requires a versioned client proof that echoes the server-generated random connection ID,
generation, and nonce before impersonation. The proof is a connection-bound challenge response,
not a client identity claim or a cryptographic MAC. The server reads it with bounded overlapped
exact I/O because `ImpersonateNamedPipeClient` uses the security context of the last message read
from the pipe. Only after the proof is complete and exact does the server impersonate the client,
duplicate and inspect the caller token, always revert impersonation, and return a similarly bound
server-accept message. A missing, partial, timed-out, wrong-version, or replayed proof never reaches
request framing. The impersonation token's type and level must be usable for impersonation; its user
SID, session, and authentication ID must agree with the OS-observed client process primary token,
whose `TokenType` and `TokenElevation` are queried separately and bound into the namespace. Client-
supplied process, session, instance, root, volume, and identity claims are only compared with those
observations and pinned handles; they never establish identity by themselves.

Production admission additionally requires the caller process to be the installed Ame client at
`FOLDERID_ProgramFilesX64\Cedarflake Ame\Application\cedarflake_ame.exe`. The service reads the
exact `AMEJBID2` identity manifest from the protected SCM service description, opens the
server-observed client PID, and independently proves its primary-token user SID, session,
authentication ID, elevation state, image path, SHA-256, valid Authenticode signature, and
signer-certificate SHA-256. The installer wrote the manifest only after validating the same exact
publisher for both broker and client, so the protected signer-certificate hash binds that publisher
without trusting a process-supplied string. Protocol version, maximum frame size, broker binary
hash, broker signer, installed client path, client binary hash, and client signer are one manifest
identity. A missing, legacy, malformed, mismatched, unsigned, movable, or user-writable client fails
closed before a request is decoded.

SHA-256 and `WinVerifyTrust` are never executed while the application synchronization mutex or
service connection lifecycle lock is held. Production runs them in the fixed protected broker
executable as an identity-probe child, binds the result to a parent-generated 256-bit nonce, and
requires one exact bounded success record. The child is assigned to a kill-on-close Job Object; a
three-second absolute deadline terminates the job, waits for process exit, joins the bounded stdout
reader, and degrades to `LiveOnly`. Timeout never skips SCM PID, pipe PID, LocalSystem/restricted-SID,
fixed-path, manifest, hash, signature, signer, token, or client checks, so this bound does not weaken
pipe-squatting resistance.

The duplicated client root directory handle is the registration capability input. The duplicate
requests only `FILE_READ_ATTRIBUTES`; it cannot enumerate, read content, or mutate. The broker proves
directory type, NTFS volume GUID, 128-bit file ID, canonical path, caller PID/token binding, and
non-volume-root scope before issuing a server-secret 120-second capability. Each path component is
compared using the case semantics of its parent directory, beginning with the volume root; the
declared path and live handle path must match under that exact profile. Capabilities are bound to client binary
identity, token facts, connection ID, generation, nonce, application client instance, root ID and
generation, volume and file identity, and the duplicated handle value. Each connection retains at
most eight registered roots; replacement drops the old pinned handles and disconnect drops the
entire registry. A service-wide permit pool additionally limits all connections to sixteen pinned
roots and all accepted reads to eight workers; permits move across same-root replacement and are
released on expiry, replacement, terminal completion, or connection teardown.
Idle maintenance actively reaps expired registrations every 25 milliseconds so a peer cannot retain
service-wide root permits merely by keeping a connection open.

Before every journal query and before and after every journal read, the service revalidates the live
pinned handle's final DOS/GUID path, complete file ID, volume, filesystem, and non-reparse state;
rename or replacement is a continuity discontinuity. Before publishing a candidate, the service
impersonates the captured caller token again and opens the pinned root, every ancestor, and the
candidate with metadata-only access plus `FILE_FLAG_OPEN_REPARSE_POINT`. Any reparse point, junction,
cloud redirection, access denial, uncertain absence, final-handle escape, or containment failure
suppresses the candidate; only `FILE_NOT_FOUND` or `PATH_NOT_FOUND` after successful parent proof may
represent a deleted in-root candidate. LocalSystem access never substitutes for caller access and no
candidate check requests media-content rights.

The client transport is a crate-private sealed adapter owned by exactly one authenticated connection
dispatcher. It exposes only bounded, non-blocking send, response, abandon, discard, and close poll
steps. Request processing, cleanup, and close each carry the dispatcher's operation epoch and stop at
an absolute deadline. Close or poison advances that epoch, terminates the bounded registry, rejects
late ready results, and prohibits every later adapter call. The PoC creates no transport helper or
cleanup thread, so an always-pending adapter leaves no detached Rust thread or retained transport
clone. R2c-O must implement the sealed Windows adapter with cancellable overlapped named-pipe I/O and
prove that each poll step remains non-blocking and that cancellation and handle closure complete
within their host-owned deadlines. A pending write owns the exact complete frame that started it;
another cloned client may advance no other frame and cannot claim that write's completion. Partial
header and payload reads retain their offsets across polls. Per connection, buffered responses are
limited to twelve request keys and twenty frames, discarded-response tombstones to sixteen, and
in-flight abandon controls to two; an overflow poisons and closes the connection. Production
connections close after thirty seconds idle or five minutes total lifetime even if the peer retains
the handle.

Publishing a query, read, or cancellation result requires one request-lifecycle compare-exchange
under the same gate that advances the connection epoch for close or poison. That compare-exchange
is the only completion linearization point: a close that wins first supersedes and discards the
decoded payload, while a completion that wins first may publish even if close subsequently makes a
best-effort transport discard obsolete. Dropping a pending read performs only non-blocking atomic
retirement and bounded queue admission. Every client operation pumps a bounded number of queued
cleanup steps before admitting new work, and the future application owner loop may call the same
crate-private maintenance port while idle. Pending poll steps park with deadline-aware exponential
backoff from 50 microseconds to 2 milliseconds; they do not busy-yield.

A validated `ReadRangeAccepted` frame is an intermediate acknowledgement, not an active-read
linearization point. The client may expose a pending read handle only after it rechecks poison and
the connection epoch and changes the request from awaiting-response to active-read under the same
lifecycle gate as close. If close or poison wins that gate first, activation returns `Closed` and no
pending handle exists.

The broker may inspect volume records internally only to establish ancestry. It returns records
only after containment beneath one of the caller-authorized roots is proven. If a record cannot be
classified without exposing root-external information, the broker returns a root- or volume-scoped
continuity gap and no record data. Logs use stable codes and bounded counts, never source paths,
record names, journal payloads, account names, or machine identity.

### Broker unsafe boundary

All new `unsafe` remains inside the Windows broker adapter and is reviewed as one handle, token,
named-pipe, and device-I/O boundary. The following invariants are binding:

- UTF-16 passed to NUL-terminated Win32 APIs is terminated, bounded, and live for the complete
  call; `CompareStringOrdinal` receives explicit UTF-16 unit counts, so its slices may be non-NUL
  terminated, each length is checked to fit `i32`, both slices remain live for the call, neither
  pointer escapes, and a zero return value fails closed;
- every device, service, token, pipe, root, and file handle has one RAII owner and is closed once;
- the identity-probe process and its kill-on-close Job Object each have one RAII owner; every setup
  failure and timeout terminates and waits for the child before output-reader memory is released;
- both null and `INVALID_HANDLE_VALUE` are rejected before a handle becomes owned;
- volume handles used for `FSCTL_QUERY_USN_JOURNAL` and `FSCTL_READ_USN_JOURNAL` are opened with
  `FILE_FLAG_OVERLAPPED`; the input, output, event, and boxed `OVERLAPPED` remain pinned until
  `GetOverlappedResult` observes terminal completion and no pointer escapes the operation;
- overlapped named-pipe buffers, events, and `OVERLAPPED` allocations remain at stable addresses
  until completion; `CancelIoEx` only requests cancellation, so drop either observes completion
  within its bounded drain or intentionally retains the allocation rather than freeing a pointer
  still owned by the kernel;
- returned byte counts, record lengths, offsets, alignment, versions, filename lengths, and UTF-16
  boundaries use checked arithmetic before any field is read;
- variable-length journal data is parsed from byte slices rather than cast to aligned Rust
  references;
- V2 and V3 file-reference widths remain distinct and initialize the matching descriptor exactly;
- caller token, impersonation, DACL, service identity, volume identity, root identity, and final
  containment are revalidated at their owning boundary;
- the server performs no pipe-client impersonation before a complete connection-bound client proof
  has been read and validated; the primary process token supplies elevation while the impersonation
  token supplies callable impersonation authority, and both tokens must agree on user, session, and
  authentication ID before either can authorize a request;
- the process image identity is accepted only after the SCM-protected manifest, fixed physical
  Application path, file hash, Authenticode chain, signer-certificate hash, primary token, and
  impersonated pipe-token facts agree; the handle and token buffers remain live through each
  synchronous query and every allocation has one matching owner;
- cancellation is checked at most every 25 milliseconds during overlapped journal control calls
  and between reconstruction steps. `CancelIoEx` is followed by a terminal completion drain; if
  the NTFS driver does not release the operation within the one-second hard drain, the isolated
  broker process fail-fasts so Rust never frees kernel-owned buffers or leaves a detached
  source-access worker. Connection teardown similarly cancels the registry and drains all accepted
  workers within two seconds or fail-fasts. During service stop, `SERVICE_STOP_PENDING` checkpoints
  advance every 500 milliseconds with a five-second wait hint; and
- no SQLite transaction, Flutter callback, application mutex, or source-content handle crosses an
  FFI or IPC wait.

Focused adversarial tests are required whenever this boundary changes. `unsafe` is not permitted in
the domain or application layer to avoid an IPC contract.

### Handle-anchored local metadata-inventory boundary

P2 directory discovery on Windows is a separate, non-privileged adapter boundary. A path check
followed by `read_dir` is not sufficient: an ancestor can be replaced by a junction between those
operations and both later identity checks can consistently observe the escaped target. The source
adapter therefore pins the canonical selected root with a metadata-only, no-follow, no-recall
directory handle and binds each inventory epoch to that root's ADR 0007 volume-plus-128-bit file
identity. On the supported Windows 11 x64 target, every descendant is opened relative to that live
root handle with `NtCreateFile`; `OBJECT_ATTRIBUTES.RootDirectory` is the pinned handle, and
`OBJ_DONT_REPARSE` plus `FILE_OPEN_REPARSE_POINT` prohibit ancestor and terminal reparse traversal.
Controlled Windows 11 x64 NTFS evidence rejects `FILE_OPEN_NO_RECALL` when it is incorrectly added
to the root-relative directory-component call with `STATUS_INVALID_PARAMETER`, but accepts it on a
terminal `FILE_NON_DIRECTORY_FILE` call. The adapter therefore requests only list/attribute rights
while walking and proving components, keeps `EaBuffer = null` and `EaLength = 0`, opens every
component as the reparse point itself, and rejects every reparse, offline, or recall-on-access
candidate before content access. For a proven local terminal file, it reopens the configured path
with no delete sharing, matches that handle to the persisted root identity, and issues a second
root-relative `NtCreateFile` from that exact guard. Only this terminal call requests
`FILE_READ_DATA` and native `FILE_OPEN_NO_RECALL`; it uses `FILE_NON_DIRECTORY_FILE`, omits delete
sharing, neither opens a raw volume nor resolves an absolute content path, and remains unauthorized
until its complete 128-bit file ID, volume serial, attributes, reparse state, and live containment
match the metadata handle and rebound root. An object moved outside the root, an offline transition,
a different object at the old path, a renamed or replaced configured root, or an unsupported access
upgrade fails before any read. Raw volume and device namespace roots fail before an operating-system
handle open.
A directory must remain a
non-reparse directory with the expected handle identity and is enumerated from that same live handle
with `GetFileInformationByHandleEx(FileIdExtdDirectory*Info)`. Entry
name, attributes, size, timestamp, reparse tag, and file ID come from the handle-owned directory
buffer rather than a second path lookup. The same handle supplies opening and closing directory
identity. A resumed source page, candidate drain, or absence/finalization boundary must first pin
the configured root again and match the persisted epoch root identity; mismatch preserves the last
trustworthy catalog and withholds `Current` and absence.

Case authorization follows the directory that owns each name. Before every root-relative component
open, the adapter queries `FileCaseSensitiveInfo` on the live parent handle and sets
`OBJ_CASE_INSENSITIVE` only when that parent is case-insensitive. Consequently `Photos` and
`photos` remain distinct beneath a case-sensitive NTFS directory, while ordinary case-insensitive
roots retain Windows behavior. The configured root itself is pinned through the OS path resolver,
and containment is proved by reopening the candidate's live root-depth prefix and matching its full
root identity; neither root selection nor containment lowercases path strings.

Every P0 live or P1 journal path, paired rename, root/subtree reconciliation, resumed P2 source
page, owned-candidate publication, absence authorization/page, completion, and finalizer binds both
the active root generation and the persisted v29 publication-namespace identity before content
inspection and again immediately before its catalog transaction. A missing proof is not repaired
from the configured path: the operation fails closed to `RecoveryRequired` or `LiveOnly` and keeps
the prior catalog authoritative.

Publication normalizes the configured path to canonical long DOS names and opens the local DOS
volume root plus every existing ancestor and the root as one RAII chain. The Win32 adapter opens
the actual drive root (`C:\` for a C-drive path) with
`FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`; the native adapter opens every descendant
component relative to its already pinned parent with pure `FILE_TRAVERSE`. Volume and descendant
guards use read/write sharing but no delete sharing plus backup/open-reparse semantics; directory
guards never use no-recall. After pinning a parent, a separate root-relative
`FILE_READ_ATTRIBUTES | SYNCHRONIZE` metadata handle checks the next component's canonical final path, complete
ID, volume, directory/reparse/offline state, NTFS volume, and actual parent case semantics before
the next guard is opened. Neither
`GetLongPathNameW` nor a handle-returned final path supplies authority or rewrites the configured
path. The entire chain and one publication-namespace guard remain owned through the actual SQLite
commit or rollback, including the second revision check. A renamed or replaced ancestor, renamed
old root, replacement at the configured path, missing component, short-path expansion failure, or
insufficient ancestor access therefore cannot inherit the old epoch or create a transient
authoritative delta.

The publication capability is represented by a private-field, non-cloneable
`PublicationGuardedFileDiscovery`. Its constructors are crate-private and perform the complete
namespace-chain bind; no `From`, `Default`, public field, or ordinary `FileDiscovery` conversion can
forge it. It has no `Deref`/`DerefMut` escape hatch and cannot return a raw `File`, `ReadDir`,
underlying `FileDiscovery`, or independently surviving raw iterator. Authoritative enumeration uses
an opaque cursor whose lifetime borrows the capability. Durable inventory uses a separate opaque
cursor that privately duplicates the root proof and every namespace-ancestor handle, owns the raw
directory cursor ahead of that guard in drop order, and exposes no `Clone`, `From`, or `into_inner`.
Authoritative catalog-delta execution is private to the application coordinator; its only sibling-
module entry requires a capability borrow separately from ordinary request context, and that borrow
spans construction, final revalidation, and repository commit or rollback. The expected v29 root
identity is checked through the pinned descendant metadata handle before the enumeration-root handle
opens, and the completed capability retains that enumeration handle and the full namespace chain
through commit or rollback. Metadata inventory has no public proofless entrypoint: production can
enter it only after loading matching unretired recovery authority and the applicable v29 or first-
baseline proof.

This adapter adds the following binding safety invariants:

- discovery root and directory handles request only metadata/list rights, use backup and no-follow
  flags, share delete/read/write, have one RAII owner, and remain live for every query;
- the configured-namespace volume handle opens the local DOS drive root with
  `FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`; descendant publication guards alone use
  pure `FILE_TRAVERSE` relative to their parent. Both use backup/no-follow flags and read/write
  sharing without delete sharing. Directory guards never request no-recall. A separate relative
  `FILE_READ_ATTRIBUTES | SYNCHRONIZE` metadata handle validates each component from its already
  fixed parent, and every guard remains live until catalog commit or rollback;
- the root-relative UTF-16 name buffer remains alive and immutable for the complete synchronous
  `NtCreateFile` call; `UNICODE_STRING.Length` and `MaximumLength` are checked byte counts without a
  layout cast, `OBJECT_ATTRIBUTES` is initialized with its exact generated type and size, and its
  `RootDirectory` handle outlives the call;
- only a nonnegative `NTSTATUS` creates a `std::fs::File`; failure is converted through
  `RtlNtStatusToDosError`, while a successful raw handle is transferred exactly once into RAII and
  never closed through a second owner;
- every native metadata open fixes `EaBuffer` to null and `EaLength` to zero and requests no
  content-read right; the terminal data call alone adds `FILE_READ_DATA`,
  `FILE_NON_DIRECTORY_FILE`, and native `FILE_OPEN_NO_RECALL`, omits delete sharing, and is rooted
  in the newly rebound no-delete configured-root guard; its successful raw handle has exactly one
  RAII owner, and complete 128-bit ID, volume, attributes, reparse state, and root containment are
  queried again before that handle can be returned for a read;
- the directory-information buffer has a fixed upper bound, remains live and writable for the
  synchronous call, and every returned offset, filename length, UTF-16 boundary, signed size and
  timestamp, and next-entry link is checked before access;
- the first call uses the restart information class and later calls continue the same handle-owned
  cursor; `ERROR_NO_MORE_FILES` is the only successful exhaustion signal;
- `.` and `..`, embedded separators, NULs, invalid UTF-16, duplicate relative paths, reparse
  directories, and any final-path/root-identity disagreement fail closed;
- Cloud Files state is derived only from enumerated attributes and reparse tag; enumeration never
  opens file content, follows a reparse directory, or hydrates a placeholder;
- the application, domain, SQLite, Flutter, and candidate publisher receive only Ame-owned
  metadata and identity values; no raw pointer, Win32 structure, or borrowed buffer escapes; and
- no SQLite transaction, priority permit, runtime mutex, or UI callback is held across a Windows
  directory call.

The minimal `unsafe` for the generated `NtCreateFile` FFI call, one-owner raw-handle transfer, and
parsing the variable-length directory buffer remains in the
local-filesystem adapter. It does not enter domain or application code and is replaceable by a
future safe handle-relative Windows API that preserves the exact pinned-root, no-follow,
no-recall, identity, and lifetime contract.

### Per-root checkpoints and shared volume reads

Every journal-capable root persists an independent checkpoint containing at least:

- root ID and current root generation;
- stable volume GUID and volume serial evidence;
- normalized root identity and root file reference;
- journal ID and next unread USN;
- the captured exclusive end boundary covered by durable enrollment;
- the associated catalog revision and checkpoint contract version;
- continuity state, last structured failure, and update time.

Unsigned journal identifiers, signed USNs, and V3 identifiers use lossless representations rather
than SQLite signed-integer assumptions. Schema v23 supersedes v22 with source-range lifecycle and
bounded pending-rename carry ownership. Schema v24 then adds a canonical complete batch payload,
64-character lowercase hexadecimal source-range identity enforcement, and exact journal,
OLD/NEW-USN, and previous-carry identity on carried lineage. Its atomic forward migration
deterministically rekeys only v23 ranges whose queue and carry content remains provable, updates
every owning foreign key and queue watermark, and rejects unprovable v23 lineage before mutation.
Fresh and current v24 triggers reject null, non-text, non-64-character, or non-lowercase-hex IDs on
both insert and update. A known prerelease-v24 trigger pair may be replaced transactionally with
those typed guards; a missing, mixed, or otherwise weakened definition remains a schema error.
Catalog open recomputes the payload digest and matches its immutable range prefix in addition to
comparing complete canonical DDL for every persistent-journal table rather than sampling SQL
fragments or counting constraints. DDL canonicalization normalizes only formatting outside quoted
tokens: single-quoted values, quoted identifiers, BLOB and GLOB operands, `RAISE` text, and escaped
quotes retain their exact bytes. Catalog open then validates exact columns, indexes, foreign keys,
marker, generation authority, bounds, relational ownership, and row values. The state is derived
coordination data but may be discarded only by an explicit migration or recovery transition that
preserves the last trustworthy catalog and schedules the required baseline.

Schema v25 adds one exact durable lane row for every retained queue row, guarded origin-to-lane
mapping, the recovery-authority allowlist bound to matching P2 queue work, and a persisted
opening/inventory/replay/absence/completed baseline lifecycle. Its immediate forward migration
classifies existing live notification rows as P0, journal catch-up rows as P1, and inventory,
consistency-audit, and explicit-refresh rows as P2 without granting implicit recovery authority.
Catalog open validates the complete tables, indexes, triggers, relationships, reason and phase
domains, immutable authority identity, and one-to-one queue ownership. A malformed v24 contract or
partial v25 contract fails before the migration commits.

Schema v26 adds a recovery-execution marker, one persistent owner for every P2 inventory candidate,
and the first metadata-inventory directory frontier. Queue publication, candidate ownership, and
page cursor advancement are atomic. A recovery can retire only after every owned candidate is
terminal through final filesystem revalidation or a proven higher-lane supersession; retry
exhaustion is not completion authority. The v25-to-v26 migration preserves terminal runs and admits
a pristine active run whose first page has not begun; an already-advanced active run without durable
frontier evidence fails closed instead of replaying the root from the beginning.

Schema v27 replaces source-directory rescanning at every output-page reopen with an
application-storage durable spool bound to the exact run, root generation, recovery authority,
scope, and opening directory identity. The source adapter consumes a live Windows directory
iterator in raw batches of at most 128 entries, checks cancellation between entries, and yields the
P2 root slot at every batch. Entries from an incomplete directory remain provisional and are reset
on process loss; they cannot publish candidates or absence. Only exhaustion plus closing directory-
type, no-follow/reparse, and matching Windows file-identity revalidation atomically marks that
directory complete and makes its entries available to the stable 4,095-entry output page. Completed
parent directories survive process reconstruction and are
not read again while child directories continue; no database, spool, cursor, or sidecar is written
inside the source tree. Completion, cancellation, supersession, and bounded cleanup remove the
derived spool under application storage.

One mid-directory process loss can therefore replay only the one incomplete directory and consumes
at most twice that directory's source entries. This is deliberately not a persistent Windows
directory cursor: repeatedly crashing at the same point can repeat that incomplete directory.
Because provisional entries never become authority and every completed directory is identity-
revalidated, that limitation can delay recovery but cannot produce false freshness or partial
absence. Catalog open validates the v27 marker, DDL, indexes, triggers, foreign keys, run/root/
authority binding, completed-directory state, candidate owners, frontier, checkpoints, and baseline
lifecycle as one contract; partial or contradictory projections fail closed.

Schema v28 makes the canonical root proof an immutable part of that spool. Each new spool stores the
ADR 0007 root identity scheme and complete volume-plus-128-bit value captured from the same pinned
root handle that owns enumeration. Candidate drain, absence publication, run completion, and
authority retirement must reopen the configured root, reproduce that proof, and fail closed before
publication if it differs. Stable output paging uses one indexed ordered `LIMIT page_size + 1`
probe: the sentinel establishes whether another page exists, so no output page performs a count or
anti-join over all unstaged spool rows. Cursor advancement, candidate ownership, and comparison
state remain transactional.

The v27-to-v28 migration first validates the complete v27 contract and never invents a root proof.
For every unretired recovery authority in opening, inventory, replay, or absence work, it
transactionally supersedes unresolved derived candidates, removes their owners and provisional
inventory/spool rows, resets the run and baseline to a recapture boundary, preserves the original
control and allowlisted recovery authority, leaves the last trustworthy catalog active, and
withholds checkpoint advancement, absence, and `Current`. Completed or retired v27 history remains
historical evidence only: its root and checkpoint are marked `RecoveryRequired` with the typed v28
recapture reason rather than being accepted as proof. Exact current-shape validation admits that
historical projection only with the complete invalidation marker and matching lifecycle evidence.
A partial v27 shape, mixed contract marker, or contradictory active projection rolls the migration
back.

Schema v29 promotes that proof into a root-generation publication contract rather than leaving it
inside P2. `library_root_publication_namespaces` contains at most one exact proof for the active
generation, and `library_scan_publication_namespace_bindings` records the proof captured by each
foreground scan. Foreground publication and metadata-inventory/P2 completion establish or verify
the proof in the same transaction that can publish `Current`; generation retirement removes it.
P0 and P1 must load the persisted proof and hold the same configured-namespace guard across their
content checks, revision retry, and final catalog delta transaction. The v28-to-v29 migration accepts
only mutually consistent full spool or V3 checkpoint identities, never upgrades V2 evidence, and
rolls back a partial schema or conflicting proofs. An active generation without trustworthy proof
keeps its catalog but becomes `RecoveryRequired` or `LiveOnly` until an authorized foreground or P2
workflow establishes one atomically.

Legacy v25/v26 catalogs may also reopen with up to 3,584 unresolved P2 rows that predate candidate
ownership. This debt does not change the 4,096 total limit or borrow either 512-row higher-lane
reserve. Path-scoped zero-owner rows drain in bounded recovery pages without fabricated ownership
or authority. One or more non-path zero-owner controls are transactionally coalesced into one
blocked root-scoped `FreshnessUnknown` control with combined evidence. Even a single legacy row
becomes the durable, exhausted `legacy_recovery_authority_missing` survivor on its first bounded
debt pass. Each superseded source retains its
bounded catch-up lineage and points to that survivor; terminal cleanup retains those rows while the
survivor is unresolved, so thousands of distinct watermarks are neither truncated nor forced
through the per-change lineage limit. The survivor remains non-publishable unless a pre-existing
matching unretired allowlisted authority exists; otherwise it cannot authorize enumeration, absence, a
checkpoint, or `Current`. One bounded debt page releases P1 admission, P0 can still preempt, and P2
then continues with per-root rotation. Crash/reopen repeats are idempotent and no unresolved row is
dropped or falsely completed. Polling an already blocked single survivor neither increments attempts
nor creates lineage, while explicit refresh or a legitimate matching authority can supersede it.

Roots on one volume share the physical journal query and bounded record read. Every successful
query result first participates in the minimum common exclusive boundary; only then are roots with
empty intervals removed from the read. A `NextUsn` increase observed by a later query therefore
cannot let that root alone advance beyond an earlier result. Every bounded page issues exactly one
physical `FSCTL_READ_USN_JOURNAL`; one native buffer may prove a partial covered boundary and the
next request resumes there. Roots whose next-unread USN is already at or beyond the common
exclusive end are successful no-ops and do not enter that read. When all roots are equal to the
end, startup issues no FSCTL and performs no enumeration or media read.

The broker distributes only root-contained normalized records. Per-root start USNs are passed to the
backend and applied before candidate or evidence budgeting; retained semantic events are ordered by
USN rather than request-root order. A root-specific live-handle, caller-access, containment, or
resolution failure remains that root's outcome; the broker selects the physical volume handle from
a surviving root, and only failure of the shared journal control call is volume-wide. A healthy
root may publish an unrelated proven interval when a sibling fails, but it may not advance across a
potential handoff whose complete evidence is unavailable. The barrier is the earliest OLD or NEW
USN at or after the uncertain root's own start. This same rule applies to native endpoint
resolution, service ACL/identity/containment translation, and pending-OLD revalidation. A known OLD
may be durably carried up to a later uncertain NEW; malformed owner-local evidence becomes that
root's outcome rather than a volume-wide error. When the page contains no rename, the healthy
sibling retains its full proven progress.

When source authorization and endpoint evidence are complete but target translation fails after
admission, the service replaces that handoff with a durable pending OLD bound to the same source
range, root generation, volume, journal, file reference, path, and OLD USN. It removes the target
candidate and original handoff and holds the proof boundary at NEW, so publication cannot lose the
OLD while still permitting the proven source prefix. If source translation fails or the OLD cannot
be constructed and authorized exactly, the boundary falls back to OLD and no carry is fabricated.
For multiple handoffs the earliest such safe boundary wins.

Records that prove a move between two configured roots carry one bounded cross-root lineage so
asset and compatible preview identity survive either processing order. Protocol v5 supersedes and
rejects v2, v3, and v4. It carries a bounded durable pending `RENAME_OLD_NAME` when the matching NEW
record is outside the current semantic page or native buffer; an intervening number of unrelated
records does not require rereading one fixed buffer. The codec, client, and session bind every
pending carry and completed handoff to the original request, exact file reference, root generation,
path, directory kind, and proven USN range. Same-page or carried cross-root publication places both
owners, source-range lifecycle, queue evidence, carry consumption, checkpoint compare-and-swap, and
root state in one `IMMEDIATE` volume-batch transaction. Half-lineage is rejected. An incomplete
participant preserves prior evidence and retries without introducing a wait cycle or letting one
root claim another root's coverage. A consumed carry maps to exactly two carried owners and binds
the previous source range, source and target root IDs and generations, volume, journal, file
reference, previous path, OLD/NEW USNs, and previous carry ID. Carry deletion, both owners, queue
evidence, and checkpoint advancement either commit together or all remain unchanged.

On every catalog open, each pending or completed cross-root lineage must have exactly two owners:
one `previous` and one `current`. Validation traverses every owner and checks role, root and
generation, source range, volume, journal, OLD/NEW coordinates, previous carry identity, and the
corresponding durable child in the canonical source-range payload. Missing or duplicate owners,
role substitution, or any owner/payload disagreement fails closed. The terminalizer repeats this
invariant inside its transaction before it can publish either participant as `Current`.

Every retained canonical source-range payload is also a complete child manifest. Catalog open
reconstructs its pending-carry and lineage sets and proves both directions against durable rows. A
pending carry must either remain as one exact pending row or have one exact consumed-lineage proof
through `previous_carry_id`; every payload lineage must retain its parent and exactly two matching
owners. Conversely, every durable carry, consumed proof, lineage, and owner must occur in the owning
payload. Absence, duplication, extra children, or a payload-only child fails closed rather than
being interpreted as completion. Cleanup may remove this evidence only by deleting the complete
terminal lineage/range proof cluster in the same bounded transaction.

The canonical payload also owns the complete normalized intent set. Catalog open and the
production terminalizer use the same coalescing normalization as publication, then require exact
bidirectional equivalence with retained queue rows, persistent source-range ownership, and any
cross-root peer evidence. Root, generation, kind, scope, paths, origin, observation times,
sequences, and coalesced count all match exactly. Peer evidence is accepted only when its lineage,
endpoint enrollment, and, for consumed-carry recovery, previous root, generation, path, NEW USN,
volume, journal, and covered recovery range prove its provenance. Queue rows that are terminal remain
durable until the complete closed range cluster is deleted; neither queue evidence nor ownership may
disappear independently while its payload remains.

Durable lineage state is derived from endpoint proof. `Completed` is valid if and only if both exact
owner range lifecycles are completed. `Superseded` is separate: it requires a superseded owner range,
an inactive or replaced root generation, and matching durable retirement authority; it cannot grant
`Current` or absence authority. Otherwise the only valid state is `Pending`. Catalog open and the
terminalizer enforce the same transition predicate inside their transaction.

Volume-wide reads remain bounded even when every retained record is unrelated to the requested
root. A semantic record or evidence limit ends the response immediately before the first complete
event that would exceed it, returns a strictly advancing `covered_until_usn`, and reports the page
incomplete. OLD/NEW pairs are never split by a response budget; an oversized first event fails only
its affected root rather than livelocking the volume. The next request resumes from the exact proof
boundary, so no record is skipped and repeated bounded pages eventually cover the captured end.
Root-external records influence only the coverage boundary; their names and paths never enter
root-scoped candidates or response evidence.

For each root and bounded range:

1. capture an exclusive journal end boundary;
2. validate exact root and generation, volume GUID and serial, journal, checkpoint, protocol, and
   contract identity;
3. read and normalize only records in `[checkpoint, boundary)`;
4. prepare bounded Ame intents and any cross-root handoff lineage;
5. publish every successful page from the same shared response as one atomic volume batch, including
   queue ownership, range lifecycle, pending carry mutation, lineage owners, checkpoint compare-and-
   swap, and root state;
6. retain `CatchingUp` after raw journal coverage; and
7. let the ordinary final-state reconciler decide current file state and, in its terminal
   transaction, publish `Current` only when all required queue, range, and lineage evidence is
   terminal.

A complete empty range with no queue, pending carry, or unresolved lineage reaches terminal range
lifecycle in its publication transaction and may publish `Current` when it covers the captured
boundary. A partial empty range terminalizes only that range and leaves the root `CatchingUp` at the
next strictly advancing boundary. Migration backfill applies the same durable queue/carry/lineage
predicate rather than marking every historical range pending or terminal indiscriminately.

A crash before publication leaves every old checkpoint and carry unchanged. A crash after
publication may replay only the same complete normalized batch digest; changing an intent, timing
evidence, pending carry, lineage endpoint, owner, or handoff conflicts without moving a checkpoint.
Range and handoff IDs include complete volume, journal, range, root, and participant-generation
identity, so a replacement generation cannot collide at reused USN coordinates. Checkpoint
advancement uses the same `IMMEDIATE` volume transaction and requires current next-unread USN equal
to the requested start, exact continuous durable range evidence, monotonic covered/end/revision/time
values, and full-row compare-and-swap of the range, checkpoint, and root state. It never establishes
an implicit baseline and no crash point may advance past evidence that was not durably enrolled.
Terminal cleanup retains unresolved carries, active authority, current checkpoints, cross-root
owners, and the newest necessary historical proof; active generations retain 64 completed ranges.
Older terminal history is deleted in bounded batches only when every connected completed lineage
and all of its owner ranges form a closed deletion set in the same transaction.

### Normal startup

For each configured root, startup is:

1. show the last trustworthy catalog immediately;
2. validate root availability and capability with metadata only;
3. start the live watcher before opening a downtime boundary;
4. obtain the current journal identity and exclusive `NextUsn` through the broker;
5. when the persisted checkpoint is continuous, enqueue only records between the checkpoint and
   that boundary as P1 work;
6. process P0 work immediately even while P1 remains; and
7. mark continuity current only after the root checkpoint covers the boundary and its required
   queue lineage is terminal.

When `checkpoint == NextUsn`, startup performs zero source-root enumeration and opens no media. It
must not create a metadata-inventory run merely to reaffirm an empty journal range. Cached gallery
use, live change publication, and preview demand are not gated on the root already being current.

### Running-time changes

Watcher callbacks remain non-blocking hints and enter P0 through the existing retained-plan handoff.
The P0 reconciler inspects only the affected final paths or bounded subtree, publishes one guarded
catalog delta, and lets Flutter refresh from the resulting revision. A newly discovered image may
appear before its preview is ready; preview generation remains bounded derived work and cannot gate
the catalog location.

Journal replay overlaps safely with watcher evidence. The watcher supplies latency; the journal
supplies persistent continuity. Duplicate observations coalesce by root generation, affected path,
file identity where available, source range, and final-state evidence. A later journal observation
cannot overwrite a newer final state merely because its USN is higher. The application may perform
bounded journal advancement while running to close watcher gaps, but it never substitutes a
periodic directory walk.

A watcher, observer-ingress, or availability transition that loses exact evidence remains a durable
`LiveNotification` P0 root `EvidenceGap`. It must not be rewritten as a P1 `StartupCatchUp` root
freshness marker, because that would discard its reserved live lane and leave no production owner.
The P0 row remains retryable until one of the two recovery transactions below has persisted a
specific consumer for the same root generation and lineage.

### Shutdown and closed-process changes

Shutdown hides the window immediately, stops accepting new watcher callbacks, returns or persists
leased work safely, cancels broker and recovery I/O at bounded boundaries, and retains the last
durable per-root checkpoint. It does not need to keep the service or another Ame process running.
Foreground first-import or explicit-refresh full scans retain their separately accepted checkpoints.

Changes made while Ame is closed remain in the NTFS journal. The next process starts the watcher,
replays only the missing journal range through P1, and processes the changed final paths. Cost is
bounded by retained journal records and affected paths rather than by every entry in the library.

### Recovery and one-time baseline

P2 metadata inventory is authorized only when:

- an existing root is migrated to this contract without a trustworthy checkpoint;
- a root completes its first import and needs a journal boundary bound to that published catalog;
- the journal ID changes, the checkpoint is before `FirstUsn`, or a requested range is deleted;
- volume, root, generation, protocol, record-version, reconstruction, containment, or checkpoint
  evidence cannot be proved;
- the broker becomes unavailable after the root previously claimed persistent continuity;
- the watcher loses evidence while no continuous journal range can cover it.

First import, explicit `更新图库`, and resumption of their exact foreground checkpoints remain the
only full media-scan authorities. They are not automatic P2 work. Ordinary startup, elapsed time, an
empty journal range, queue size, slow media, and a healthy watcher do not authorize inventory.

When a continuous journal range can cover a live gap, one SQLite transaction first persists the P1
source-range ownership and exact P0 lineage, then supersedes the P0 row. Publishing the covering
range may complete that transfer in the same transaction; a claimed but not yet published range
keeps the P0 retryable and unavailable to an unrelated consumer. The implementation never deletes
the gap merely because a broker or journal is expected to cover it.

When bounded P0 reconciliation proves that watcher evidence cannot be reconstructed inside its
declared scope and no continuous journal range can cover it, one SQLite transaction creates an
independent P2 control row and matching allowlisted authority, transfers the durable lineage, and
only then supersedes the P0 row. The P2 row starts pending with no P0 lease or worker ownership. If
lower-lane admission is unavailable, that transaction creates nothing and the original P0 evidence
remains retryable; queue size or slow work alone never performs this promotion.

For a baseline or continuity gap, production starts or keeps the watcher first, captures a journal
boundary, performs one metadata-only inventory, and then replays journal changes that occurred
during the inventory. P0 remains reserved and publishes throughout recovery. Positive candidates
may publish after final-state revalidation; removals require complete scope authority plus the
closing journal boundary. A cancelled, partial, failed, raced, or unavailable inventory preserves
the last trustworthy catalog and cannot publish mass absence. Successful completion atomically
establishes the new checkpoint and retires the recovery authority.

Existing catalogs receive this baseline once during migration because no new implementation can
prove changes before its first checkpoint. This cost is explicit migration work, not a startup
policy. First import may establish its initial checkpoint from the scan boundary and the watcher/
journal overlap without adding another complete pass when its authority is provable.

If a root is permanently outside the supported journal boundary, it becomes `LiveOnly`; automatic
recovery does not repeat an O(N) inventory on every launch. A failed P2 path becomes
`RecoveryRequired` or blocked with one structured issue and an explicit user decision. It never
starts a hidden full media scan.

### Product state

Live delivery and historical continuity are independent internal dimensions:

- live state: `Healthy`, `Degraded`, or `Stopped`;
- continuity state: `BaselineRequired`, `CatchingUp`, `Current`, `RecoveryRequired`, `LiveOnly`, or
  `Unavailable`.

The existing compact Chinese product states remain `已同步`, `正在更新图库`, `更新受阻`, and
`目录不可用`. A healthy root may publish P0 work while `CatchingUp` or `RecoveryRequired` maps to
`正在更新图库`. `已同步` requires a checkpoint covering the declared boundary. A failed broker,
exhausted durable recovery, or incompatible protocol maps to `更新受阻` only when no healthy
automatic path remains. `LiveOnly` must be explained as a capability limitation and cannot imply
closed-process coverage.

Normal update, catch-up, retry, and recovery success remain silent. One root-keyed notification may
explain a blocked or capability condition. Presentation never exposes raw service, journal, USN,
queue, file-reference, or privilege vocabulary in its compact status.

### Persistence migration

The forward migration preserves catalog roots, assets, locations, source-state evidence, previews,
user decisions, operation history, foreground scan checkpoints, terminal-media evidence, and
unresolved trustworthy change work. Historical schema v19 journal and handoff objects are migration
input; they are not accepted as current checkpoints without exact contract validation. Active ADR
0023 startup inventories are terminalized or converted into the one-time baseline authority without
publishing their partial absence evidence. Historical `authoritative_recovery` scan checkpoints
remain retired and cannot return to the full scanner.

Before persistent-journal contract validation, an upgrade from a catalog that predates recovery-
authority ownership normalizes metadata-inventory lifecycle state transactionally. Every unfinished
`running` or `comparing` run becomes `superseded`, and every non-`completed` run loses
`absence_authority`; completed authority, inventory entries, issue evidence, roots, assets,
locations, and queued work remain intact. Runtime terminalization and newer-epoch supersession must
revoke the same authority in the transaction that changes status. The journal validator remains
fail closed after this bounded normalization; migration must not manufacture recovery authority or
enumerate a source root.

The migration adds versioned broker capability, per-root journal checkpoint, covered-range, lane,
and recovery-baseline authority. It may rebuild derived queue indexes or origins transactionally,
but it must preserve row identity, retry, lease generation, root generation, catalog revision,
handoff ownership, and stale-publication guards. A partial contract or malformed named object fails
closed. No migration touches a source root.

Schema v30 adds durable live-gap recovery claims that bind one exact P0 row and lineage to one of:
a pending journal boundary, a persisted journal source range, an independent P2 metadata-inventory
control, or an explicit-recovery-required state. During an explicitly requested foreground library
update, only an explicit-recovery claim may transition to the typed `foreground_scan` consumer. The
same transaction binds its scan ID, root, and generation. Generic scan high-watermark, leasing,
completion, and abandonment exclude every claim-owned row, including pending-journal claims.

The v29-to-v30 migration recognizes the historical naked shape
`P1 + Root + FreshnessUnknown + StartupCatchUp` as ambiguous. A root-generation or publication-
namespace identity proves which root may publish; it does not prove which filesystem event created
the row or that live P0 provenance existed. V29 retains no phase-25-specific evidence that
distinguishes this row from the earlier legal fallback shape, so every such naked row remains in
place and receives explicit-recovery ownership. The migration preserves its root identity, retry
evidence, and catalog row; it never deletes the row, fabricates a journal range or recovery
authority, relabels it as proven live P0, or touches source media.

Foreground publication atomically publishes the catalog revision, completes the owned gap, and
records the consumed claim plus scan linkage. Abandonment and current-schema crash recovery
atomically restore the original explicit consumer as `retry_wait` with no retry deadline and the
typed explicit-recovery failure. The validator admits only the exact explicit, active foreground,
and consumed foreground relations; partial or conflicting states fail closed before recovery. V30
has not shipped as a catalog version, so this correction evolves the current v30 DDL and v29
migration in place rather than creating a meaningless v31. Fresh catalogs and v29 upgrades remain
supported; partial prerelease v30 shapes remain conflicts and are not silently repaired.

### Installer and portable distribution

ADR 0015's portable ZIP remains a valid application distribution, but it cannot install or update a
Windows service or establish the protected installed-client identity. It therefore always runs in
explicitly presented `LiveOnly` mode, even when a broker is already installed. It must not claim
closed-process continuity or treat a valid signature in a user-writable portable directory as a
production code-identity boundary.

Broker activation is admitted before constructing or connecting the production broker factory and
before any Service Control Manager or named-pipe operation. The client obtains its own image path
from the current-process handle and compares it with the fixed native
`FOLDERID_ProgramFilesX64\Cedarflake Ame\Application\cedarflake_ame.exe` identity; it never derives
that boundary from an environment variable or launch argument. Any mismatch or identity-query
failure returns the explicit portable `LiveOnly` reason with zero broker-factory connections. The
Windows transport repeats the same fail-closed check before demand-starting the service, so a
portable process cannot activate or contact an otherwise valid installed broker through a direct
transport call.

The complete Windows 11 x64 synchronization product uses a signed installer that installs,
repairs, updates, and removes the broker with explicit administrative consent. Installer work is an
R2c dependency rather than deferred wholesale to R9. General automatic update, downgrade, catalog
rollback, and broader supply-chain policy may remain later work, but broker and desktop protocol
compatibility, service lifecycle, safe upgrade, and uninstall behavior are required before this
decision can ship.

Uninstall removes the service, broker binary, protected Application bundle, and transaction-owned
install directories without deleting source media. Catalog and durable user-data retention follow
an explicit installer choice; cache cleanup cannot erase those classes. A service protocol mismatch
fails closed and leaves the cached catalog usable. The desktop never silently installs, replaces,
disables, or deletes the service.

The lifecycle foundation fixes the service binary and installed Application bundle under the native
`FOLDERID_ProgramFilesX64`, requires Windows 11 client x64 and a native 64-bit installer process,
and never derives authority from the `ProgramFiles` environment value. It validates the PE machine
and exact Authenticode publisher before every install/repair/upgrade, uses a same-volume staged
replacement with rollback, and verifies the SCM registry contract plus service and binary ACLs
after mutation. Broker validation executes `--protocol-info` and accepts exactly one versioned
protocol/max-frame record; a signed but incompatible executable is rejected. Upgrade additionally
requires the old binary's exact ACL, publisher, protocol, hash, signer, installed-client binding,
and protected SCM manifest to agree before mutation. Install and upgrade maintain an explicit
per-invocation ownership journal. Rollback runs every owned action independently in reverse order
and aggregates failures; it never deletes a pre-existing file or service, retains backup ownership
after a failed deletion, stops a newly started service before restoration, and treats a
missing-binary repair as having no backup. Service removal releases controller handles and waits a
bounded interval for the deleted SCM object to disappear before the fixed binary tree is removed.
The ownership journal is a protected, bounded, write-through transaction marker written before the
first owned mutation. Every lifecycle entry recovers a dead or PID-reused owner before proceeding,
preserves and restores the pre-existing Ame parent descriptor, and deletes owned trees only through a
bounded no-follow walker. A markerless broker tree is adopted only when it is empty, exactly
protected, and has neither a service nor binary; other orphans fail closed.
Uninstall records its planned transaction-created directories before the first protected-tree or ACL
mutation, then uses write-ahead pending and completed states for protected-tree preparation, service,
binary, Application bundle, install-tree and ancestor removal, and parent-descriptor restoration.
`committed` records that the core service and payload are absent; it is not permission to delete the
marker. The marker retains the exact pending uninstall operation and a strict prefix of completion
flags. Recovery rejects any phase, pending-operation, or flag contradiction. It re-verifies each
completed operation unless that operation's postcondition is provably superseded by the persisted
pending or completed state of a specific later operation. In particular, install-tree deletion may
supersede protected-tree readiness, while service, binary, Application, ancestor, and parent
postconditions remain independently verified. Recovery retains a fail-closed recoverable marker
until final absence checks and the exact parent prestate have both succeeded. Only a separate
`final_cleanup_verified` transition permits marker deletion.
Every existing path ancestor must match its final handle path, be non-reparse, have
a trusted owner, and grant no applicable untrusted replacement, directory-write, or parent
`FILE_DELETE_CHILD` right. Missing Ame directories are created atomically with a protected
SYSTEM/Administrators descriptor; final directory, binary, and service descriptors are matched as
exact rule sets. The Application bundle is copied only into the fixed physical Application tree;
its complete bounded, non-reparse tree is reopened and every entry must resolve inside that tree.
Its protected ACL grants SYSTEM and Administrators full control and BUILTIN Users only read and
execute. The broker tree remains separately protected for SYSTEM, Administrators, and the restricted
service SID. Only after both trees pass exact ACL and final-path verification does the installer
write the `AMEJBID2` manifest to the protected SCM object. The portable archive includes the broker payload for
installer compatibility but has no SCM operation and does not activate it. The current repository
has no production signing credential or signed candidate; release-candidate scripts therefore
require an immutable signed Application bundle, its exact signed broker path, and the expected
publisher and fail closed when any input is absent. Hosted release automation accepts production
PFX material only through repository secrets, builds and signs once, uploads that exact bundle as a
short-lived artifact, and makes packaging and post-publication verification revalidate the same
application/broker signatures and broker protocol without rebuilding.

The installed-broker acceptance harness is intentionally separate from release signing. It requires
an exact authorization token, an already elevated shell, explicit nonexistent fixture children
under one pre-created empty temporary workspace, an absent fixed service, and an externally
pre-signed V1/V2 acceptance-bundle pair. Both bundles have fixed `Application` and `Broker`
payloads, one exact publisher and compatible protocol identity, no reparse points, trusted
ownership, and no untrusted mutation right on any ancestor or payload. Their broker SHA-256 values
must differ, while the broker-only upgrade requires the installed client identity to remain exact.
The repository runner cannot create a signing certificate, change
a trust store, access a private key, sign a staging copy, or turn a user-writable staging directory
into elevated installation input. If a dedicated VM needs a temporary identity, its protected
offline preparation and destruction are separate prerequisites and never part of this runner.
After the elevated lifecycle step, the harness creates one exact Task Scheduler entry
for the already logged-on user with `InteractiveToken` and `RunLevel Limited`; the Rust acceptance
client independently rejects an elevated token. The parent owns a one-instance named pipe with a
protected SYSTEM/Administrators/current-user DACL and accepts only a bounded result record carrying
the parent-generated 256-bit nonce, phase, task instance, OS-observed client PID, fixed protected
client path/hash/signer/protocol, token elevation, process creation time, and current Task Scheduler
`LastTaskResult`. A user-writable result file or client-reported PID cannot produce acceptance. It
pins the physical workspace ancestor chain against rename and grants its limited user only
read/execute access. It snapshots disposable root and root-external fixtures before the broker runs,
verifies content and metadata afterward, and executes every cleanup through no-follow object handles
plus every absence confirmation independently for the
scheduled task, service, fixed installed Application bundle, fixture directories, and workspace even
when an acceptance assertion fails. It restores the exact workspace descriptor in `finally` and
never mutates or removes either external pre-signed bundle.
Before elevation-bound work, the harness requires the exactly counted deterministic broker matrix
and installer rollback guardrail. The elevated lifecycle additionally exercises missing-binary
repair of V1 and a distinct-hash same-publisher upgrade to V2 before its restart phase.

## Validation gates

### Decision and security feasibility

- a minimal Windows 11 x64 PoC proves that an installed broker can query and read the existing
  journals for disposable local NTFS roots while the Ame desktop process remains unprivileged and
  receives no per-start UAC;
- an independent security review verifies service account, required privileges, service SID,
  binary ACL, pipe DACL, caller-token validation, root authorization, impersonation, and no-network
  behavior before product data flows through the broker;
- adversarial requests for another user, session, volume, root, file ID, path escape, malformed
  frame, oversized frame, stale generation, and incompatible protocol fail without record or path
  disclosure;
- the PoC confirms that no broker operation reads media content, writes source data, changes the
  journal, or hydrates a placeholder.

### Correctness and migration

- deterministic parser and IPC fixtures cover V2/V3 records, mixed and unknown versions, malformed
  lengths and offsets, invalid UTF-16, cancellation, journal recreation, trimmed ranges, and bounded
  memory;
- application fixtures prove watcher-first capture, per-root enqueue-before-checkpoint, replay after
  every injected crash boundary, root-generation invalidation, same-volume multi-root sharing,
  independent root progress, and cross-root move handoff in either order;
- migration fixtures preserve every durable data class, terminalize partial ADR 0023 inventory
  authority safely, establish exactly one baseline where required, and reject partial schemas;
- create, modify, rename, move, same-path replacement, and delete while Ame is closed converge from
  journal records through final-state reconciliation without source-root enumeration;
- journal reset, deletion, trimming, broker interruption, root replacement, and reconstruction
  ambiguity enter P2 without false freshness or partial removal;
- OneDrive fixtures prove metadata-only journal, final-state, and recovery handling does not hydrate
  placeholders.

### Priority and user experience

- a no-change startup across the retained approximately 79,000-location catalog performs exactly
  zero source-root entry enumeration, opens zero media files, and creates no inventory run;
- a controlled running-time change reaches the visible catalog at P95 no greater than one second;
- the same P95 bound holds while P1 contains a large backlog and while P2 enumerates a target-scale
  recovery root;
- a closed-process single-file create becomes visible at P95 no greater than two seconds after the
  normal application runtime is ready, without O(N) root enumeration;
- one recovering or unavailable root cannot delay P0 or continuous P1 work for another root;
- an event storm, million-record unrelated journal interval, retry, cancellation, and repeated
  startup remain bounded in queue, memory, SQLite transaction time, service response, and shutdown;
- cached gallery content is usable immediately and preview readiness never gates a new location.

### Release and safety

- focused tests, migrations, format, Clippy with warnings denied, complete Rust and Flutter tests,
  generated bridge verification, Daily, Windows Release, installer, service lifecycle, and package
  upgrade/uninstall gates pass serially on the workstation;
- a final independent code, architecture, security, migration, and source-safety audit reports no
  unresolved findings before R2c closeout;
- retained real-library validation requires separate current authorization and isolated derived
  storage; ordinary unattended gates never access those roots;
- every controlled run verifies source bytes and entries unchanged, no placeholder hydration, and
  no journal configuration change.

## Implementation checkpoint

The 2026-08-30 working tree has an R2c-Q remediation checkpoint behind schema v29. Production starts
live observation before its retained broker session, services P0 before each lower-lane page, and
keeps a dedicated bounded P0 worker separate from P2 recovery. Each production runtime epoch first
creates one validated SQLite session: schema migration and complete cross-table validation run once,
outside the runtime mutex and every lane-priority permit. Later poll and worker connections perform
only constant-size checks for the canonical catalog path, Windows file identity, WAL mode,
application ID, user version, and schema cookie. A path replacement or schema-cookie change makes
the session stale and forces a fresh complete validation before use; ordinary data-version changes
do not. The lane-aware admission permit begins at `BEGIN IMMEDIATE` and ends at commit or rollback,
so it never covers connection open, PRAGMA configuration, migration, validation, filesystem, broker,
hash, signature, or pipe I/O. The runtime registry uses short epoch and ownership transitions, so
stop can cancel and join while those slow operations are blocked and late results cannot install
into a replacement epoch.

P1 requests at most 64 records per broker page, yields at that boundary, checks cancellation between
roots and broker operations, and persists reset, trim, reconstruction, containment, and
broker-after-current failures as typed per-root recovery transitions without checkpoint advancement.
P2 readiness and lease selection require a matching persisted unretired authority. Its production
candidate drain is bounded, candidate ownership survives crash/backpressure/coalescing, and the v28
durable spool consumes live source iterators in 128-entry batches before exposing stable 4,095-entry
output pages. Completed directories are not re-read after a process reconstruction, while the one
incomplete directory is explicitly reset and recaptured. P2 roots rotate after each raw batch, not
only after a full output page. Ordinary startup, time, empty ranges, queue pressure, slow work,
`LiveOnly`, and consistency-audit rows cannot create or lease P2 authority.

The v28 root identity stored with the spool is reproduced before owned-candidate drain, every
absence page, completion, and final retirement. Schema v29 additionally binds every active root
generation to one persisted publication identity, establishes foreground-scan and P2 proof in the
same transaction as freshness, and retires proof with the generation. P0 live and P1 journal delta,
paired rename, subtree/root reconcile, absence, and finalizer paths all require that proof; no path
string or current replacement identity can synthesize old authority.

The fifth R2c-Q remediation moves that rule ahead of the production enumeration boundary. P0 root,
P0 subtree, and bounded authoritative P2 obtain the active-generation proof and one full
configured-namespace guard before status checks, enumeration, or delta construction. The same
non-forgeable `PublicationGuardedFileDiscovery` capability is borrowed by the authoritative
request and remains live through the real
catalog-delta transaction. A root-ID-keyed, one-shot test hook placed immediately before SQLite
commit proves the ownership window without creating a second production path: each production poll
enumerates real controlled source state, receives Win32 32 when it attempts a root or ancestor
rename, rolls the transaction back to a structured retry, and permits the rename only after worker
shutdown releases the guard. Catalog revision, completion/current state, publication proof, and P2
authority retirement remain unchanged.

Windows enumeration walks components relative to a live pinned canonical-root handle and never
follows a junction. Every component obeys its live parent's case-sensitivity flag, so case-sensitive
siblings are never collapsed by an unconditional `OBJ_CASE_INSENSITIVE` or lowercase containment
check. Media inspection rebinds the configured root with a no-delete handle, then performs a second
root-relative terminal-file `NtCreateFile` with native no-recall and no delete sharing. It
revalidates the metadata handle's complete 128-bit identity, attributes, volume, and live root-
identity containment before reading; raw volume/device namespaces never reach an OS open. Every
publication boundary holds the full configured namespace chain from the local DOS volume root
through every ancestor and the root. The volume handle uses
`FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`; root-relative descendant guards alone use
pure `FILE_TRAVERSE`. Both use backup/open-reparse flags and no delete sharing; directory guards do
not request no-recall. The configured long-DOS path supplies normalized descendant names rather
than the volume-handle path. A separate relative `FILE_READ_ATTRIBUTES | SYNCHRONIZE` metadata
handle validates each component from its already fixed parent. The chain remains held
through the real SQLite commit or rollback.
An ancestor junction swap, moved-out object, offline object, renamed configured root, replacement
root, missing root, or root-proof mismatch remains retryable and cannot publish absence or
`Current`. The v27-to-v28 migration discards unproven derived active work and schedules bounded
recapture under its existing authority instead of fabricating proof.

The same proof-first boundary now owns first-open durable metadata inventory. When an active proof
exists, source construction receives that exact proof and root identity, validates the complete
namespace guard before any discovery or metadata inspection, and keeps it through spool-init commit
or rollback. A mismatched or replaced root/ancestor therefore creates no spool, candidate, absence,
catalog, revision, `Current`, or authority change. The legitimate first baseline with no publication
proof first establishes and holds the full guard, then persists the proven spool identity in the
same transaction; a path string alone cannot create that authority.

The default queue ceiling remains 4,096 unresolved rows, divided into a steady P2 cap of 3,072,
512 rows reserved for P1, and 512 rows reserved for P0. P1 checks for ready or active P0 work before
opening another page; P2 checks both P0 and P1. A bounded page already in progress may finish, but a
lower lane does not unconditionally open the next one. A migrated v26 catalog may temporarily carry
the former 3,584-row P2 occupancy as capacity debt. Rows without a v26 candidate owner use a
separate bounded legacy-debt path: path rows are resolved without inventing an owner, while repeated
non-path controls compact to one blocked root control with retained lineage and no new recovery
authority. While P1 waits, production permits exactly one bounded P2 debt page to release capacity,
prohibits new P2 enumeration and refill, then admits P1 before P2 resumes. P0 retains its reserve,
and the same page-boundary cursor preserves fairness among roots.

Legacy readiness selects zero-owner non-path work only when an unblocked survivor exists or more
than one row still requires compaction. The final survivor, including an input containing exactly
one row, runs once and becomes a durable exhausted `legacy_recovery_authority_missing` control with
no invented authority, enumeration, absence, checkpoint, or `Current`; that blocked row cannot
repeatedly start a worker after reopen. Runtime and Flutter project it as blocked
`RecoveryRequired` with an explicit update-library action. A later explicit refresh or legitimate
allowlisted authority can supersede the survivor, retaining bounded lineage while providing a real
recovery path. The 3,584-row migration debt, lane reserves, and root-fairness contracts remain
unchanged.

The baseline and watcher-gap paths persist full opening root/volume/journal identity, opening
`NextUsn`, metadata-only inventory, closing `NextUsn`, and the exact closing P1 replay window.
Absence, the replacement checkpoint, `Current`, and authority retirement share the final barrier
with terminal candidate ownership and replay evidence. Cancellation, crash, retry exhaustion,
identity drift, or non-terminal replay retains `RecoveryRequired` or `CatchingUp` and the last
trustworthy catalog. Catalog reopen validates these cross-table lifecycle relationships rather than
accepting a plausible individual row.

Public stop now has one success condition: the current epoch has cancelled and joined its P0, P1,
and P2 workers, stopped the core observer, closed the retained journal session exactly once, and
removed the registry entry. It waits on bounded channel/condition-variable completion outside the
registry mutex. The first public call creates one absolute two-second deadline and stores it with the
epoch in registry-owned `Stopping` or `Draining` state. Concurrent stops reuse that exact value;
neither a late start/poll owner nor a retry can replace it with `Instant::now() + 2s`. A runtime that
returns after expiry is first installed as complete `Draining` ownership, including P0/P1/P2,
observer, journal session, close receiver, join handle, and cached outcome, and only then attempts
cleanup with the exhausted deadline. No root or phase receives a fresh allowance. Journal close is
one retained owned worker with its exact receiver and join handle. A timeout keeps that same close
task and session in `Draining`; retry or the reaper can only join the original task, never call close
again, refresh the deadline, clear the registry, or start a replacement epoch first. Panic and
disconnect remain terminal fail-closed outcomes of that same task.

Flutter invalidates the current poll generation and calls native stop as soon as stop begins; it
does not await an active poll. A never-completing poll therefore cannot make the public call exceed
its absolute deadline. Ready and Polling entries retain the complete runtime in one registry-owned
`Arc`; stop immediately changes either entry to `Draining` with that same `Arc` and the first
deadline, even while a poll still holds the runtime operation mutex. When that poll returns, its
result is discarded and cleanup continues on the already retained owner under the unchanged,
possibly exhausted deadline. Concurrent stop callers share only the in-flight stop future,
settlement clears it, failure can retry the same native cleanup, success permits immediate restart,
and dispose permanently blocks new starts while sharing the active stop boundary.

The registry boundary also contains constructor, poll/operation, and drain panics without relying on
the desktop bridge's outer unwind handler. A constructor panic removes and notifies only its matching
runtime-less `Starting` or `Stopping` epoch. An operation panic moves the same complete runtime into
`Draining`; it reuses an existing public deadline or creates one fault-stop deadline exactly once.
The runtime mutex guard is acquired outside the drain `catch_unwind`, so a cleanup panic does not
poison the mutex. After acquiring that mutex, a drainer rechecks the exact epoch, `Arc`, and deadline
before doing any work, preventing a stale caller from cleaning an empty or replacement epoch. P0,
P1, P2, observer, journal session, close receiver, cached result, and every join handle remain in
their owner slots until each idempotent step completes. A panic therefore preserves the same
`Draining` owner for retry; a persistent panic stays fail closed without refreshing the deadline,
re-requesting stop, re-closing or detaching work, publishing `Empty`, or admitting an ABA epoch. All
contained owner panics return the stable `library_synchronization_owner_panicked` error without
copying the panic payload.

The ninth R2c-Q remediation closes the remaining startup and panic-reconciliation races. Flutter
start now returns an explicit `started`, `failed`, or `skipped` result and owns one generation-aware
in-flight operation. Recoverable native failures retry the real start call through a finite 100 ms,
500 ms, and 2 s backoff; polling begins only after a current-generation start succeeds. Stop and
dispose cancel that retry and conservatively invoke native stop whenever native ownership may have
been entered, even if logical start is false. Native not-started is an idempotent convergence fact;
other stop failures do not clear ownership. A late start result cannot publish or create a timer in
a replacement generation.

The native registry now records the greatest proven retired epoch and models panic reconciliation
as either retention of the same runtime/deadline or proof that the old epoch already drained. A
test-only registry-local barrier forces the exact sequence in which an operation panic drops its
runtime guard, concurrent stop closes and retires that epoch, a replacement epoch enters Polling,
and only then the old caller resumes. That caller returns the sanitized stable panic error without
touching the replacement runtime. An active same-epoch mismatch, poisoned registry, or lock failure
still fails closed; runtime-less `Stopping` cannot adopt an external poll runtime.

The tenth R2c-Q remediation moves lifecycle admission ahead of the asynchronous desktop bridge.
Two short synchronous bridge calls allocate checked, process-global, strictly increasing lifecycle
tickets under the registry mutex. A start request reserves one owner ticket before submitting its
asynchronous task; all retries for that request reuse the ticket. A public stop synchronously
reserves a greater cancellation fence, advances the registry's cancel-through watermark even when
the registry is `Empty`, and records the first absolute two-second deadline before the asynchronous
stop task can be delayed. In the same short critical section it changes every matching `Starting`
owner to `Stopping` and every matching `Ready` or `Polling` owner to `Draining`, retaining the same
runtime `Arc`, cancellation flag, owner ticket, fence, and deadline. The synchronous admission path
does no construction, I/O, join, or runtime-mutex acquisition.

Start rejects every ticket at or below the cancel-through watermark before epoch allocation or
construction. A newer ticket may enter a new epoch only after the retained owner drains. Poll
requires the exact active owner ticket, so an old poll cannot attach to a replacement epoch. A
same-ticket bounded start retry may reuse a ready owner or construct a later epoch after contained
constructor recovery; a different ticket cannot adopt that owner. Ticket or epoch overflow,
unissued tokens, registry poison, runtime poison, and impossible same-owner state remain structured
fail-closed outcomes. The asynchronous stop accepts only a previously admitted fence and can drain
only the owner covered by that fence; it reuses the synchronously recorded deadline and never
creates a new allowance when bridge scheduling is late.

Desktop bootstrap now renders the trusted cached catalog and initial state before synchronization
startup. A process-lifetime owner stores the handled start future, attaches synchronous and
asynchronous error handling before scheduling background start, and shares shutdown disposal.
Shutdown calls dispose and therefore the synchronous cancellation fence immediately without waiting
for a hung start. Only `catalog_database_busy` and `catalog_database_locked` use the finite 100 ms,
500 ms, and 2 s retry schedule; every other start failure is attempted once. Generation and owner
ticket checks keep every late start or poll result from changing the stopped or replacement UI.

The eleventh R2c-Q remediation closes a Dart-side ownership gap created by that process-global
admission boundary. Once public stop successfully reserves a cancellation fence, the registry may
already have moved a different controller's native owner to `Stopping` or `Draining`. The matching
asynchronous fenced stop is therefore unconditional; local `_isStarted`-style presentation state no
longer decides whether native drain runs. An unstarted second controller, a hot-restart replacement,
or close-before-start now drains the owner covered by its admitted fence. Native `Empty` and
`library_synchronization_not_started` remain idempotent convergence, while every other stop failure
remains retryable and does not publish a false stopped state.

The process-lifetime Flutter owner now caches only an in-flight or successful close. A failed dispose
clears that completed future so the same owner can retry native cleanup, but `_isClosing` remains
permanently true and background start cannot be scheduled again. Deterministic Rust boundary tests
also pin zero and unissued start-ticket rejection, zero and unadmitted stop-fence rejection, epoch
exhaustion returning to `Empty` without constructing a runtime, and an old admitted fence remaining
unable to drain a newer epoch.

The twelfth R2c-Q remediation made native lifecycle tokens kind-distinct, but the low-bit tag proves
only the shape of a bridge value, not that a stop fence was issued. The bridge still transports
process-local tokens as `u64`/Dart `BigInt`; Rust owns one checked, strictly increasing
`LifecycleOrdinal`, decodes the required start-ticket or stop-fence kind, and rejects zero before
consulting the admission watermark. This rejects an unchanged start token at the stop boundary, but
flipping its low bit produces a stop-shaped value with the same ordinal. Range and watermark checks
alone therefore do not establish stop-fence provenance. Inside the registry,
`RuntimeStartTicket` and `RuntimeStopFence` remain distinct narrow types, while ordering,
cancel-through, same-start retry, and old-fence/new-epoch isolation compare their shared ordinal
rather than tagged raw values.

The kind bit limits the process-lifetime ordinal to `u64::MAX >> 1`; allocation fails closed with the
existing lifecycle-exhaustion error before encoding can overflow. Tokens are not persisted and need
no cross-process migration. Panic-only `Draining` ownership records no admitted fence instead of
forging one from its start ticket; a later real stop may attach its first admitted fence without
changing the retained owner, deadline, or runtime.

The thirteenth R2c-Q remediation adds exact, deterministic, bounded stop-fence provenance. The
process-local runtime state owns a hard-capped 64-entry admitted-fence ledger. Each entry records the
exact typed fence, the owner visible at reservation, and whether the fence is pending or consumed.
Reservation checks ordinal space and ledger capacity under the same registry mutex, records the
exact pending fence, then advances cancel-through and transitions its covered owner before releasing
the mutex. If all 64 entries are pending, or the only consumed entries still cover the active owner,
the next reservation returns
`library_synchronization_lifecycle_fence_capacity_exhausted` without allocating an ordinal,
advancing the watermark, signalling cancellation, or changing owner state.

Pending fences are never evicted. A stop call first proves exact ledger membership and marks that
entry consumed; concurrent and repeated calls for the same retained fence converge on the same
drain and never close twice. Consumed history is reclaimed only when a later reservation needs
capacity, starting with an entry that no longer covers the active owner. A call that already proved
membership continues from its local typed fence even if later capacity pressure reclaims history.
An admitted fence reserved while the registry was `Empty` may be consumed later as an idempotent
no-op after a newer owner appears, while ordinal ownership prevents it from draining that newer
owner. A timed-out or panicked drain may retry the retained fence, and a later real admitted fence
may finish the same retained `Draining` owner without refreshing its first deadline. The ledger is
process-local, never persisted, and adds no catalog migration.

The final controlled disposable-root performance run used 2,048 real PNG P1 candidates, a cold
10,000-entry P2 recovery with the production 4,095-entry output page, a competing low-lane writer,
and 25 live PNG changes. Its first P0 sample occurred after only the first 128 raw P2 source entries
had been consumed and before the first output page existed. P1 was active and made real progress in
25 of 25 samples, completed all 2,048 candidates, and P2 was active and consumed source entries in
25 of 25 samples. P2 source reads advanced from 128 to 3,200 to 10,000 and then published 4,095
staged entries; the low-lane writer advanced from 1 to 391,335 transactions. Queue P95 was 31 ms,
worker-start P95 was 30 ms, visible-query P95 was 0 ms, and end-to-end event-to-visible timing was
P50 136 ms, P95 179 ms, maximum 251 ms, with zero samples above one second. The current third-round
focused rerun completed in 98.16 seconds; the earlier 61/70/78 ms, 70.19-second result remains only
a historical pre-remediation snapshot.

A separate production coordinator fixture enumerated 4,096 real controlled files,
published the default 4,095-entry source page, exercised the steady 3,072-row P2 backpressure and
drain/refill path, terminalized all 4,096 candidate owners, and reached completed control/run,
`Current`, and retired authority only through production poll, open, worker, closing replay, absence,
and finalizer steps. Its eighth-remediation ordinary-user rerun passed in 73.61 seconds; the earlier
33.63-second run remains historical evidence. The 100-startup no-change fixture remains the separate
zero-enumeration contract. Its second R2c-R remediation run traverses exactly 200 actual production
polls, performs exactly 200 metadata-only root-availability probes, and completes 100 journal queries
and 100 session closes with zero physical journal reads, inventory runs, source-root entry
enumerations, discovery-root handles, publication-guard root handles, media opens, or new scan rows.
The availability adapter exposes only one metadata operation to the algorithm; its executable
contract returns `Available` for a metadata fixture while the supplied enumeration path is poisoned
and non-existent, proving the normal path is O(1) in source-root entries.

The isolated release-profile large-v26 session fixture populated 1,024 queue, lineage, owner, and
frontier rows, completed its one migration and complete validation in 55.447 ms, and then performed
100 production-session reopens in 1.1784592 seconds. The slowest individual constant-size reopen was
13.2528 ms and instrumentation recorded exactly one complete validation.

Final format, development- and release-profile check and warnings-denied Clippy, and the complete
Rust suite pass for this remediation checkpoint. The earlier seventh-remediation focused reruns pass
34/34 local-file, 41/41 incremental, 11/11 authoritative, 35/35 metadata-inventory, 47/47
production, 274/274 catalog, 62/62 migration, 23/23 legacy, and 36/36 scan-library tests with two
intentional scan ignores. The eighth-remediation panic matrix adds eight production tests covering
constructor recovery with and without a concurrent stop, poll panic before and after worker
creation across the public deadline, one-shot panics at every cleanup boundary, persistent panic,
and a stale drainer; its complete serial production module passes 55/55 in 203.65 seconds. Ninth-
 remediation red evidence records Flutter 20 passed and 2 failed plus the forced Rust interleaving
 failing 1/1 before the fixes. The green lifecycle file passes 26/26, the panic/stop filter 5/5, and
 the complete ordinary-user serial production module 57/57 in 147.66 seconds. Tenth-remediation red
 evidence records Flutter 26 passed and 1 failed because a non-transient start was attempted four
 times, plus the delayed-start Rust interleaving failing 1/1 because stop at `Empty` did not prevent
 construction. The fixed controller and lifecycle-owner files pass 28/28 and 2/2, and the complete
 ordinary-user serial production module passes 62/62 in 241.94 seconds. Its non-sandbox Daily
 records 846 passed, zero failed, and 11 ignored library tests, followed by 3/3 broker integration
 tests and all 307 Flutter unit and widget tests.

Eleventh-remediation red evidence first failed against the prior Dart production code: the
controller file passed 28 tests and failed 1 because an unstarted controller reserved a global stop
fence but made zero native drain calls, and the lifecycle-owner file passed 2 tests and failed 1
because a failed close future remained permanently shared. After the fixes those files pass 29/29
and 3/3. The four new Rust lifecycle boundary tests were direct green against the existing native
implementation; no Rust production behavior was changed. The focused Rust filter passes 4/4, and
the complete ordinary-user serial production module passes 66/66 in 281.70 seconds.

Twelfth-remediation evidence used a deterministic native interleaving in which a ready start ticket
was followed by a real stop fence and then the unchanged start raw value was presented at the stop
boundary. It proved kind decoding, but did not prove issuance: changing only the low bit was not in
that fixture. Its historical 5/5 lifecycle-boundary and 67/67 ordinary-user production results must
not be read as stop-fence provenance evidence.

Thirteenth-remediation red evidence repeats the ready-owner interleaving but presents
`start_ticket.raw() | 1` after a later real fence has advanced cancel-through and published
`Draining`. The tagged implementation decoded the same-ordinal value, accepted it through the range
checks, returned success, and closed the owner; the focused run failed 1 test with 861 filtered.
With the exact admitted-fence ledger, the forged raw value returns
`library_synchronization_lifecycle_fence_invalid`, performs zero closes, leaves the real owner in
`Draining`, and the real admitted fence drains exactly once. The production lifecycle-boundary
filter passes 6/6, the stop-fence filter passes 8/8, and the active-owner capacity-pressure retry
fixture passes 1/1.

Twelfth-remediation explicit Rust formatting, development- and release-profile check, and
warnings-denied Clippy pass. `quality_lint.ps1` passes with 149 files unchanged and no Dart analyzer
issues. The unchanged Daily gate passes under the previously established one-Cargo-job,
one-Rust-test-thread resource bound: 851 Rust library tests pass, none fail, and 11 authorization or
manual-performance tests remain ignored in 255.01 seconds; broker integration passes 3/3 in 2.37
seconds; all 309 Flutter unit and widget tests, controlled Windows scan 2/2, native Windows
accessibility 2/2, generated bridge compatibility, and tracked-diff whitespace validation pass.
The internal Windows x64 Release application builds in 96.9 seconds. A formal Release-verifier run
with explicit absent external application-bundle and broker paths fails closed before SCM or process
startup with `The Ame application bundle was not found`; no signed-bundle admission is claimed.

Thirteenth-remediation explicit-file formatting, development- and release-profile check, and
warnings-denied Clippy pass. `quality_lint.ps1` passes with 149 files unchanged and no Dart analyzer
issues. The complete ordinary-user serial production module passes 72/72 in 74.32 seconds. The
unchanged Daily gate passes with one Cargo build job and one Rust test thread: 856 Rust library tests
pass, none fail, and 11 authorization or manual-performance tests remain ignored in 190.63 seconds;
broker integration passes 3/3 in 2.36 seconds; every Flutter test file, controlled Windows scan 2/2,
native Windows accessibility 2/2, generated bridge compatibility, and tracked-diff whitespace
validation pass. The internal Windows x64 Release application builds in 89.9 seconds. Formal
`release_verify_windows.ps1` with explicit absent application-bundle and broker paths fails closed
before SCM or process startup with `The Ame application bundle was not found`; no signed-bundle
admission is claimed.

The fourteenth independent read-only R2c-Q re-audit reports zero Critical, High, Medium, or Low
findings. Its focused reruns pass 6/6 Rust lifecycle-boundary tests, 8/8 stop-fence tests, 1/1
active-owner capacity/retry test, 29/29 Flutter controller tests, and 3/3 process-lifetime owner
tests; `git diff --check` is clean. This closes the R2c-Q implementation/audit checkpoint without
accepting R2c-Q, R2c-P, R2c-O, or the accumulated R2c milestone.

The R2c-R non-external controlled local checkpoint also passes its final applicable gates. The
resource-bounded complete Daily run reports 859 Rust library tests passed, zero failed, and 16
expected ignored in 216.77 seconds; broker binary integration passes 3/3 in 2.07 seconds; Flutter
unit and widget tests pass 309/309, controlled Windows scan passes 2/2, and native Windows
accessibility passes 2/2. The internal unsigned Windows x64 Release application builds in 70.1
seconds, and the repository PE parser verifies machine `0x8664` for the application and Rust DLL.
All 81 Cargokit Release dependency files are present and no newer than the built DLL, the packaged
DLL hash matches, and dependency/binary scans contain zero R2c-R fixture, `test_support`, harness,
environment, or counter-name matches. Formal Release verification with explicit absent signed-
bundle and broker paths fails closed at bundle admission before SCM, FSCTL, or packaged-process
execution. These are local implementation and negative-admission facts, not external Release
acceptance.

The independent R2c-R audit then found four High, three Medium, and two Low evidence defects in
that checkpoint rather than in the accepted architectural direction. The remediated gate no longer
trusts caller public/temp variables: it cross-checks Windows 11 client registry, native
`RtlGetVersion`, product type and SKU evidence, resolves one physical fixed-NTFS Local AppData
known-folder descendant, rejects reparse/volume escape, and contains source, catalog, report, and
logs below one nonce root. Cargo children enter a private kill-on-close Job Object before resume
and have parent wall-clock deadlines. The exact 19/15/4 matrix is validated against current test
attributes, and closed workers bind exactly one report to the run nonce, runner PID, parent PID,
actual child PID, and phase.

The corrected production evidence distinguishes 100 necessary watcher root handles from zero
enumeration across 100 fully harvested, nonzero-instance, `Current` starts; bounds 115 affected-path
reads and exactly 99 media opens against 4,096 unrelated live-storm entries with no new scan row;
routes two same-volume roots through one retained production session read while isolating one root
failure; and drives one million unrelated records through the 256 KiB production buffer, real
per-USN reference histories, one million scope decisions, and fixed-shape capacity accounting.
These repairs close only local evidence findings. The checkpoint remains non-external and not
accepted until the unchanged external boundaries below are available and the final accumulated
audit closes.

A second independent R2c-R review reported zero Critical, two High, one Medium, and one Low finding.
The native common module now has a definition-only load boundary: explicit initialization first
establishes a high-entropy, non-reparse repository bootstrap and rebinds compiler temporary storage,
then restores hostile environment values and removes all bootstrap state on every exit. Disposable
storage no longer traverses `LocalAppData\Temp` or creates a predictable owner. It opens the physical
LocalApplicationData KnownFolder base, creates the root exclusively with `NtCreateFile` relative to
the held parent, verifies the returned non-reparse same-volume handle as a direct child of the held
physical anchor, and holds the complete OS filter-redirection chain without delete sharing through
the runner lifecycle until cleanup begins. After every owned child exits, cleanup releases the root
blocker, reopens the unpredictable child relative to the still-held physical parent, rechecks its
exact file identity and physical path, and deletes that empty handle. This cleanup transition is not
atomic: a same-user racer can force fail-closed residue, but an identity mismatch cannot redirect
deletion to a replacement. The guardrail performs only internal junction attacks and proves
rejection, zero sentinel writes, and zero failed residue. Production now records one metadata-only
availability probe per actual poll instead of claiming zero, and an O(1) adapter contract prevents enumeration
from becoming part of that algorithm. The missing/ambiguous worker-report binding test is the unique
nineteenth public case. These findings are closed only at the non-external implementation checkpoint;
status remains not accepted and all signed-bundle, SCM/service, named-pipe/FSCTL, retained-root, Cloud
Files, and final accumulated audit boundaries remain open.

A third independent R2c-R review found two High, one Medium, and two Low defects in those local
guardrails. The compiler bootstrap now starts with an in-memory `Reflection.Emit` P/Invoke surface,
so no compiler write occurs before the repository `tool` parent has a non-delete-share handle and a
verified non-reparse identity and final physical path. `NtCreateFile` creates the high-entropy child
relative to that held parent; the same identities are rechecked after `Add-Type`, and only an empty,
identity-matching relative reopen can remove it. Active replacement is blocked, a pre-created
junction fails at exclusive create, and forced compilation failure leaves no safely deletable
residue or sentinel change. Runtime cleanup has no managed recursive or catch fallback: native
reopen, identity, physical-path, reparse, or emptiness failure reports and retains the owned root.
Executable ordinary and junction replacement races prove that neither a replacement nor a junction
target is deleted or traversed, after which the guardrail removes only its own original fixture by
the retained identity. Failures report the owned leaf and expected file-identity token rather than
misidentifying a replacement at the original path as the owned object. Owned NT leaves are limited
to the fixed ASCII prefix and two 32-character lowercase-hex fields; the only additional form is an
exact `-moved-<32 lowercase hex>` suffix used by the internal cleanup race fixture. ADS, separators,
controls, dot components, terminal dot/space, arbitrary ASCII suffixes, uppercase hex, and non-ASCII
names are rejected.

The fourth review found that both writable-anchor paths still trusted more namespace than their
terminal handles proved. The accepted local gate now completes a read-only binding phase before any
bootstrap or runtime-root write. Starting at the trusted volume root, it opens every existing
component of the logical repository-tool or `SHGetKnownFolderPath` path relative to its held parent
with `FILE_OPEN_REPARSE_POINT`, rejects any reparse or volume transition, and retains each handle and
identity. It then repeats the same binding for the terminal handle's filter-resolved physical path
and requires logical and physical terminal volume/file identity equality. Only the final held
physical parent can create the unpredictable leaf with `NtCreateFile`. This admits a legitimate OS
filter/container redirection that is already visible through the resolved handle while preventing
an intermediate junction from redirecting the first write. Parent-relative object lookup consults
the live per-directory Windows case-sensitivity flag; case-folded path text is not identity
evidence. A production KnownFolder intermediate-junction fixture proves rejection before the root
create with zero sentinel change and no root residue.

The same review makes every guardrail teardown resource identity-bound. Files that need no retained
post-close path are opened with delete-on-close handles. Directories are created or captured through
held-parent no-follow handles with expected volume/file identities, and teardown uses only a
parent-relative reopen that rechecks identity, reparse state, volume, and emptiness before native
handle deletion. An ordinary replacement and a junction are each swapped a second time between
validation and teardown: stale expected tokens retain the unknown object, and only a newly captured
fixture identity permits later cleanup.

Root availability remains split between a filesystem probe that produces opaque metadata evidence
and a classifier whose type signature carries no path or directory-enumeration capability. The
executable source guard now parses the Rust file with `syn` 2.0.119 and visits the owning functions'
ASTs through a default-deny structural allowlist. The entry function admits only the explicit probe
and classifier calls; the probe admits only the exact metadata adapter, opaque evidence constructor,
test instrumentation, and pure string/error methods it currently requires; and the classifier
admits only its exact `Some` construction and string literals. Any macro, local item or `use`,
unsafe, closure, async block, unknown expression shape, indirect call, unknown callable path, or
unknown method is rejected. Module and local aliases, macro-token calls, external helpers, method
aliases, and fully qualified directory enumeration therefore fail through the same predicate rather
than an enumeration-name blacklist. Ordinary strings, raw strings, comments, and the current three
production ASTs remain green. `syn` is an exact, test-only direct
dependency under its existing `MIT OR Apache-2.0` license; the locked version was already a
transitive procedural-macro dependency, and the production/release dependency graph gains no new
edge. The existing 100-start fixture still performs exactly 200 polls and 200 metadata probes with
every enumeration, inventory, full-scan, media-open, discovery, and publication counter at zero.
Finally, `quality_lint.ps1` reaches the lightweight guardrail, which now actually executes the exact
worker-report tamper test and accepts only one pass with zero failed, ignored, or measured tests.
These are local evidence hardenings, not new release authority; the signed-bundle, SCM/service,
named-pipe/FSCTL, retained-root, Cloud Files, and final accumulated audit boundaries remain open.

The fifth review found that the Rust guard was still a callable-name blacklist, the PowerShell
deletion audit did not cover the common module or recursively parse child commands, and two native
post-open exception windows did not yet have immediate local handle ownership. The common module now
owns one reusable PowerShell AST audit used for the common module, runner, guardrail, and every
actual `EncodedCommand` or fresh `-Command` payload. It normalizes module-qualified command names to
their leaf, rejects deletion aliases and alias definition, rejects `Invoke-Expression`,
`ScriptBlock.Create`, reflection invocation, and path-addressed `.Delete`, and applies an exact
command/member allowlist to the three cleanup-owning functions. A dynamic `& $variable` or any
unknown cleanup command therefore fails closed. Executable bad fixtures cover module qualification,
dynamic invocation, aliases, each of `File.Delete`, `Directory.Delete`, and `Remove-Item` inside a
recursively parsed child, reflection, and an unknown cleanup command; current scripts and child
payloads use the same passing audit.

Native anchor binding now places the newly opened volume handle inside local `try/finally` ownership
before reparse, identity, or filter-path queries, and transfers it to the retained chain only after
the identity and both retained-list entries succeed. Child handles retain the same immediate local
ownership, and retained-list insertion rolls back if its paired identity entry cannot be recorded.
Failed-root cleanup first transfers `rootHandle` from the field to one local owner, clears the field,
then wraps fault injection and identity capture in `try/finally` before any identity-bound reopen.
The executable tool-only fault seam records `SafeHandle.IsClosed`; pre-fix evidence showed both
handles open and a same-process root rename blocked, while the corrected path reports both closed,
no held root, and successful same-process rename plus identity-bound empty-root deletion. This seam
is compiled only from the acceptance common script and is not a Rust or packaged Release edge.

Fresh fifth-remediation ordinary-user evidence passes the exact 19-case runner, the lightweight
guardrail, the 19/15/4 ValidationOnly matrix, format, development/release checks, warnings-denied
Clippy, `quality_lint.ps1`, and the complete Daily gate. Daily records 861 Rust library tests passed,
zero failed, and 16 ignored; broker integration 3/3; Flutter 309/309; and Windows scan plus native
accessibility 2/2 each. The unsigned Windows x64 application and broker build in 38.5 and 75 seconds.
Five inspected PE files are machine `0x8664`; the 81-file DLL and both 82-file broker dependency
graphs are current; packaged/built DLL hashes match; and dependency plus boundary-qualified binary
scans contain zero fifth-remediation test seam. Explicit missing signed inputs and nonexistent signed
artifacts fail closed with zero packaged process started. These results neither provide nor
substitute for the external authorities that remain open.

The sixth review found zero Critical, zero High, two Medium, and one Low defect in the remaining
local evidence boundary. First, the Rust availability source proof could still be bypassed by
callee or receiver shadowing, a newly side-effecting helper, or reachable `Drop`, operator, or
lazy/static behavior. The replacement contract starts from the three production owners, resolves
their exact local call closure, and locks 17 cfg-qualified function items plus 16 support items by
readable item key, purpose, normalized-token digest, direct local callees, and observed call and
receiver shapes. Missing, changed, ambiguous, or newly reached items fail by key. A separate
full-crate visitor rejects side-effect traits only for the six evidence types reachable from that
closure, so unrelated implementations are not swept into the policy. The exact digest makes local
bindings, callee provenance, receiver provenance, static references, macros, new expression shapes,
and helper bodies part of the accepted contract; it is not a blacklist of suspicious filesystem
names. Production availability behavior remains the same bounded O(1) metadata path.

Second, the PowerShell deletion proof now operates as a default-deny source and execution closure,
not an owner-name list. The common, runner, guardrail, every recursively reached local helper, and
every exact dot-source dependency are audited from held read snapshots. Dot-source resolution must
produce an exact repository-tool script identity and content; variable or otherwise unrecoverable
loading fails. Trusted cmdlets use an explicit leaf/module allowlist, while unresolved commands,
call operators, unknown helpers, external executables, alias creation, dynamic script construction,
reflection, `System.IO` mutation, `cmd`, and mirror/delete tools fail in every scope. The sole
process wrapper and its native Job Object implementation are independently digest-locked. Before
launch, that wrapper validates the exact Cargo matrix or recovers and audits the final actual
PowerShell `Command`, canonical UTF-16LE `EncodedCommand`, or held `File` source and arguments; an
unrecoverable or nested external payload fails closed. This keeps runtime-generated legal payloads
possible without admitting an unaudited execution boundary. Forty-five adversarial source/runtime
fixtures and every actual legal child payload exercise the rule.

The remaining handle-ownership window is closed at the point of acquisition. Volume binding owns
its `SafeHandle` in a nullable local `try/finally` until both retained records accept the transfer.
Compiler initialization owns the bootstrap before environment access and keeps creation, native
definition work, compilation, and later initialization inside one nullable-owner `try/finally`.
The tool-only `volume-post-open-pre-transfer` and
`bootstrap-post-create-pre-initialization` faults execute before controlled native types are
initialized and prove the corresponding handle or bootstrap owner is already closed in-process.
Neither seam is a Rust or packaged Release edge.

Fresh sixth-remediation evidence passes the exact adversarial Rust control, PowerShell parse and
closure checks, both native faults, the 45-case audit matrix, 19-case guardrail, 19/15/4 validation
matrix, no-change/tamper/replacement/Cloud Files focused tests, all-target/all-feature check,
warnings-denied Clippy, `quality_lint.ps1`, the complete ordinary-user runner, and serial Daily.
Daily records 861 Rust library tests passed, zero failed, and 16 ignored; broker integration 3/3;
Flutter 309/309; and Windows scan plus native accessibility 2/2 each. The correction changes only
acceptance tooling, test-only source proof, and exact dev dependencies, so the fifth-remediation
fresh unsigned Release remains the applicable artifact rather than being relabeled as a new build.
A new ASCII/UTF-16 scan finds both new fault tokens absent from the application, packaged and
Cargokit Rust DLLs, and independent broker. These facts do not provide an external signature,
publisher, installed service, real pipe/FSCTL, retained-root authorization, Cloud Files acceptance,
or final accumulated audit.

The seventh review found zero Critical, zero High, two Medium, and one Low defect in the remaining
proof boundary. The first Medium finding was a Rust macro-resolution hole: the 17 function and 16
support-item token digests did not include a same-module or parent-module macro namespace, so the
unqualified `vec!` inside `windows_extended_path` could resolve to different code without changing
either digest set. The production function now uses the fully qualified non-macro
`std::vec::Vec::with_capacity` constructor and `extend_from_slice`. The availability contract rejects
every macro expression in the protected call closure, admits only locked `cfg` attributes on those
functions and locked built-in derives on named support items, and scans the local, domain,
metadata-domain, adapters-parent, and crate-parent scopes for relevant `macro_rules!`, macro-name
imports or renames, glob imports, and `macro_use`. Synthetic same-module, parent-module, and renamed-
import `vec` environments fail with the exact protected item and source key. The production
availability path remains O(1); this change removes a parsing-environment ambiguity rather than
adding filesystem work.

The second Medium finding was an object-continuity gap at the PowerShell `-File` boundary. The old
audit held an ordinary followed-path `FileStream`, while the child opened the same spelling again.
A real Windows PowerShell child probe confirmed that wrapping the same source bytes in an anonymous
`EncodedCommand` script block clears both `$PSScriptRoot` and `$PSCommandPath`; the runner's
transitive `quality_common.ps1` and acceptance-common functions also depend on those file semantics.
Rewriting all three sources and teaching the static audit to ignore the original dot-sources would
create a second general loader, not an execution of the same canonical bytes. The accepted fallback
therefore keeps `-File` semantics but removes the ordinary stream as identity authority. A separately
digest-locked native `AmeR2cRAuditedScriptSnapshot` binds the repository tool root and every source
parent component with parent-relative `NtCreateFile`, no-follow and live case-sensitivity semantics,
rejects reparse or volume transitions, opens the terminal no-follow, rejects a terminal reparse or
directory, and records its volume/file ID. All ancestor and terminal handles remain open without
delete sharing, and the terminal also denies write sharing. Immediately before either native Job or
detached transfer, every source in both the payload and wrapper closure is re-opened relative to its
held terminal parent and compared with the retained identity. The existing native Job suffix keeps
its independent digest. Ordinary identity, terminal junction rejection, post-audit terminal swap,
post-audit parent swap, and injected exceptional-open closure are executable, fixture-only controls.
There is no unverified `-File` launch path.

The Low finding was unbounded PowerShell closure construction. The audit now fixes eight small
limits: 8 sources, dot-source depth 8, 256 KiB per source, 512 KiB total source bytes, 32,768 AST
nodes, 128 function definitions, 512 audited scopes, and a 512-entry queue. Every increment uses one
checked `uint64` operation and reports `budget`, `limit`, and `actual` (or `overflow`). Source count
and depth are checked before any native open; file length, per-source bytes, and total bytes are
checked before reading or `ParseInput`; AST and function counts are checked immediately after the
bounded parse but before state publication; scope and queue counts are checked before either
collection changes. Path deduplication happens before I/O but cannot replace any budget. The final
real-closure measurement peaks at 3 sources, depth 1, 158,839 bytes for one source, 239,967 total
bytes, 17,943 AST nodes, 60 functions, 54 scopes, and queue high-water 47. Limit-minus-one, limit,
limit-plus-one, multi-source/depth, and checked-add overflow controls all execute in memory; the real
runner closure remains the three-source depth-one integration control.

Fresh seventh-remediation verification passes all three PowerShell parses and real closures, the
four Rust availability-contract tests, exact no-change, report-tamper, root-replacement, and both
Cloud Files cases, the lightweight 19-case guardrail, exact 19/15/4 `ValidationOnly` matrix, format,
development and Release all-target/all-feature checks, warnings-denied Clippy, `quality_lint.ps1`,
and the complete ordinary-user 19-case runner. The serial Daily gate records 862 Rust library tests
passed, zero failed, and 16 expected ignored; broker binary integration 3/3; Flutter 309/309;
controlled Windows scan 2/2; native Windows accessibility 2/2; generated bridge compatibility; and
whitespace validation. The production change is an O(1)-equivalent standard-library `Vec`
construction and does not change ABI, bridge, package graph, or runtime policy, so no fresh Windows
Release is claimed. The retained fifth-remediation application, packaged DLL, Cargokit DLL, and
independent broker remain prior packaging evidence rather than a current-tree build or new
signature; ASCII/UTF-16LE scans find seven new macro, identity, fault, and budget tokens absent from
all four images.

The ninth reliability review found zero Critical, zero High, one Medium, and zero Low defect. The
eighth remediation's first complete watcher-overflow runner had failed closed on SQLite
`FileLockingProtocolFailed` (code 15), and five later independent exact roots reproduced that
transient boundary as four passes and one failure. The bundled `rusqlite` 0.40.1,
`libsqlite3-sys` 0.38.1, and SQLite 3.53.2 path is correctly configured for WAL; the defect was
ownership of transient WAL protocol recovery, not an outdated SQLite or a busy-timeout omission.
The previous application reader repeated full catalog open/validation and a separate query open for
each request, and the acceptance observer performed raw opens every 10 ms. Open, fast schema
validation, and query could each expose code 15 and map it to a generic database error.

The accepted local design places protocol recovery in the SQLite adapter, with application policy
continuing to own creation and migration. `SqliteCatalogReadExecutor` is usable only for idempotent
catalog reads. Each protocol failure destroys the prior `SqliteCatalog`, opens a new connection,
runs a pure `SQLITE_OPEN_READ_ONLY` schema-cookie/application-ID/version/catalog-identity
validation, and then executes the operation. A missing or preparation-required catalog returns to
the application boundary, which may explicitly run the existing full validation/migration once and
then constructs the read owner. No retry closure can migrate, write, retain a transaction across
sleep, or repeat preview reconciliation or usage publication. The six public catalog read families
share this one route; write and root-removal paths are unchanged.

Only rusqlite `ErrorCode::FileLockingProtocolFailed` is classified as
`catalog_database_protocol`. Busy and Locked retain the established five-second SQLite busy-handler
semantics and return their existing errors without owner-level retry; every other error also returns
immediately. Production permits five total attempts inside a 100 ms monotonic protocol-retry
admission window and sleeps 1, 2, 4, then at most 8 ms. The window is checked after each completed
SQLite call and before another attempt, so it bounds admission and backoff but cannot interrupt an
active SQLite call. Exhaustion is `catalog_read_protocol_retry_exhausted` and preserves operation,
attempt count, actual elapsed milliseconds, and the final structured cause without placing an
absolute catalog root in the message. An uncontended success has no sleep.

The controlled watcher path now holds a persistent production-equivalent observer for gap count,
active inventory, active recovery authority, and location reads. It returns structured errors to the
outer report rather than reopening raw SQL, ignoring a result, or panicking. Deterministic tests
prove protocol recovery and fresh connections at open, validation, and query; non-protocol/Busy/
Locked no-retry; attempt and monotonic-window exhaustion; no transaction or writer admission across
backoff; and current/missing catalog no implicit migration. A real WAL writer plus the public
timeline read completes 256 subprocess reads. Thirteen focused tests pass. The first full Daily
revealed that four attempt-count fixtures could spend 101-212 ms in one fresh open under parallel
load; only those deterministic fixtures now use a 30-second test budget, while a dedicated 1 ms
deadline test and all production paths retain the exact production policy.

A partial change covering only the original gap helper was rejected after four green exact samples
and one nonzero fifth sample whose child log was not retained by the diagnostic wrapper. Once all
raw observer reads used the same owner, twenty consecutive fresh-nonce, identity-held exact cases
passed. P95 spans 307-327 ms, convergence 11.463-11.872 seconds, maximum P0 is 420 ms, and each run
uses one attempt per operation with zero protocol retries. The subsequent full ordinary-user
19-case runner passes, with watcher P95 308 ms, 11.631-second convergence, 795 operations/attempts,
zero retries, and maximum attempt one. Guardrail 19, ValidationOnly 19/15/4, topology/macro/source,
case-sensitive, parse, build, lint, and complete Daily gates pass; Daily reports 878 Rust passed,
zero failed, and 17 ignored plus all Flutter and Windows integration gates.

The current unsigned x64 application and independent broker rebuild, all application/DLL/broker
PEs are `0x8664`, 82 Cargokit dependencies are present and current, packaged and built DLL hashes
match, and ASCII/UTF-16 scans contain no new test seam. Formal signed Release and portable
admission fail closed on missing external signed inputs before process, SCM, named-pipe, or FSCTL
work. This evidence remains a non-external implementation checkpoint. R2c-R is not accepted,
R2c-O remains active, and the external signed bundle/publisher, installed-service, real brokered
journal, retained-root, real Cloud Files, and final accumulated audit remain required.

The tenth R2c-R boundary review reports zero Critical, zero High, two Medium, and one Low finding
against the ninth-remediation reader. The read executor's generic callback is now private to its
module; crate callers can invoke only sealed named read methods and cannot receive a connection,
transaction, or mutable catalog. A structural `syn` contract locks that signature boundary. Raw
rusqlite `FileLockingProtocolFailed` is captured only inside an active attempt and becomes an
internal typed attempt failure; public `ScanError.code` and message text are never retry authority.
Busy, Locked, ReadOnly, Corrupt, I/O, and forged protocol codes remain terminal. Exhaustion has a
fixed sanitized message and optional typed operation, attempts, elapsed milliseconds, and cause,
which canonical FRB generation exposes to Dart while ordinary errors retain absent details.

Every Windows read validation and query attempt now binds SQLite to a retained catalog handle. The
standard-library `OpenOptionsExt` open requests read-data, read-attributes, and synchronize access,
shares read/write without delete, sets no-follow and no-recall flags, rejects a terminal reparse or
non-file, and obtains identity from the live handle. That identity must equal the session identity
before `Connection::open_with_flags` uses `READ_ONLY | URI | NO_MUTEX`. The `SqliteCatalog` field
order drops the connection before the guard; errors and retry backoff release both exactly once.
Delete, replacement, ABA, guard mismatch, reparse, cleanup, and release-order fixtures cover this
ownership. The implementation adds no `unsafe` and reuses the accepted local-files identity adapter.

The retry clock is private. Release builds construct the system monotonic clock and sleep through
`Instant` and the standard thread API; only `cfg(test)` can inject the manual clock, fault owner, or
stage observer. With the unchanged five-attempt/100 ms/1-2-4-8 ms constants, deterministic tests
cover second-through-fifth-attempt success at open, validation, and query, post-deadline admission
refusal, a single call crossing 100 ms, checked/saturating elapsed arithmetic, and the zero-sleep
uncontended path. The persistent R2c-R observer also routes completed P1 evidence through a named
typed read, so its location and range polling no longer use raw reopening or `expect`.

Focused evidence passes 32/32 retry tests, 62/62 migration tests, application catalog/WAL 256,
canonical bridge generation and SSE serialization, three script parses, guardrail 19,
ValidationOnly 19/15/4, topology/macro/source closure, no-change, tamper, root replacement, and both
disposable Cloud controls. Sandbox access-denied attempts remain recorded and only ordinary-user
reruns count green. This does not accept R2c-R: R2c-O remains active, and repeated watcher, complete
runner, Daily, fresh packaging, signed/service/real-journal, retained-root, real Cloud Files, and
final accumulated audit evidence remain outstanding.

The eleventh R2c-R remediation closes the bounded-WAL and observer-frequency gaps found while
completing that review. Locked SQLite 3.53.2 `walTryBeginRead` can traverse up to 100 protocol retries
with quadratic delay after retry ten, matching the observed 10.941-second read. The bundled Windows
build now enables SQLite's supported `SQLITE_ENABLE_SETLK_TIMEOUT` path. Every read connection sets
`busy_timeout = 100 ms` and `query_only`; the writer retains five seconds. This composes the Windows
blocking shared-memory lock with the existing five-attempt/100 ms protocol owner without increasing
either limit. The compile-option, read-timeout, and writer-timeout contracts are executable tests.

The controlled watcher observer now reads gap, inventory, authority, and bounded locations in one
typed snapshot, including one bounded bulk query for its 256 locations. The pre-aggregation exact
control failed at 789 operations; twenty fresh-nonce corrected runs pass with 474-527 operations,
zero retries, maximum attempt one, sample-P95 P95 374 ms, and convergence P95 12.128 seconds. This
changes only observation round trips; it does not retain an unbounded transaction or expose a
connection, callback, or mutable catalog.

The closed-process fixture then began waiting at the then-current production 250 ms synchronization
cadence. This fixes a test harness that polled 10 ms after the observer became faster; it does not
cache availability or remove validation. Each actual poll still owns one fresh O(1) metadata probe.
Retained red evidence is 78 probes in the first complete runner and 75/75/77 in exact reproduction.
Five corrected exact runs each use 24 probes, the final 19-case runner also uses 24, and the
independent no-change contract remains exactly 200 polls/200 probes with no enumeration, inventory,
or media access.

The final non-external evidence is 33/33 retry tests, all 19 exact runner cases, 20/20 watcher runs,
format/check/Clippy/lint/Daily, and a current unsigned Windows x64 app/broker build with PE, dependency,
hash, `NotSigned`, and ASCII/UTF-16 seam checks. This evidence does not accept R2c-R. R2c-O remains
active, and signed publisher/bundle, elevated service, real named-pipe/FSCTL journal, authorized
retained-root and real Cloud Files, and final accumulated audit evidence remain open.

The subsequent cadence-binding review found zero Critical, zero High, zero Medium, and one Low
defect in the acceptance boundary. Production Dart owns the default synchronization cadence in the
`RustLibrarySynchronization` constructor, stores it as `pollInterval`, passes it as the first
argument to `Timer.periodic`, and is constructed by `main.dart` without an override. The
closed-process Rust fixture must not own a second numeric cadence. Its test-only module now embeds
the two authoritative Dart sources at compile time and derives the wait duration through a
default-deny token contract.

That contract lexes comments, raw and ordinary strings, triple strings, and nested interpolation
without treating their contents as code. It then requires exactly one owner class, constructor
default, typed field, timer consumption, and production construction, and rejects a missing,
duplicate, ambiguous, unconsumed, or overridden binding. Only the parsed duration reaches the
closed-worker wait loop. The Dart sources, tokenizer, and parsing seam remain below `cfg(test)` and
introduce no production runtime dependency or Release payload.

The red mutation changed the embedded Dart default from 250 ms to 875 ms while the old Rust fixture
continued to return 250 ms. After binding, the same mutation changes the acceptance duration, the
normal source passes, every malformed ownership case fails, and a source-level control rejects the
removed independent Rust constant/literal. A locked Release library build succeeds and ASCII/UTF-16
scans find none of five cadence parser/source seam tokens in the DLL, static library, or rlib. The
complete ordinary-user 19-case runner remains green; its closed-process case reports 567/581/581 ms
P50/P95/maximum and 24 fresh availability probes.
This decision repairs proof ownership only. It does not accept R2c-R: R2c-O remains active and every
external signed/service/real-journal/retained-root/real-Cloud boundary remains open.

The phase-19 follow-up audit found that the first cadence proof remained syntactic rather than
reachable. Whole-file token counts admitted the only synchronization construction in an unused
top-level helper and the only periodic timer in an unused class method. Its separate raw-text Rust
guard also confused a comment/string literal with code and missed suffixed integer or independent
interval expressions. Those are validation defects, not a change to the accepted production cadence
ownership or the change-driven architecture.

The phase-20 proof was reachability-shaped and default-deny at delimiter depth. In `main.dart`, one top-level async
`main` and one direct success `try` must construct `RustLibrarySynchronization` with zero arguments;
that exact variable must enter one lifecycle owner and the production provider, and that lifecycle
must enter shutdown registration and `startInBackground` in execution order. In the synchronization
owner, one async `_start(int generation, BigInt ownerTicket)` method must contain the class's only
`Timer.periodic`, whose first argument is the single typed `pollInterval` field. Thus an unused helper,
unused method, duplicate entrypoint/method, override, variable substitution, or broken lifecycle link
cannot satisfy the proof merely by retaining familiar tokens.

The phase-20 closed-process worker proof became an AST contract rather than a literal blacklist. The existing
dev-only `syn` dependency selects one worker, one direct immutable cadence local initialized by the
zero-argument production cadence function, one owner call, and three wait calls whose interval AST is
the same local path. Comments and strings are outside the AST; suffixed numeric literals, aliases,
arithmetic, and second owners do not match the data flow. All embedded Dart, tokenizer, parser,
visitor, and fixture code remains in the existing `cfg(test)` module, so the architecture gains no
runtime dependency, ABI, bridge, or package change.

Four executable red controls captured both dead-code admissions, the raw-text false positive, and
the independent-wait false negative before correction. Current green evidence is 14 cadence tests,
the production worker AST contract, warnings-denied all-target/all-feature Clippy, and a complete
ordinary-user 19-case internal-disposable runner. Closed-process P50/P95/maximum is 577/592/592 ms
with 24 availability probes, zero source or inventory reads, and three bounded content opens; the
100-start no-change case remains 200 polls/200 probes with zero enumeration, inventory, or media
access. A read-only scan finds none of six new seam tokens in the retained Release DLL, static
library, rlib, or packaged DLL; no new Release is claimed. This closes only the two Low local proof
findings. R2c-R remains non-external and not accepted, R2c-O remains active, and all external signed,
service, real-journal, retained-root, real-Cloud, and final accumulated-audit evidence remains open.

The phase-21 re-review found zero Critical, zero High, zero Medium, and two Low gaps in those
structural claims. Delimiter depth did not establish statement ownership: an unbraced
`if (false) try` still exposed the nested `try` as apparently direct, receiver matching accepted a
`holder.synchronizationLifecycle` suffix, and a periodic timer inside a constant-false branch or
uncalled local function still counted as `_start` work. Recursive `syn::Visit` similarly counted
waits inside a constant-false branch, closure, or nested item, while the single-segment helper match
missed qualified and `UseTree`-aliased cadence owners. Eight valid mutation controls were red against
the phase-20 implementation before this correction.

The current proof is intentionally a strict structural contract for the checked-in source shape,
not a general Dart or Rust reachability theorem. The test-only Dart cursor parses direct statements
and their owners. It admits one top-level `main` whose direct `TryStatement` success block directly
owns the zero-argument synchronization declaration, lifecycle declaration, exact shutdown receiver,
direct `runApp`/`ProviderScope` binding, and exact lifecycle start receiver in order. A control
statement, nested block, closure/local function, `return`, or `throw` before completion fails closed.
The unique `_start` must directly own one retry loop; that loop must directly own the exact
succeeded-status branch; and the branch must directly own
`_timer = Timer.periodic(pollInterval, ...)` before its direct `started` return. The class-wide unique
timer and typed constructor-field ownership checks remain independent.

The Rust contract now binds the production worker's exact allowed statement ancestry. One direct
cadence local precedes the ready wait anchored immediately after `runtime`; the direct `crash-ready`
branch follows `availability_readiness_probes` and owns its wait as its first statement; and the
visible wait is anchored between `ready` and `ready_to_visible`. All three use the same direct local.
A separate whole-worker visitor requires exactly three wait-call paths, exactly one `syn::Path`
whose final segment is `production_synchronization_poll_interval`, and zero `UseTree` references to
that helper. It therefore rejects qualified or aliased second owners and protected calls in a
constant-false branch, uncalled closure, or nested item. This fixed-shape default-deny boundary is
the complete claim; arbitrary dead-code reachability remains outside it.

Fresh phase-21 evidence is 25/25 focused cadence tests and warnings-denied all-target/all-feature
Clippy. The sandbox runner failed before fixture creation at the known ancestor-pin Win32 5 boundary;
the identical ordinary-user runner passed 19/19 internal-disposable cases. Closed-process
P50/P95/maximum is 560/580/580 ms with 24 availability probes, zero source or inventory reads, and
three bounded content opens; no-change remains 100 starts, 200 polls, 200 probes, and zero
enumeration, inventory, full-scan, or media access. Four retained Release artifacts contain zero
ASCII/UTF-16LE matches for eight new test seam tokens. No Release was rebuilt and no real library or
external broker path was accessed. R2c-R remains non-external and not accepted, R2c-O remains active,
and all external and final accumulated evidence remains open.

Phase 23 replaces that fixed-shape cross-language proof rather than adding a fourth parser layer.
The phase-19 through phase-21 reviews showed that a handwritten Dart lexer/statement cursor and a
Rust `syn` ancestry visitor could only prove their admitted syntax subset; every expansion increased
test-only control flow while leaving cadence authority implicit in production source. The accepted
replacement makes the policy itself data: the exact tracked
`tool/library_synchronization_poll_interval_ms.txt` bytes contain one positive decimal millisecond
value and LF. Non-canonical text, zero, overflow, whitespace, BOM, CRLF, units, or multiple lines fail
closed.

`quality_generate_library_synchronization_policy.ps1` is the sole generator for the Dart constant.
Normal mode writes deterministic UTF-8/LF bytes and `-Check` only compares; its guardrail covers
drift and illegal input, including the exact maximum 9,223,372,036,854,775 milliseconds imposed by
Dart's signed 64-bit microsecond `Duration` representation, and lint runs both before formatting.
Dart production consumes the generated constant through a private implementation constructor and
exposes only zero-argument
`RustLibrarySynchronization.production()`. Dependency, clock, retry, diagnostics, and cadence
injection remain on `@visibleForTesting .testing(...)`; analyzer policy makes production use fatal.
The owning test executes a successful start and observes the actual `Timer.periodic` duration through
a Zone timer factory, so missing, hardcoded, or dead timer installation fails behaviorally.

The shared cfg(test) `production_synchronization_cadence` module reads the same text once with
`include_str!` and strictly parses a positive non-zero value within that Dart-safe maximum.
`ProductionSynchronizationCadence` is stored by `ProductionSynchronizationTestHarness`. R2c-R
composes it with a counted closed-worker wrapper whose readiness, crash-ready, and recovery operations
expose no interval parameter; the child report and parent still validate `1/1/0` or `1/0/1`. R2c-M
obtains every foreground wait interval from the harness rather than owning another constant. The
embedded Dart/main payloads, Dart tokenizer and statement cursor, Rust cadence visitor, and their
mutation matrix are deleted. `syn` remains an exact dev dependency only because the independent
filesystem source-topology boundary still uses it.

Red evidence changed the policy to 875 with stale generated Dart, hardcoded the actual timer at 875,
called `.testing()` from production main, and redirected the recovery counter. Generation check,
timer behavior, analyzer, and Rust count assertions all failed. Restored focused policy, Dart,
Rust, report, warnings-denied Clippy, lint, and ordinary-user lightweight guardrail evidence passes.
The complete ordinary-user runner passes 19/19 with closed-process P50/P95/maximum 566/703/703 ms,
24 availability probes, zero source or inventory reads, and three bounded content opens; no-change
remains exactly 100 starts, 200 polls, and 200 probes without enumeration or publication work. The
serial Daily gate passes 900 Rust tests with zero failed and 17 expected ignored, broker integration
3/3, all Flutter tests, and both controlled Windows integrations 2/2. A fresh local unsigned Windows
x64 Release build proves x64 PE payloads, matching packaged/Cargokit DLL hashes, no policy source in
assets or dependency manifests, and no deleted cadence-source seam in five Rust Release artifacts.
The sandbox commands stop only at the expected held-parent reopen denial; identical ordinary-user
commands pass without weakening the gates. This design removes an inference seam; it does not change
the 250 ms product policy, bridge, ABI, assets, source-media safety, or external acceptance authority.
R2c-R remains non-external and not accepted, and R2c-O remains active.

Phase 24 follows an independent review with zero Critical, zero High, two Medium, and zero Low
findings. First, phase 23 bounded the policy as a signed i64/u64 millisecond value, but Dart converts
milliseconds to signed 64-bit microseconds. The exact common maximum is therefore
`floor(9223372036854775807 / 1000) = 9223372036854775`. PowerShell normal and `-Check` modes validate
this before any output access or change; the shared Rust parser enforces the identical value. Exact
maximum succeeds, while 9,223,372,036,854,776, u64 maximum, and u64 overflow fail. This boundary is a
cross-runtime representation invariant, not a product cadence change; the tracked value remains 250.

Second, R2c-M still carried `Duration::from_millis(250)` beside the phase-23 shared policy. A tracked
875 mutation proved the divergence with actual 250 ms versus expected 875 ms. Cadence parsing and the
value object now live only in the shared Rust test-support module. The R2c-M harness initializes from
that object, and the R2c-R phase-count wrapper composes the harness cadence. Both consumer contracts
interpret an exact `875\n` source, and a tracked 875 mutation passes before restoration. R2c-M remains
historically accepted and its gate remains runnable, but its authorization-bound retained-root phase
was not rerun.

The follow-up review reported zero Critical, zero High, zero Medium, and one Low finding: the harness
still exposed a naked `Duration`, and R2c-M's real wait helper still accepted it. A
`Duration::from_millis(250_u64)` mutation in the real startup-wait path left both the semantic source
containment test and the 875 accessor test green. The design no longer treats source spelling as the
boundary. `ProductionSynchronizationCadence` has a private field and owns the wait loop; it exposes no
interval getter to R2c-M or R2c-R. R2c-M's helper requires that nominal type and every real call obtains
it from `ProductionSynchronizationTestHarness`. R2c-R's counted wrapper calls the same opaque object's
wait method and then reports readiness, crash-ready, or recovery counts. A compile-time function type
locks the R2c-M helper signature; the former `250_u64` mutation now fails with Rust `E0308`, expected
`ProductionSynchronizationCadence`, found `Duration`. A cfg(test)-only thread-local sleeper capture in
the shared module drives the real R2c-M helper with 875 ms and records the exact interval without
sleeping or entering a Release artifact.

Focused contracts, the R2c-M non-accessing guardrail, warnings-denied Clippy, complete ordinary-user
lint, and the complete ordinary-user R2c-R 19-case runner pass. The closed-process result is
565/578/578 ms P50/P95/maximum for the initial correction and 565/585/585 ms for the follow-up opaque
API, with 24 probes, zero source or inventory reads, and three bounded content opens; no-change remains
100 starts, 200 polls, and 200 probes. The sandbox denial remains the
known held-parent `C0000022`/Win32 5 boundary. Production Dart and Release payloads are unchanged, so
Daily and Release were not rebuilt; a fresh eight-token scan across five retained Release Rust
artifacts contains no opaque-cadence test seam.
This correction does not change bridge, ABI, assets, source-media safety, or acceptance authority.
R2c-R remains non-external and not accepted, and R2c-O remains active.

The eighth review found zero Critical, zero High, two Medium, and one Low defect in the local proof
boundary. The first Medium finding extends the Rust source contract from item/call closure to module
loading topology. The five protected `syn` files now lock their complete top-level module declaration
sets, visibility, attributes, and inline/external shape. In particular, the unconditional private
crate-to-adapters-to-local-files and crate-to-domain-to-metadata-inventory declarations are identical
for test and non-test production cfg. File attributes, `cfg_attr(path)`, direct `path`, arbitrary or
procedural attributes, `include!` item macros, extern-crate aliases, and unexpected nested/generated
modules are rejected structurally. The current test-only `thread_local!` declaration is admitted only
with its exact cfg and token digest. This check does not recursively scan unrelated module files; it
binds only the declarations and five source scopes that can select or parse the protected availability
implementation. Same-module/parent/renamed macro controls remain independent and green. No runtime
availability item changed, so the one-probe O(1) contract remains intact.

The second Medium finding was pre-open path folding in the PowerShell deletion audit. `SourceByPath`
is now an ordinal, non-authoritative index and is never consulted to return a source before native
binding. Each request consumes source-count admission, opens a no-follow `AmeR2cRAuditedScriptSnapshot`,
holds and revalidates the parent chain and terminal, and uses only the resulting volume/file ID as the
deduplication authority. A duplicate snapshot is disposed before return; a unique snapshot transfers
exactly one owner into the state; and mismatched text, parse failure, budget failure, or another
pre-transfer exception closes the local owner. Per-source size is still checked before reading into
`ParseInput`, while total bytes count only unique identities. Executable ordinary-duplicate, case-
disabled alternate-spelling, and exceptional-close controls prove this ownership order. A real NTFS
case-sensitive high-entropy child creates both `Safe.ps1` and `safe.ps1`; the lower file contains a
forbidden operation, is opened as an independent identity, and makes the closure fail closed. If the
platform explicitly cannot enable the flag, the control reports `skipped` rather than green.

The Low finding was the AST-node count's `@($ast.FindAll(...)).Count` materialization. The replacement
uses `FindAll` only as a traversal driver with an always-false predicate, retains no node collection,
counts the root once, records a visited high-water, and performs checked budget admission at every
node. The limit-plus-one visit therefore throws immediately with its exact budget, limit, and actual.
Independent fixtures fix the empty AST at two nodes and a literal AST at five, then place that known
shape at limit-minus-one, limit, and limit-plus-one without reusing the production counter. The seven
other budgets preserve their previous admission order. Measured maxima across the current common,
runner, and guardrail closures are 3 sources, depth 1, 162,853 bytes per source, 251,983 total bytes,
18,901 nodes, 62 functions, 56 scopes, and queue high-water 49.

Fresh eighth-remediation local evidence passes the three PowerShell parses, real case-sensitive
control, duplicate and exceptional ownership controls, 19-case guardrail, 19/15/4 ValidationOnly
matrix, Rust topology/macro/exact controls, no-change, report-tamper, root-replacement and both Cloud
Files cases, formatting, development and Release all-target/all-feature checks, warnings-denied
Clippy, and `quality_lint.ps1`. A sandbox Win32 5 ancestor-pin result was recorded as a failure and the
same Cloud fixture passed only outside the workspace sandbox as the ordinary user. One first complete
ordinary-user runner failed closed on a transient SQLite locking-protocol result in watcher overflow;
it retained no owned process or root, and two subsequent complete runners passed all 19 cases through
that same production path. The complete serial Daily passes 863 Rust library tests with zero failed
and 16 expected ignored, broker integration 3/3, Flutter 309/309, and Windows scan plus native
accessibility 2/2 each. Read-only residue comparison shows only the five empty compiler directories
and one three-entry failed-run root from 2026-08-31, so the eighth remediation added no residue and
deleted no historical object. These changes are acceptance tooling and test-only source proof, not
an ABI, bridge, package, or product runtime change; no current-tree Release or signature is claimed.
External signed-bundle, publisher, SCM/service, named-pipe/FSCTL, retained-root, real Cloud Files, and
final accumulated-audit authority remain open.

The earlier second-remediation applicable gates passed in the ordinary-user context. The three PowerShell
scripts parse; the fresh hostile-temporary/internal-junction guardrail and 19/15/4 ValidationOnly
matrix pass; development- and release-profile formatting, check, and warnings-denied Clippy pass;
and `quality_lint.ps1` passes with 149 files unchanged and no analyzer issue. The complete serial
Daily gate reports 860 Rust library tests passed, zero failed, and 16 expected ignored in 219.75
seconds; broker binary integration passes 3/3 in 2.07 seconds; all 309 Flutter unit/widget tests,
Windows scan 2/2, native accessibility 2/2, bridge compatibility, and whitespace pass. The internal
unsigned x64 application builds in 67.3 seconds. Both packaged PE files are `0x8664`, all 81 current
Cargokit dependencies exist, the DLL hash matches, and dependency plus ASCII/UTF-16 binary scans
contain zero R2c-R test seams. Explicit absent signed inputs still fail formal Release admission
before process startup. None of this supplies an external signature, service, FSCTL, retained-root,
or Cloud Files acceptance fact.

The immutable Windows Release orchestrator remains deliberately unclaimed: it requires an exact
externally pre-signed Application bundle, matching pre-signed broker, and expected publisher before
it can verify signatures, payload hashes, packaged-process single-instance behavior, replacement,
and release bridge smoke. With explicit absent-input paths it fails closed at bundle admission with
`The Ame application bundle was not found`. This remains an implementation checkpoint pending
external release evidence. R2c-O remains the active acceptance slice. R2c-R remains a non-external
controlled local reliability checkpoint and is not accepted. This evidence does not
close R2c-O or claim
elevated installed-service lifecycle, real brokered FSCTL, separately authorized retained-library,
source immutability, or no-hydration acceptance evidence.

### Phase-26 live-gap ownership remediation

The 2026-09-01 production audit found one High continuity defect: a running-time native
`need_rescan`, observer-ingress loss, or offline-to-available transition could be rewritten as a
naked `StartupCatchUp` P1 root `FreshnessUnknown` marker. Production had no consumer for that shape,
so the root could remain updating forever even though no full scan was authorized. The remediation
keeps new live and availability gaps as durable `LiveNotification` P0 work and gives every
supersession an atomic, lineage-bound consumer.

Focused production evidence covers native `LiveOnly` `need_rescan`, observer ingress drop,
offline-to-available recovery, expired-lease restart recovery, and both journal outcomes. Each case
reaches `Synchronized`, drains the relevant queues, preserves lane, origin, scope, intent, lineage,
and consumer ownership, and records zero automatic full scans. The continuous-journal case persists
the exact source-range claim before supersession. The uncovered case creates the independent P2
control and `watcher_uncovered_gap` authority in the same transaction; a capacity failure rolls the
transaction back and leaves the original P0 retryable. Schema v30 migration tests cover the
ambiguous historical v29 fallback both with and without a persisted root publication identity,
idempotent current-catalog opening, partial-schema rollback, legacy upgrade conflicts, and fresh-
catalog validation without inventing event provenance, coverage, or authority.

This is non-external implementation and migration evidence only. It does not use retained roots,
real Cloud Files, SCM, a real named-pipe/FSCTL journal, signing, or a release candidate. R2c-R remains
not accepted and R2c-O remains the active acceptance slice.

### Phase-27 explicit manual recovery and provenance remediation

The phase-26 follow-up audit found that root publication identity had been treated as event
provenance and that explicit recovery ownership had no typed user-authorized consumer. Phase 27
therefore keeps every ambiguous naked v29 fallback as `explicit_recovery_required`; the Rust
synchronization snapshot projects it as `NeedsReconciliation`, `Blocked`, and
`recoveryBlocked=true` with `live_gap_v30_explicit_recovery_required`. Dart preserves that field and
the notification presents a clear manual-library-update explanation and accessible `更新图库`
action. It does not present ordinary persistence failures as scan authorization.

The foreground scan begin, publish, abandon, and reopen/crash paths use the exact transactional
claim transitions described above. A successful publication removes the explicit block and permits
`Synchronized`; abandon or interrupted-open recovery restores the same typed block. Focused fixtures
cover historical migration provenance, partial v30 rollback, exact validator rejection for malformed
explicit/active/consumed states, pending-journal isolation, Rust production projection, bridge-model
mapping, notification behavior, and button semantics. These are non-external implementation and
migration facts only: no retained root, real Cloud Files content, SCM, real named-pipe/FSCTL journal,
signing, or release candidate is used. R2c-R remains not accepted and R2c-O remains active.

### Phase-27 capacity deferral and real P2 visibility completion

P0 live-gap promotion can be blocked by bounded P1/P2 capacity without becoming a processing
failure. That state is now a typed `MetadataInventoryLane` capacity deferral. Persisting it
atomically refunds the lease attempt, retains a non-null bounded retry deadline, and records
`live_gap_p2_capacity_deferred`; lease expiry and process restart preserve the same non-terminal
meaning. Only the exact leased P0 root live-gap shape may use this path. Ordinary processing
failures continue to consume the configured retry budget and become terminal at the unchanged
limit.

Releasing matching P1/P2 capacity wakes a deferred gap in the same completion or finalization
transaction. A later precise P0 path event remains independent rather than being absorbed into the
capacity-wait root gap, so recovery work cannot monopolize the reserved live lane. The schema-v30
attempt, failure-code, lease, and retry fields already represent this invariant; no DDL or v31
migration is required.

A root can legitimately own more than one unretired P2 authority while an older inventory page is
retained, for example when a watcher gap arrives during a containment baseline. A retained source
therefore belongs to its immutable `LibraryChangeId`, not merely to the root. Page completion and
restart handoff return or acquire only that exact owner; catalog validation happens before handoff,
and a worker-spawn failure restores the untouched source to the same change ID. Each poll prunes only
sources whose exact authority is missing, retired, or has a different run ID. This prevents a newer
same-root authority from consuming or silently reopening another authority's live frontier while
keeping the in-memory set bounded by current durable authority.

The production native `need_rescan` fixture now starts from an explicitly published non-empty PNG
baseline, then adds one asset and removes another before exercising the real P2 consumer. It
requires exact addition visibility and removal absence, unchanged source bytes and hashes, zero
queued work, claims, and active authority, `Synchronized`, and no new automatic full-scan run.
Instrumentation permits only the bounded metadata inventory and media opens required by this
consumer. A transaction-scoped rollback mutation deliberately removes the newly published location
and proves that the former empty-fixture/queue-only contract would still pass, so the visibility
assertions close a real acceptance blind spot.

Accumulated ordinary-user evidence passes the exact L1 path and all 83 production synchronization
tests, including capacity, fairness, restart, genuine failure exhaustion, overlapping authority,
catalog-validation, and worker-spawn restoration. The first concurrent Daily run exposed only a
test-observation race: a helper ordered all root gaps by newest ID and could observe the valid P2
consumer after promotion instead of the retained P0 lineage owner. It now selects the exact
`live_notification`/`p0_live` row, so a missing P0 still fails closed. The corrected concurrent Daily
passes 927 Rust tests with zero failed and 17 expected ignored, broker integration 3/3, all Flutter
tests, and both controlled Windows integrations 2/2. The ordinary-user R2c-R runner passes 19/19,
`quality_lint.ps1` and Release-profile warnings-denied Clippy pass, and the generated bridge hash is
`941711727` on both sides.

A fresh unsigned Windows x64 application build completes in 58.2 seconds. The application,
packaged/Cargokit Rust DLLs, and broker are PE `0x8664` with `NotSigned` status; all 83/82/82 broker,
rlib, and DLL dependencies are present and current; packaged and Cargokit DLL hashes match. Seven
ownership/fault test-seam tokens are absent from six Release artifacts under ASCII and UTF-16 scans,
and ten Flutter assets contain no policy source or test seam.

This completion uses only disposable source and catalog storage. It does not access retained roots,
Cloud Files, SCM, a real named-pipe/FSCTL journal, or signing; it does not add source mutation or an
automatic full scan. R2c-R remains not accepted and R2c-O remains active.

### Phase-31 migration ownership and crash-cleanup remediation

Schema v30 continues to evolve in place because it has not shipped. The v29 migration may create an
`explicit_recovery_required` claim only for a truly naked P1 root fallback: the row has no
`authoritative_scan_id`, catch-up source or watermark, supersession target, queue or journal
lineage, recovery authority, persistent-journal baseline, or metadata-inventory candidate owner.
A configured-root publication identity remains root identity rather than event provenance, so its
presence does not make a genuinely naked row safe. Conversely, a non-null authoritative scan ID is
real owner evidence and must never be erased merely because the row otherwise resembles that
historical fallback.

Running and paused v29 foreground scans therefore retain their exact scan ID, queue row, root
generation, scan owner, and publication binding during the v30 upgrade. Reopen and resume use the
existing foreground checkpoint; successful publication completes the owned queue row without
creating an explicit claim and leaves the root eligible for `Synchronized`. Truly naked rows both
with and without a root publication identity retain the conservative phase-28 explicit-recovery
behavior. Partial prerelease v30 objects and conflicting legacy ownership remain fail closed.

The live-gap validator now treats `scan_runs.scan_owner = 'foreground'` as part of both the active
and consumed foreground-claim relation, alongside the existing scan ID, root, generation, status,
completion-time, and queue-state requirements. Current-v30 interrupted-claim discovery and
terminalization use the same owner predicate. A claim attached to an `authoritative_recovery` scan
is rejected with the typed, path-free live-gap contract error before the scan, claim, or queue row is
changed; the recovery path never broadens the admitted owner set.

Interrupted explicit foreground recovery and ordinary scan publication or abandonment now call one
transaction-local orphan cleanup helper. Its predicate retains every asset referenced by an active
location, `library_change_catch_up_handoffs`, or `library_change_scan_handoff_items`, and deletes
only assets referenced by none of those owners. The current-v30 repair performs no wider global
delete. An injected cleanup failure rolls back claim restoration, scan terminalization, location
deletion, and asset deletion together; a later reopen can retry, and subsequent reopen is
idempotent with no dangling handoff references.

This is disposable-catalog implementation and migration evidence. It changes neither the external
broker, bridge shape, schema version, nor source-media policy. R2c-R remains a non-external
checkpoint and is not accepted; R2c-O remains active. Daily, Release, the complete 19-case runner,
retained-root, real Cloud Files, SCM, named-pipe/FSCTL, and signing evidence remain outside this
focused slice.

### Phase-32 leased capacity-gap and reserved-code integrity remediation

Capacity-deferred live gaps remain P0 continuity authority, but they must not become a covering
coalescing target for new precise work. The protected shape is exact: `p0_live`,
`live_notification`, `freshness_unknown`, root scope, empty relative path, no previous path, the
typed metadata-inventory capacity code, `retry_wait` or `leased`, and no live-gap recovery claim.
Only a concurrent live-notification path intent invokes this exception. An unrelated root row, a
different status, or a forged partial shape follows ordinary coalescing. The leased gap keeps its
current lease and ownership; the precise path is inserted or merged into its own P0 row, duplicate
path evidence still coalesces, and the existing P0 reserve and fairness policy remains authoritative.

`live_gap_p2_capacity_` is a reserved failure-code namespace owned by
`defer_library_change_for_capacity`. Generic retry validation, including the transaction-local
helper used by internal callers, rejects that namespace with a path-free typed error before any
lease or queue mutation. Refund on lease expiry, exemption from maximum attempts, exhausted metrics,
and capacity wake-up no longer use a string match alone. Each requires the same exact typed gap
shape; wrong lane, scope, status, or recovery-claim state receives ordinary expiry, retry-budget,
and terminal semantics. The real typed deferral still refunds its leased attempt, survives restart,
wakes when matching recovery capacity is released, and yields to ordinary P0 work. Schema v30
already stores every required discriminator, so this correction introduces no v31 DDL.

The queue red reproduced a leased capacity gap absorbing a precise path (`superseded_count = 1`),
and the generic-retry red forged `live_gap_p2_capacity_deferred`, then observed lease-expiry refund
and nonterminal retry. Wrong-scope, claimed, and wrong-lane controls separately reproduced the
string-only exemptions. After the correction, 82/82 queue tests and 84/84 production tests pass,
including a production worker paused immediately after leasing the gap while a real created PNG is
enqueued, published, and completed before that gap leaves capacity wait. Existing restart,
fairness, retained-owner, native `need_rescan`, journal, migration, and scan controls remain green.

Accumulated ordinary-user evidence includes `quality_lint.ps1`, 56/56 focused Flutter tests, bridge
hash `941711727`, development and Release warnings-denied Clippy, the complete 19/19 internal
disposable runner, and a canonical Daily pass with 940 runnable Rust tests, 17 expected ignores,
broker integration 3/3, all Flutter tests, and both Windows integrations 2/2. The first Daily attempt
had one existing stop-deadline test time out under full concurrent load; its exact rerun and the
unchanged canonical Daily both passed, with no deadline or assertion relaxed. A fresh unsigned x64
Release completes with four `0x8664`/`NotSigned` PE images, current 83/82/82 dependency graphs,
matching packaged and Cargokit DLL hashes, and zero executable test-seam matches across six Release
artifacts and ten Flutter assets.

All production evidence uses disposable source and catalog storage. It does not access retained
roots, real Cloud Files, SCM, a real named-pipe/FSCTL journal, signing, or source-media mutation, and
it does not authorize an automatic full scan. R2c-R remains not accepted and R2c-O remains active.

### Phase-33 retained-catalog startup authority remediation

A retained schema-v23 catalog exposed one lifecycle state created before the persistent-journal
recovery-authority tables existed: a metadata inventory had already been superseded by a newer
epoch but still retained `absence_authority = 1`. The original v21-to-v22 transition retired only
runs that were still `running` or `comparing` at that migration instant. Because the retained run
was already terminal, the v23-to-v24 journal validator correctly rejected the unowned authority as
`catalog_persistent_journal_contract_unverifiable`, rolled back the upgrade, and made every later
startup repeat the same failure.

The forward migration now applies the persistence rule above both before v23 journal validation and
before a direct v24 lane/recovery upgrade. It preserves terminal issue evidence and staged inventory
entries, terminalizes genuinely interrupted legacy work, clears authority only from non-completed
runs, and leaves completed authority unchanged. The runtime repository also clears authority
atomically whenever a run is terminalized or superseded by a newer epoch, preventing a current
catalog from recreating the invalid state. No validation predicate is weakened and no schema version
is added.

The first independent Phase-33 audit found two adjacent legacy-state gaps. Newer-epoch supersession
now deletes the retired run's durable source spool in the same transaction as authority revocation
and frontier retirement. Schema v25 through v30 also share an exact-DDL-gated shrink-only repair
before their owning validators: it deletes spools only for `failed`, `cancelled`, or `superseded`
runs and clears authority only from those terminal runs when no matching unretired recovery owner
exists. Healthy current catalogs remain on the read-only validation path. A malformed schema, an
unowned active run, or any other unresolved contract failure rolls the complete repair transaction
back; completed authority, valid active authority, issues, entries, roots, assets, locations, and
queued work are not changed.

Recovery-authoritative P2 leasing is now exact-owner-affine per root. An unfinished persistent-
journal baseline makes its recorded inventory run the only eligible P2 owner. Without an unfinished
baseline, an active inventory run must lease through its own unretired recovery authority. If that
owner is not currently due or cannot be leased, unrelated P2 rows for the same root wait; P0/P1 work
and work for other roots continue. The scheduler selects with this rule and re-resolves the exact
owner inside the `IMMEDIATE` transaction before mutation. Starting a distinct unretired recovery
run for the same root fails before writes, and validation rejects an unfinished baseline that points
to a mismatched or terminal inventory run. Old-library recovery therefore cannot become a mutable
root-wide prerequisite for live publication.

Hosted PowerShell 7 also exposed a platform-probe defect before the Rust CI tests began:
`$IsWindows` is an automatic read-only variable and PowerShell names are case-insensitive, so a
boolean parameter with that name could not reliably receive the guardrail's false-platform probe.
The R2c-R and broker scripts now use `IsWindowsPlatform`, their guardrails assert the exact false-
platform diagnostic, and the digest-locked R2c-R source hash was updated for those reviewed edits.
The first dual-shell workflow attempt then demonstrated why the complete guardrail must not be used
as a shell-compatibility probe: Windows PowerShell 5.1 on the hosted Windows Server 2025 image left
compiler output in the held `Add-Type` bootstrap, so the default-deny cleanup correctly retained the
non-empty directory and failed. The workflow now invokes a compiler-free exact platform-binding
probe under Windows PowerShell 5.1, while the existing PowerShell 7 Daily job continues to execute
the complete R2c-R and broker guardrails. No residue allowlist, deletion rule, compiler bootstrap,
or destructive-fixture source audit was weakened.

The next hosted run passed that Windows PowerShell 5.1 boundary and exposed a separate cold-start
assumption in the PowerShell 7 process-tree fixture. Its three-second parent deadline included a
fresh child loading the complete common module and compiling its native Job Object helper before it
could spawn and report the intentionally blocked descendant. The hosted child was terminated before
that readiness marker; this was not evidence of a leaked descendant. The fixture-only parent budget
is now 15 seconds while the descendant remains blocked for 30 seconds and must still terminate
within five seconds after the owned Job Object is stopped. Production process deadlines, the
wrapper's 300-second default, cleanup behavior, and source audit are unchanged. The whole-file
guardrail digest was relocked after this exact reviewed edit.

Red regressions first reproduced the exact v23, direct-v24, newer-epoch, terminalization,
superseded-spool reopen, and polluted-current-schema failures. The complete migration module then
passes 73 tests and the metadata-inventory application module passes 37 tests. The new controls
prove current repair idempotence, malformed-DDL non-mutation, rollback when an active run lacks its
owner, valid-owner preservation, and v27 spool retirement before validation. A consistent online
backup of the retained 1.18 GB catalog was opened read-only,
migrated only in disposable storage through production `SqliteCatalog::open`, and reopened. Root,
asset, location, queue, inventory-run, inventory-entry, and completed-authority counts were
unchanged; foreign-key checking returned zero rows; only the one illegal superseded authority was
revoked. The original catalog and both source libraries were not changed. Repository lint passes,
including the 19-case R2c-R guardrail, warnings-denied Clippy, and Dart analysis. The complete Daily
component evidence was collected serially with one non-incremental Cargo job after the workstation's
default parallel compile exhausted its commit limit: the Rust library suite passes 954 tests with
17 expected ignores, broker integration passes 3/3, all Flutter tests pass, Windows scan and native
accessibility pass 2/2 each, bridge/whitespace checks pass, and both integrations build the current
Debug application. Eighteen focused owner-affinity regressions pass. Two disposable-directory test
fixtures also pass 200 repeated post-guard rename/restore cycles each with bounded Windows sharing-
violation handling while their active-guard failure assertions remain immediate and exact.
This is migration and startup-recovery evidence, not retained-root synchronization acceptance;
the final accumulated independent audit and hosted PR gate remain the closeout evidence. R2c-R
remains not accepted and R2c-O remains active.

## References

- [Microsoft: Change Journal Records](https://learn.microsoft.com/en-us/windows/win32/fileio/change-journal-records)
- [Microsoft: Using the Change Journal Identifier](https://learn.microsoft.com/en-us/windows/win32/fileio/using-the-change-journal-identifier)
- [Microsoft: FSCTL_READ_USN_JOURNAL](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_read_usn_journal)
- [Microsoft: READ_USN_JOURNAL_DATA_V1](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-read_usn_journal_data_v1)
- [Microsoft: ReadDirectoryChangesW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesw)
- [Microsoft: Named Pipe Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)
- [Microsoft: ImpersonateNamedPipeClient](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-impersonatenamedpipeclient)
- [Microsoft: GetNamedPipeClientProcessId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid)
- [Microsoft: QueryFullProcessImageNameW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryfullprocessimagenamew)
- [Microsoft: GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation)
- [Microsoft: WinVerifyTrust](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust)
- [Microsoft: DuplicateHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)
- [Microsoft: CancelIoEx](https://learn.microsoft.com/en-us/windows/win32/fileio/cancelioex-func)
- [Microsoft: GetOverlappedResult](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult)
- [Microsoft: Service Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/services/service-security-and-access-rights)
- [Microsoft: ServiceMain](https://learn.microsoft.com/en-us/windows/win32/services/service-entry-point)
- [Microsoft: SERVICE_SID_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/ns-winsvc-service_sid_info)
- [Microsoft: SERVICE_REQUIRED_PRIVILEGES_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/ns-winsvc-service_required_privileges_info)
- [Microsoft: Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [Microsoft: Get-AuthenticodeSignature](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature)
- [Microsoft: Register-ScheduledTask](https://learn.microsoft.com/en-us/powershell/module/scheduledtasks/register-scheduledtask)
- [Microsoft: Task Scheduler Principal.LogonType](https://learn.microsoft.com/en-us/windows/win32/taskschd/principal-logontype)

## Consequences and risks

- Normal no-change startup becomes O(1) with respect to source-root entries, while downtime catch-up
  is proportional to retained journal evidence and affected final paths.
- Ame acquires an installer and privileged service attack surface earlier than the former R9 plan.
  Protocol minimization, caller-root authorization, constrained output, signing, service isolation,
  and independent security review are release requirements rather than follow-up hardening.
- Journal truncation, recreation, disabled service, enterprise policy, unsupported storage, and
  ambiguous path reconstruction remain possible. They produce explicit recovery or `LiveOnly`
  behavior, never silent freshness or routine startup inventory.
- A first migration requires one baseline inventory for existing roots without a valid checkpoint.
  P0 remains responsive during it, and the cost must not recur after authority is established.
- Sharing one volume read reduces I/O, but independent per-root advancement and cross-root handoff
  add persistence and test complexity.
- A signed installer is necessary for the complete product. The portable ZIP remains useful but has
  a deliberately weaker synchronization capability.

## Rollback and replacement strategy

The broker is replaceable behind `PersistentChangeJournal`. A future supported Windows index,
snapshot API, or safer unprivileged journal interface may replace it only if it preserves per-root
coverage, enqueue-before-checkpoint, bounded root-scoped output, caller authorization, final-state
reconciliation, P0 isolation, and source safety.

If the broker PoC cannot obtain the required journal access without an unacceptably broad security
boundary, implementation stops before product integration and this decision is reopened. The safe
rollback is the existing live watcher plus explicit `LiveOnly` or user-requested refresh; it is not
a return to routine O(N) startup inventory and not a hidden always-running media service.

If a shipped broker version must be disabled, Ame fails closed to the cached catalog and root-level
capability state. It preserves checkpoints for a compatible repair, does not advance them, does not
publish absence from incomplete evidence, and never mutates source media. Removing the service does
not require a catalog rewrite.
