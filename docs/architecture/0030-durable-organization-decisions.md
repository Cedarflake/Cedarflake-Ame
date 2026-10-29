# ADR 0030: Bind durable organization decisions to content lineages

Status: Accepted for validation.

Date: 2026-10-24.

## Context

[R4](../product/r4-organization.md) assigns complete exact-image groups to one album, independently
records Favorites and resumes organization. Groups can be identified late, merge across roots or
split when content changes. A displayed group can also retain older membership while current
fingerprints are being verified under ADR 0028. Neither a path nor a digest is a permanent asset
identity, and retained display evidence cannot authorize a new durable decision.

## Decision

### Durable records and current projections

Keep organization data in dedicated tables in the existing catalog. Use independently generated,
opaque record and operation identifiers. Album identifiers, organization record identifiers and
asset/location identifiers have separate meanings. Do not merge assets or use a digest as a record
identifier. Foreign keys between organization tables enforce their relationships; no cascading
foreign key to a derived location, fingerprint or asset may erase user data during rebuilding.

An organization record represents a handled or explicitly bookmarked content lineage. Its bindings
retain the asset identity, source generation, physical identity/revision evidence and, when available,
the versioned exact content fingerprint. Root associations are retained separately so deliberate source removal can
retire only its associations and the records with no surviving owners. A rebuilt catalog can
reassociate a record through compatible identity and content evidence; a filename is insufficient.
Unresolved associations retain their first-unassociated time for explicit cleanup.

The association-lifetime owner records loss of the last compatible published member within the
location-removal or root-publication transaction. It checks the canonical group's complete lineage,
including members in other configured roots. A retained published location remains association
evidence when its watcher is inactive. Repeated observations keep the first timestamp; compatible
reassociation clears it for every lineage in the group. Older records without a timestamp begin at
their first observed loss, rather than an invented historical date. Root refresh materializes its
scoped group identifiers once and processes windows of 256; it does not read original media.

Schema 55 adds a rebuildable association lookup containing only records whose unassociated time
is set, indexed by canonical group and record. Database triggers maintain it with timestamp changes,
group insertion/merging and record deletion in the owning transaction. Reassociation selects only
this group's unresolved records, so an ordinary newly identified copy does not expand already
associated historical aliases or unrelated unresolved groups. The original timestamps remain the
durable values; migration backfills the lookup without changing decisions, revisions or source data.
Historical organization migrations use their admitted membership tables until schema 55 is installed;
a missing or inconsistent lookup in a current catalog is an error. The lookup is derived catalog data
and is excluded from portable organization archives.

Current compatible fingerprints project complete groups across configured roots. Without such a
fingerprint, an admitted asset version is a provisional singleton. Several durable records may
contribute provenance to one current group; they are not several confirmed album memberships.
Group transitions reconcile these records and preserve their stable references for continuation
and undo. A new identical copy inherits the group's decision. Changed content requires a new
binding and cannot inherit the old record solely through its asset ID, location or path.

Persist flat albums, independent favorite decisions, organization state, conflicting album choices,
field-specific decision revisions and operation receipts. A conflict preserves its competing
choices while the group has no confirmed album and remains deferred. Continuation stores a logical
record/member anchor, owning scope, queue ordering and fallback position, separately from handling
state. An ordinary album command does not replace the organization-mode anchor.

Separate stable lineage records from their current group mapping. Each record maps directly to a
canonical record; chains and cycles are invalid. Keeping lineage references allows later group
reconciliation to retain operation provenance without exposing several memberships for one group.
Album deletion retires its definition and moves assigned records to ignored in the same transaction.
The retired definition remains outside ordinary album queries so version-checked undo can restore
its name and assignments without inventing another album identity.

Explicit source removal forgets that root's organization ownership in the catalog removal
transaction. Retire all lineage records of groups with no surviving configured source, while
preserving a shared group's records and choices. Ordinary loss of derived locations is not this
authority. When a complete parent import replaces covered child registrations, transfer their
ownership to the admitted parent before child retirement; cancellation or failed publication rolls
back the transfer with the catalog. Album definitions remain available even when their last group
is removed.

