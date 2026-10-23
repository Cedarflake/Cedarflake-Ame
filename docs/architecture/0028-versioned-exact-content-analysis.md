# ADR 0028: Version exact-content evidence behind a guarded reader

Status: Accepted for validation.

Date: 2026-10-16.

## Context

R3 identifies byte-identical cataloged images without another directory scan or source copy.
Windows file identity and ChangeTime establish which source version may be read; they do not
establish equality between different physical files. R2c already owns catalog publication,
monotonic source generations, hardlink invalidation and held, no-recall Windows source access.

The MIT `czkawka_core` 12.0.2 is reusable independently of the GPL-3.0-only Krokiet UI. Its
[duplicate pipeline](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/src/tools/duplicate/core.rs)
owns directory traversal, size grouping, hashing and cache persistence. Its public
[hash entry point](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/src/tools/duplicate/mod.rs)
opens a path itself; its internal stages do not accept Ame's already-admitted reader. Its
[manifest](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/Cargo.toml) also includes
unrelated image, audio, video and archive dependencies. Embedding that pipeline would require
adapting source admission, candidate traversal, cache validity and mutation dispatch.

## Decision

Use the existing `blake3` 1.8.6 dependency through an Ame-owned content-reader port. The dependency
offers CC0-1.0, Apache-2.0 and Apache-2.0 WITH LLVM-exception alternatives; retain the existing
distribution notices. Pin the existing version so recorded engine provenance matches the build. No Krokiet UI, assets or
implementation is copied. Czkawka remains an independently evaluated candidate for later
perceptual comparison; this choice does not admit that future dependency.

The exact fingerprint scheme is `blake3-256-full-v1`: the complete encoded file, no image decoding,
normalization or metadata removal. Parameters are `full-file;output=256`. Matching requires both
byte length and this complete fingerprint. Partial hashes, if later measured useful, can only
eliminate candidates and must never establish a group.

Each execution has immutable run identity, engine/version/parameter metadata and start time.
Terminal outcome is appended separately. Evidence records its run, source generation and scheme;
later algorithms append another scheme rather than overwriting old evidence. Compatible complete
results survive cancellation, failure and restart. Human decisions are stored separately.

## Reading and publication

- Read candidates from the published catalog in bounded keyset windows. Prune sizes with fewer
  than two active locations and skip compatible fingerprints. Do not enumerate source directories.
- Hash one file at a time with a fixed 256 KiB buffer; check cancellation between reads. Files larger
  than the existing 256 MiB original-image read limit remain visible and unclassified. This bound
  also limits the duration of a source namespace lease; expose skipped files for retry.
- Reuse the root-relative Windows guard from ADR 0007/0009. Retain the guarded file and namespace
  handles through fingerprint publication, revalidate the same handle after hashing, and reject
  placeholders, changed identities/revisions, changed roots and unsupported reparse paths.
- Perform source reads outside database transactions. The short publication transaction checks
  active scan, root generation, location identity, size, revision and source generation again.
  Cancellation and stale completions cannot replace newer evidence.
- Evidence is keyed by the catalog-wide source generation. Hardlink aliases share one content
  read while every admitted path remains a distinct inspectable location. The existing atomic
  generation invalidation removes changed aliases from confirmed results immediately.