### Group evolution and publication

The canonical record owns the current album, progress and favorite state. Alias records retain
stable provenance; readers resolve their direct mapping before interpreting membership. Explicit
whole-group actions record the canonical previous value for every affected lineage. Automatic
merges update the canonical state without manufacturing user operations or rewriting all historical
aliases for each newly identified copy.

One compatible album choice survives a merge. Different album choices, or an existing unresolved
conflict, produce a deferred group with no confirmed album and retain the competing choices and
their decision revisions. Favorite intent follows its latest field revision independently; equal
revisions prefer a positive favorite. With no album or conflict, the latest organization decision
survives. Retiring a conflicting album retains its provenance and does not resolve the conflict.
Removing a confirmed album ignores the complete current group.

Reconcile at the existing bounded and terminal fingerprint-publication transactions, and when
catalog publication reuses current evidence. Individual fingerprint writes do not publish group
changes. The first handled member of newly confirmed content binds all already verified copies,
including copies identified before that member; later copies use indexed association lookups.
Changed generations have no inherited membership until compatible current evidence establishes a
binding. An independent new content version remains pending, while a match to another known group
adopts that group's decision. Identity-proven renames retain their compatible binding.

Publication can also recover a durable binding after derived asset identities or source generations
are rebuilt. Use the existing Windows metadata-reuse boundary: the same volume-qualified native
file identity, canonical ChangeTime token and byte length. A dedicated indexed owner performs this
association inside the catalog publication transaction, without original-media reads. Different
known exact fingerprints for that native version, or a compatible current fingerprint contradicting
its retained evidence, disable metadata association; ordinary exact-content publication can still
resolve the supported content key. Missing native/version evidence cannot be replaced by a filename.
Source removal deletes the durable records and their index entries, so later reimport has no recovery
authority for those deleted decisions. Source reconciliation gives all current physical aliases the
same generation; recover that complete generation across roots. Deduplicate native keys and canonical
groups before comparing current and historical bindings, including the already-associated fast path.
Schema 53 adds the native-version lookup index without changing records or requiring a rescan.

Schema 54 retains portable recovery evidence and foreign-to-local generation mappings separately
from active locations. Imported field markers identify backup intent until a local command changes
that field. During later merges, an explicit current local choice takes precedence over imported
intent, independently for organization and Favorite state; conflicting imported album assignments
retain the existing deferred-resolution behavior. Recovery admissions retain original queue order
without treating foreign ordinals as live positions. Indexed links expose a proven earlier position
to current queue queries. Pending R3 choices retain their physical-copy evidence and source ownership.
The [portable archive decision](0031-portable-organization-archives.md) owns staging, revision
mapping, atomic application, repeated requests and re-export compatibility.

Schema 49 normalizes previously stored content bindings and already available fingerprints once,
without original-media reads. Shared reconciliation reads the transaction's schema version: before
schema 54, every existing decision has local authority and portable archive provenance is unavailable.
After schema 54, missing provenance or recovery tables are errors, not a legacy fallback. This keeps
historical migration independent of tables installed by later migrations. Generation lookups reuse
the existing exact-source index; the
root/scan/generation index supports bounded keyset windows. Schema 50 retires the redundant
generation index added in schema 49 without changing records or catalog revisions. Temporary
member/group sets drive joins into durable history; their
lack of persistent planner statistics must not turn one-copy publication into a history scan.

### Collection queries

Gallery queries carry a typed library, Favorite or album collection alongside source, folder,
search and ordering scope. Collection identity participates in the query hash and client equality,
so old cursors and asynchronous results cannot cross collections. Albums always fold their current
groups; Favorites retain the selected gallery display mode. The same relational scope supplies
pages, counts, layout and anchor resolution inside the existing read snapshots. A missing or retired
album is an unavailable scope; an active album without members is an empty result.

Collection reads resolve compatible asset-version bindings to canonical organization records.
Published or retained gallery fingerprints can provide a display-only fallback through indexed
content bindings. This retains a card during verification without admitting a command on pending
evidence. Confirmed changed content leaves the old collection. Valid durable bindings survive
derived-fingerprint cleanup, and their canonical mapping still folds the collection. Representatives
honor the existing preferred copy within the physical query scope, then its smallest location ID.
Representative selection resolves the compatible display evidence generation before joining its
stored fingerprint, avoiding materialization of unrelated historical evidence. Current and retained
display authority continue to use the same compatibility predicate.
Only locations named by an existing preferred-copy decision need that representative-evidence
lookup. Candidate membership does not establish preference: the current content key and asset
identity still have to match. The candidate set belongs to each SQL statement's read snapshot.
Schema 59 adds a partial index over preferred location and asset identifiers. Point admission
therefore remains independent of unrelated preference/ignore history, without storing another
decision or changing its lifetime. Existing migration backup and atomic validation apply.
Folded album and Favorite queries join each compatible binding to its canonical record once,
retaining the indexed content fallback for copies without a direct binding. That record supplies
both collection membership and the group key. Preferred-copy evidence is joined after membership
resolution; source/folder/search predicates still constrain the members before grouping. Library
queues retain their provisional content/version keys for groups without organization records.
The gallery filter owner applies source, folder, search, collection and queue constraints before
organization-group representatives are selected. Page, layout and continuation reads reuse that
scoped result without repeating the same membership predicates outside it. Shared progress counts
aggregate canonical groups directly: every member of a group has the same disposition, so counting
does not depend on preferred-copy selection or its decision table.
Chronological collection and queue timelines retain the physical member count while selecting
scoped representatives. One statement-local materialized relation supplies both representative
membership and the physical total, preserving existing membership semantics across roots. Date
buckets reuse that result instead of resolving membership a second time. Page, continuation and
timeline reads share the group-selection owner; name/admission sections and unfolded Favorite
counts retain their existing paths.
Album, Favorite, deferred and ignored reads, including location anchors, first test for a matching
durable record through a one-row relation in the same SQL snapshot. Its absence avoids location traversal;
its presence retains the complete scoped membership and representative checks. Pending queues
continue to include unrecorded groups. This proof is recomputed for each read, without a result cache
or changes to shared progress counts.
These reads use catalog data only and never load the membership graph into presentation.
Shared progress counts resolve compatible bindings and canonical records once for both group
identity and disposition. Unbound copies retain the published-content fallback; unrecorded groups
remain pending. Counts keep the collection, root, folder and search scope while spanning all queue
states in the same read snapshot. This aggregation does not consult preferred-copy choices.

Position-preserving refresh resolves the requested physical copy, then its stable asset, before
falling back to the retained ordinal. The fallback reads that ordered position directly and carries
its proven ordinal into the centered window. Only an out-of-range request requires a total count
and clamping to the last eligible item. Identity resolution, fallback and page publication share
one transaction, scope and ordering; retained ordinals are not authority across transactions.

Point-anchor reads establish logical collection/queue membership before expanding the complete
scoped group relation. Conditional SQL retains that evaluation order when a location or asset is
already ineligible. Representative lookup checks the requested active copy's logical membership
without requiring that physical copy to pass the folder/search filter; the final group selection
still chooses a qualifying member inside the complete physical scope. These eligibility reads reuse
the published-source and empty-decision relation and share the enclosing read transaction.
When representative resolution succeeds, the cursor read retains the point scope, including root,
folder and membership constraints, because different published roots can share a location ID. It
does not repeat the complete representative relation. The proof remains local to the same read
transaction; unresolved locations and stable-asset fallback still apply the full anchor filters.
Window positioning and subsequent page reads retain their scoped ordering checks.

### Queue admission and continuation

Allocate a monotonic queue-admission ordinal when a source version becomes part of a published
catalog transaction. Source generations identify versions but are allocated before publication;
they cannot by themselves order concurrently staged imports. Reusing a published version retains
its ordinal. An upgrade establishes a deterministic baseline for already visible generations;
it does not invent an unavailable historical publication sequence.