- Authorized recovery may retain a generation under the accepted Windows metadata-reuse boundary
  in [ADR 0024](0024-windows-change-driven-library-continuity.md#windows-metadata-reuse-after-lost-journal-coverage).
  This preserves an already complete fingerprint; it never synthesizes content evidence from
  metadata. Observed dirty events, changed properties and incomplete evidence retain invalidation.
- Exact groups are query projections over current locations and compatible evidence. They do not
  merge asset IDs. Unavailable roots retain last-known locations; unavailable content is never
  opened or downloaded merely to complete analysis.
- Gallery membership uses a separately published projection of complete fingerprints. During a run,
  publish accepted evidence after one second or 256 sources, whichever comes first, in a bounded
  transaction that advances the catalog revision once when membership changes. The synchronization
  snapshot observes that revision and the existing query-publication owner coalesces gallery reads.
  A terminal transaction also admits retained complete evidence, including interrupted-process work.
  Individual fingerprint writes cannot change gallery counts or cursor membership. Batch publication
  retains active-run authority and never bypasses source identity or revision admission.
  Schema 41 admits existing complete evidence without rereading sources; source
  identity/revision checks still invalidate changed or removed locations immediately. A subsequent
  zero-read analysis may observe that revision; it cannot advance the revision again unchanged.
  After the native analysis stream retires, the application requests a committed gallery refresh.
  The existing query-publication coordinator preserves newer user queries and viewport anchors;
  overlapping analysis completions coalesce behind the active refresh. Zero-read terminal runs
  also refresh because they can admit evidence retained by an interrupted process.

### Retained gallery membership during revalidation

Schema 45 separates last-confirmed display membership from current content evidence. When dirty
recovery advances a generation despite an unchanged physical identity, size, modification time and
revision observation, the source-reconciliation transaction may retain the prior published content
key for gallery display. It binds that relationship to the new generation before updating aliases.
Changed identity/state does not receive a binding. A binding never creates a fingerprint, skips a
content read, admits a durable preference, or enters the confirmed Duplicates review projection.
Incremental replacement captures compatible membership before removing its prior location and binds
it after guarded source reconciliation, within the same transaction. The location-mutation owner
keeps removal, current/staging publication and preview ownership together; rollback restores them all.

The gallery projection combines current published fingerprints with those retained relationships.
Current publication wins; a different confirmed digest splits only its affected member, and a new
confirmed copy joins the existing displayed group progressively. All gallery counts, paging,
timeline/anchor resolution, annotations and location inspection use that same projection. Pending
membership exposes the group's current update, pause or background-wait state without per-location
verification labels; cancelled or restarted analysis retains this state without claiming completion.
New items without previous evidence remain ordinary
items. Source removal and scope changes still exclude absent/out-of-scope members.

The existing source-generation invalidation and immutable exact fingerprints retain their strict
contracts. The derived binding is replaced on the next physical-generation transition, and retired
bindings are discarded in batches of at most 256 during fingerprint publication. Gallery location
inspection reads counts and one keyset page in one snapshot, releases it, then inspects availability.
Badge annotations are bounded current-membership reads: the requested gallery revision is a lower
bound, so a newer committed publication does not hide their inspection action while the visible
window catches up. Each read validates current locations and compatible display evidence in its
transaction; removed locations yield no annotation. The presentation retires responses when its
requested window changes. Cursor membership and exact-analysis admission retain their stricter
revision and source-generation contracts.
No source content is opened by this display projection. The empty forward migration preserves
catalogs and user decisions without inventing historical relationships or rescanning. Disabling the
retained projection restores confirmed-only display; the additional derived table is disposable.

This choice retains browsing continuity during conservative recovery instead of expanding every
previous group while hashes are temporarily unavailable. It deliberately allows visibly pending
display relationships, while confirmed analysis and user-decision admission continue to require
current compatible evidence.

## Group and query semantics

A group key contains scheme, byte length and digest. It is content evidence, not asset identity.
The location count counts distinct catalog locations. The physical-file count counts distinct
source generations: two hardlink paths count as two locations of one physical file. Copies on
different volumes remain separate physical files even if their platform file IDs happen to match.

Global evidence determines whether an item belongs to a duplicate group. Gallery scope still
limits which locations may be represented or selected. Folding chooses the user's preferred
location when its current identity and content evidence still match within that scope, then the
lexicographically smallest location ID within it;
sort/date keys come from that representative. A group spanning folders does not silently expand
selection outside the active folder. `查看重复位置` deliberately lists all known current locations,
including those outside the gallery scope. Pending and unreadable items remain ordinary items.

These selection rules own R3 browsing, physical-location selection, and duplicate preferences.
The planned [R4 virtual-organization commands](../product/r4-organization.md#albums-favorites-and-duplicate-groups)
resolve a selected item to its complete current group across folders. That is an explicit command
target, not an expansion of the visible gallery selection or permission to mutate source files.
R4 group normalization does not change the existing R3 selection and representative rules.

Schema 42 stores preferred-copy and ignore decisions under the versioned content key, with an
explicit location/asset pair for the preferred copy. These rows have no cascading relationship to
rebuildable fingerprints or locations. Compatible reanalysis and added copies retain the decision.
Removed or replaced preferred locations fall back deterministically without erasing the choice;
changed content forms a different group. Setting a preference checks the current generation,
location/asset pair and matching fingerprint in one write transaction. Only an actual preference
change advances the gallery revision. Ignoring moves a group to a reversible review list and leaves
gallery folding, duplicate badges and physical files unchanged. General review sessions and albums
remain R4.

Schema 43 retires `asset_locations_exact_cursor`. Candidate keyset reads use the existing
`asset_locations_location_id` index, while same-size peer lookup retains
`asset_locations_exact_size`. The wider cursor did not cover the source identity and revision
columns needed for validation and duplicated storage for every location and staged scan.
The forward migration validates the complete schema 42 contract before dropping only this index
in one transaction. It preserves locations, fingerprints, user decisions and catalog revision;
it does not rescan sources or force a database rewrite. Freed pages remain available to SQLite
and the existing bounded reclamation policy. Earlier migrations retain their original index
contract so malformed legacy catalogs cannot silently acquire the new version.

Review pages filter ignored state before pagination and return at most 64 groups or members.
Adjacent pages use keyset cursors. A group viewport without a checkpoint at its missing range uses
one ordinal seek, so distant scrolling and retained-position revalidation do not replay preceding
pages. SQLite clamps and aligns the requested ordinal to a 64-group page in the current filtered
snapshot and returns its actual start with the page; the next cursor resumes ordinary keyset reads.
The application retains at most six payload pages and 64 cursor checkpoints. Member pages retain
their keyset contract and availability checks.
Each group page includes the total group count for its ignored-state filter, independent of the
page cursor. The count and bounded page share one SQLite read snapshot; only the count scalar
crosses the bridge alongside the page. Counting reads current catalog evidence and never opens
source media or enumerates folders.
Member evidence and the saved decision share one read snapshot; metadata-only availability checks
run after releasing the database transaction. The review controller owns independent read and write
generations. A late read cannot erase a newly committed choice or clear a newer request. Visible
review rows own bounded preview demand through the shared preview coordinator; leaving the page or
opening its viewer retires those requests. Viewer navigation retains its admitted member-page
snapshot while catalog changes continue in the background.

## Ownership and validation

Dedicated domain, application, port, guarded-reader and SQLite modules own analysis and evidence.
Existing catalog/session facades only register those owners. Presentation receives bounded Ame
DTOs and sends typed actions; engine types, SQL and source reads remain behind the boundary.

R3-01 exposes analysis progress/cancellation/retry and duplicate-location inspection in the Duplicates
workspace governed by ADR 0029. Changing workspace does not cancel a running task. Gallery
folding remains disabled until incremental maintenance and query consistency are connected.

An application-owned scheduler observes published catalog revisions and source availability.
It starts pending analysis after three seconds without browsing input, explicit foreground tasks,
preview reads or original-image reads. Automatic metadata synchronization and passive gallery reads
can overlap bounded analysis; source generations and held-source checks reject superseded work.
Cached browsing input delays initial admission but does not repeatedly cancel an admitted run.
This allows fresh duplicate evidence to reach the gallery during a long startup recovery instead of
waiting for recovery, all passive refreshes and a complete analysis run in sequence.
Actual foreground reads and explicit tasks request cancellation; continuation waits for
the native stream to retire and reuses completed fingerprints. A newer publication retains its
own pending generation even when an older run completes. Failed analysis waits for a new publication
or an explicit retry. A persisted pause preference disables automatic admission across restarts.
Application shutdown fences admission and drains analysis before retiring synchronization/storage.
These signals schedule bounded catalog candidates; they never trigger another source-tree scan.

Accept after fixed-fixture full-content correctness, cancellation and read-race rejection,
transactional stale-publication rejection, migration preservation, hardlink/multi-volume counting,
bounded paging and actual client inspection/restart reuse pass. GitHub Actions owns automated
checks. The local client check reuses `build/test`. Replace the adapter if these guarantees cannot
be met or measurements justify a different engine; preserve the application and evidence contract.