Keep version admissions and their configured-source ownership in compact catalog tables. Source
associations use catalog-local integer root keys behind the adapter, with foreign keys to the
configured root identifiers. These keys are neither domain identities nor export identities.
Explicit source removal forgets its unshared admission history; parent consolidation transfers
that history before retiring covered registrations. Ordinary source unavailability retains it.
Publication and source-lifecycle transactions own these writes, with no startup source scan.

The queue projection uses current group evidence and these admissions; an identical copy does not
become a new independent task after identification. Resolve the group's earliest surviving
admission through its current version, durable lineage bindings and published exact-content history.
Unpublished fingerprints and staged versions cannot supply an earlier position. The selected
physical representative and source-folder scope do not replace the global group's admission order.

Keep the queue selector alongside the existing collection and physical scope in the typed gallery
query. Queues always fold logical groups, including provisional asset versions, and share the
ordinary gallery's selected ordering expressions and deterministic tie breaker. Library-admission
descending remains the explicit `新加入优先` choice. Ordinary gallery sort preferences remain
separate. Chronological queue results retain the same date buckets as ordinary browsing. Pages,
layout and anchor resolution use the same ordering expressions; query identity includes the queue,
sort key, direction and optional current-entry admission boundary.

Name and admission sorts publish compact scroll sections in the same query snapshot. Name sections
aggregate contiguous natural-key prefixes into letters, numeric prefixes and the symbol ranges
before and after the alphabet. Admission sections compare each group's retained earliest admission
with a read-only high-water mark captured on mode entry. This ephemeral boundary stays in the active
browse query, survives workspace visits and continuation, and is not durable organization progress.
It requires no schema migration or historical admission dates. Re-entry captures the current
high-water mark; new copies of an older group remain outside the new-arrival range.

Read all four progress counts within the page/timeline snapshot. Counts cover each logical group
once within the retained collection, folder and search scope, independently of the selected queue.
Only the bounded page and scalar counts cross the desktop bridge. Read-only retained evidence may
preserve a visible group during verification; it still cannot authorize a durable command.

Durable continuation remains separate from handled state. Keep one active review session and one
last committed bookmark, each with its own retained collection, source, folder, search and queue
scope. Entering a session replaces active save authority without replacing the bookmark or creating
activity. An explicit image-position checkpoint establishes activity; opening the mode alone does
not. A checkpoint may materialize the current pending lineage without completing it.

Schema 57 persists each scope's sort and the bookmark's missing/text/number/root/location boundary
alongside its admission reference. Existing bookmarks migrate to library-admission descending.
The bookmark stores a stable lineage reference, not an ordinal into the current page. Resolve a
still-eligible lineage through its current canonical group and scoped representative. If already
handled, resolve its current ordering boundary and continue after it. A current same-scope request
can override the saved sort; an unavailable anchor cannot supply a boundary for a different sort.
An unassociated anchor stays recorded and returns a distinct issue with an
optional next eligible target. Empty scopes and exhausted suffixes have distinct results. Catalog
association does not establish filesystem availability; the existing viewer source checks retain
their authority and a failed open does not complete or discard work.

Session identifiers and monotonically increasing position sequences reject obsolete checkpoints.
Changing the current group in review mode requires its saved session/position authority and commits
the decision, after-current bookmark and operation receipt in one transaction. Retrying a committed
review operation returns its original receipt without rewriting a later bookmark. Ordinary
album/Favorite operations retain the review anchor. Explicit source removal retires scoped bookmarks
and orphaned lineage anchors; parent consolidation transfers the retained physical scope. Reopening
starts a new session authority while retaining the last committed work.

### Command admission and transactions

Application commands validate typed selection identities and bounded inputs before entering the
repository. The repository resolves the selection in one snapshot to current whole groups,
deduplicates repeated members and selects the current global representative, honoring a valid R3
preference. R3 visible selection, folder scope and physical-location inspection retain their rules.

A prepared command carries the catalog and organization revisions and selected asset/location/
generation identities. Commit rechecks these authorities inside the write transaction. A changed
representative, membership, content version or intervening decision rejects the stale command;
the client refreshes its proposal. A pending retained display relationship is not a confirmed
singleton and cannot admit a group write. Resolution and virtual mutations perform no filesystem
reads, scans, decoding or hydration.

Related album, progress, conflict, receipt and optional continuation changes commit together. Only
the committed outcome can publish counts or advance the viewer. Favoriting has its own revision
and cannot change album/progress. Undo checks the affected field revisions and compatible current
bindings, restoring only the receipt's changes. Repeated operation identifiers have idempotent
results; they must not authorize a different payload. Queries return bounded pages and scalar
counts, never the entire membership graph to Flutter.

Record album metadata changes and previous conflict choices with the operation. Keep an opaque
target receipt even when explicit record removal cascades its field history; a missing target
rejects the entire undo instead of restoring only surviving groups. Undo cannot restore an album
that another operation retired, discard later members when undoing album creation, or overwrite a
later name or field revision. Failure rolls back the undo receipt and every related write together.
If a later merge introduces a separately handled lineage outside the original operation's targets,
reject that undo as a whole. Newly identified unhandled copies reuse the existing lineage and can
follow a still-valid whole-group undo. Resolving and then undoing a conflict restores one deferred
group and its competing choices, never several confirmed memberships.

Chronological undo and redo retain a transactional history cursor and one current inverse receipt
per history entry. New ordinary writes truncate the redo branch while retaining immutable operation
receipts. Reversing a step updates its inverse receipt and cursor in the same transaction. Historical
schema upgrades start a new cursor without inventing the causal order of older direct undo calls.

Field and album before-images capture their effective revision through indexed authority records.
Ordinary writes introduce a new authority; an inverse records the authority restored by its
before-image. This allows consecutive undo/redo on one field while its physical revision continues
to increase. Unrelated writes, content changes and independently handled group merges still fail
the existing guards. Older receipts without before-version evidence retain strict single-step
validation. The cursor read returns only the current undo and redo identifiers and revision.
Cleanup and archive replacement retire the chronological branch without deleting retained receipts.
Schema 56 captures authority only for newly admitted local before-images. Archive imports retain
their historical snapshots without inventing missing before-version evidence and begin a new history
epoch after their atomic application. Historical archive formats remain readable without a new
portable cursor or authority format. A history step rechecks the requested direction, head and
revision inside its transaction; unknown outcomes replay the same operation identity.

Desktop publication retires a deleted album's active and retained browsing scopes before reading
the committed catalog. A revocable presentation projection owns that navigation and organization
session transition; the application owns serial publication and retry obligations. Failed or
superseded projection work cannot satisfy a replacement window's obligation. Retry publishes the
committed state without replaying the deletion, and ordinary record changes retain their existing
position-preserving refresh.

### Upgrade protection

Before an existing catalog advances to a newer schema, create an exclusively owned SQLite online
backup beside the catalog. This captures committed WAL data without copying original media or
opening source directories. Fresh and already-current catalogs do not create a backup. Use the
existing rusqlite backup feature in production; no new package or license is introduced.

Copy in bounded page steps, reject persistent busy/error states, validate the completed snapshot,
flush it and retain it before starting migrations. Each copy uses 256-page steps and a 60-second
total copy deadline. A flushed `.complete` receipt records the source and target schema versions;
an absent, incomplete or mismatched receipt does not qualify a snapshot for recovery. This deadline
is an upgrade-copy failure boundary, not a gallery startup performance target.
A failed or incomplete backup cannot authorize
an upgrade. Its temporary output is owned by this attempt; unrelated files and previously completed
backups are never overwritten or automatically deleted. An unsuccessful migration leaves the
completed pre-upgrade snapshot available. Recovery opens a verified copy in separate derived
storage before any explicit catalog replacement; it never restores over an active database or
silently discards newer decisions.

### Ownership and presentation

Domain types and transition policy belong to an organization module; application use cases own
validation, progress and publication. Narrow repository ports own transactions through dedicated
SQLite organization and migration owners. The existing catalog/session and desktop bridge remain
composition facades. Each new responsibility retains focused tests beside its owner.

The existing Material desktop shell, controls, colors, spacing, focus, notifications and motion
remain authoritative. The sketch supplies layout and flow only. Final R4 requirements override
obsolete prototype controls, including multiple-album choices and a missing ignored queue. No
mobile adaptation or independent design system is introduced.

Read current organization decisions through a separate bounded, read-only inspection contract.
Opening a viewer does not prepare a write, hash group membership or create a lineage record. Pending
group publication returns an explicit unavailable decision instead of a default unhandled value.
Write preparation remains at the user action boundary. Unknown write outcomes retain their operation
identifier and prepared authority for idempotent retry; confirmed stale authority requires a new
preparation and operation identifier. Refresh failure after commit retries the read independently.
The action returns its saved receipt when the write commits. Gallery publication continues under
its serial owner, so a form's save and cancellation lifetime never includes the subsequent read.

Presentation keeps one active organization write and retires pending preparation on disposal.
The existing viewer notice renders results. A bounded organization outcome owner retains only the
latest successful receipt and rejects superseded undo actions; routine successes do not enter
notification history or unread state. The shared notification owner retains actionable failures.
Transient display lifetime is shared with ordinary notifications. Query-local viewer positions
carry their catalog/query authority and are revalidated through bounded anchor reads after group or
collection changes, preserving the currently displayed source. A fallback ordinal cannot prove that
an image still belongs to a collection.

Album navigation and destination dialogs read 64-album keyset pages. A separate indexed read resolves
one saved default or selected album by ID; finding it does not traverse the list or create an
operation. Retired albums are absent from both read contracts. Prompt and default-destination
preferences use the existing durable presentation store, with prompting enabled and no default on
first use. The destination proposal is read-only and leaves conflicts for an explicit choice.

Each album dialog owns its pending command. An unknown receipt retries the same operation and
prepared selection; changing the input creates a new command. Closing the dialog retires unfinished
preparation, and failures remain in that form. Metadata-only operations retain undo even when they
change no groups. Successful receipts use the existing gallery notification surface or viewer top
notice without entering history. Gallery selection, query publication and viewer position retain
their existing owners; the screen composes the album controls rather than owning their transactions.

Gallery-wide actions use an opaque, cancellable selection session instead of expanding a full query
into the single-image command's 256-location bridge payload. Upload explicit inclusion or exclusion
IDs in bounded chunks. The adapter retains those IDs and deduplicated whole-group targets in
connection-local SQLite temporary tables backed by bounded caches, outside original-media trees.
No transaction remains open while a destination dialog waits. The existing gallery predicates own
visible query membership, including folded representatives, folders, search and collection scopes.

Preparation returns scalar counts and common-assignment information. Commit revalidates the observed
catalog/decision revisions and each group's content/membership authority, then applies all selected
groups under one operation and one transaction. Cancellation or a later failure rolls back the whole
operation; receipt replay remains independent of later catalog changes. Progress and cancellation
are session-owned, and closing or dropping a session releases its temporary data. Pending selections
are disposable; albums, decisions, history and saved continuation remain durable.

The batch connection retains at most 64 compiled statements for repeated group inspection,
membership staging and decision writes. Every execution rebinds current parameters and reads current
transaction state; only SQLite's compiled program is reused. Temporary membership/group tables are
cleared between targets. Schema changes, stale authority, cancellation and rollback retain their
existing checks and transaction ownership.

Creating a destination inside the picker may refresh a ready batch's authority without rerunning
its gallery query. Recheck the frozen group set against current representatives, content/membership
evidence, effective decisions, canonical record IDs and field revisions in one read snapshot.
Only unchanged targets receive current catalog/organization revisions. Newly matching results stay
outside the original selection; an intervening target decision or content change requires reselection.
Refresh is unavailable after a write attempt, so an uncertain receipt keeps its original digest and
operation ID for replay. Decision-version evidence shares the existing group inspection query.

The gallery selection owner uploads bounded inclusion/exclusion chunks only on an explicit action.
It releases pending preparation when selection changes or the toolbar is disposed. Once the album
picker opens, that dialog owns the batch independently of toolbar publication; creating an album
cannot retire its own pending assignment. The shared picker exposes save cancellation only when
its membership command supports it, waits for retirement, and preserves a commit that wins the race.
Ordinary favorite and unfavorite successes use the same transient receipt and undo path as the viewer.
Direct notification undo and chronological history steps both return the committed operation and
any currently retired album. Their shared receipt projection runs inside the write transaction,
including operation replay. Publication clears active and retained scopes for that album before
refreshing gallery results; a later album restoration prevents stale replay from retiring it again.

Organization mode has a typed presentation owner for its retained ordinary query, selection and
visible anchor. It composes the existing query transition and viewer rather than adding another
gallery. Entry and active query changes admit only the Library collection. Source/folder scope is
preserved by the existing bounded query; album and Favorite browsing cannot enter organization mode.
Continuation admission compares the saved source, folder and descendant policy to the active scope
before offering or restoring it. Incompatible historical collection positions remain stored but
cannot reopen organization mode in a collection or replace the selected source scope.
Workspace navigation retains an organizing Library's active browse snapshot and ordinary return
context while Albums uses the gallery. Query restoration precedes review-session reactivation on
return. Duplicates and Settings retain the active organization session behind their surfaces;
returning to an ordinary gallery resets selection and position across all primary destinations.
Mode changes and workspace navigation share an exclusive transition owner. Navigation waits for
an admitted mode change before capturing its state, blocks new mode commands during restoration,
and leaves the review session under its own navigation authority. A pending entry therefore cannot
activate behind an already published Albums workspace. Disposal retires destination publication.
Progress counts carry their query identity and catalog revision; only a matching snapshot
can supply the header and review controls. Exiting restores the retained browsing context through the existing
query/selection admission rules, so obsolete all-matching selections cannot authorize new writes.

`LibraryReviewToolbar` owns the current-queue dropdown and labeled exit/continuation controls.
`LibraryGalleryHeaderActions` owns the retained secondary row, shared animation progress and
reduced-motion behavior. `LibraryGalleryHeaderRows` measures four bounded cells and interpolates
the complete header geometry, aligning title/count with their corresponding action rows and
avoiding a minimum-height plateau before gallery movement. Desktop layouts retain title/count and
action columns with horizontally scrollable actions when constrained. Height-only viewport changes reuse existing photo
rows and apply the established viewport anchor in the same layout frame. They neither queue another
geometry publication nor restart photo reflow; width and row-layout changes retain their existing
motion ownership. The retiring row loses pointer, keyboard and
semantic input immediately. `ameMenuChoice` supplies the same dot marker and exclusive-selection
semantics to sorting, layout and organization menus. Material buttons and popup items retain
focus, selection and accessible names. The pinned SDK's `AbsorbPointer` removes pointer-related semantic
actions during a mode operation without repainting the controls as disabled; the existing mode
guard also rejects repeated keyboard commands. Published empty content remains mounted during
query replacement. `LibraryGalleryLoadingRegion` owns the optional feedback delay and cancels its
timer when loading ends or the region is disposed. Query and mode composition selects the delay;
direct positioning remains immediate. These presentation changes do not alter query admission,
progress publication or persistence authority.

`LibraryGalleryScrollSurface` composes existing date navigation for any chronological query and the
same annotated rail with compact sort sections for name and admission ordering. Organization mode
does not choose a separate rail. `LibrarySortRailProjection` maps section ordinals through query-wide
row geometry, with proportional positions before complete geometry is available. `TimelineRailRanges`
paints the admitted range without owning input. Unannotated positions retain `TimelineRailTicks`;
date and positional annotations share `TimelineRailMarker`. Decorative annotations have no input
actions and do not add Slider divisions or synthetic date buckets.
The existing Material Slider owns track hit testing and gestures; the surface translates its
normalized values through the gallery's `ScrollPosition.drag` lifecycle. This preserves the drag
notifications used by loaded-window pagination. Query/controller changes and disposal cancel the
active drag and retire prior callbacks. Both bounded gallery renderers retain their current visible-range
and page-loading owners. No second scroll position or full-library payload is introduced.
Passive continuation text is omitted. `LibraryReviewModeNotice` preserves mode
failures and retry through the same top notice used by the viewer, below the gallery header.

A separate review session owns the current image entry and lazy native session creation. Opening
the mode or changing its queue does not create activity. An explicit image visit checkpoints its
logical target; an admitted representative replacement renews that authority without duplicating
the same target checkpoint. Allocate position sequences with a reserved intervening value for a
commit still in flight. Unknown checkpoint or write outcomes reuse their original position and
operation identities; a newer image retires the previous entry even if its result arrives later.
Ending a session also retires a begin operation that completes after the surface is disposed.

Album and progress actions use the atomic review command and advance only their still-current,
committed entry. Favorites retain the ordinary independent command. Continuation restores the saved
scope and logical target through a bounded query, never a guessed queue ordinal. Closing the viewer
retires pending navigation intent. Background continuation reads run one at a time and coalesce
superseding catalog publications; they update resume availability without moving the current image.
Review failures use the existing viewer notice and retry path, while confirmed results retain the
shared transient receipt and undo behavior.

Conflict inspection has a separate read-only port. Album provenance uses 64-entry keyset pages,
including retired album names without admitting them as destinations. A bounded visible window
reads conflict markers in batches of at most 256 locations through the existing display-binding
projection. These markers grant no write authority; opening the picker revalidates the selected
content through the organization command boundary. A scalar status query counts conflicting
canonical groups that retain compatible configured-source bindings, independently of the current
gallery filter. It does not read media or materialize unhandled decisions.

Background status reads serialize and coalesce catalog publications. One shared notification points
to the global deferred queue without changing current browsing until its action is selected.
Dismissing that notice leaves per-image conflict labels and viewer resolution controls available.
The existing destination picker displays provenance, retains explicit destination choice and uses
ordinary or atomic review membership commands according to the active mode. Resolution and undo
keep their existing transaction and continuation owners.
The picker retains the framework dialog and radio selection. Its scrollable body keeps actions
reachable at compact desktop heights and enlarged text. Provenance and destinations each render
only their bounded 64-entry page; the framework scroll views and shared motion owner provide
scrolling without intrinsic measurement of a lazy viewport.

## Alternatives and consequences

Content-keyed decisions alone simplify queries but cannot retain provisional intent and conflict
provenance safely across group evolution. Per-location album membership contradicts whole-group
operations. A separate decision database avoids catalog cascades but requires cross-database
publication/recovery coordination; dedicated durable tables retain one atomic boundary instead.

Revision checks can reject a proposal after an unrelated catalog change. Initially prefer this
conservative retry over accepting stale writes; any narrower group authority needs equivalent
race tests. Schema upgrades pay a one-time catalog backup cost and require sufficient derived
storage. Normal startup and browsing do not pay this copy or enumerate original media.

## Validation and replacement

Validate cross-root and unfolded target equivalence, pending-evidence rejection, stale membership
and representative races, atomic failure, reopen persistence, old-schema preservation and backup
recovery. Later group-lifecycle tests must prove conflicts, splits, shared-root removal and undo;
bounded queries and real desktop flows retain the R4 plan's exit conditions.

Reject this design if stable intent cannot survive evidence rebuilding or if group changes require
unbounded UI payloads or source reads. Replace bindings/projections behind the repository port with
forward migrations that preserve records and receipts. Rollback uses a verified retained backup
and compatible application version after explicit reconciliation of any decisions made since it;
never downgrade a live schema in place.
