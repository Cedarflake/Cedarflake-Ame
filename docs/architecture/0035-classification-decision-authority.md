# ADR 0035: Keep classification evidence and human authority independent

- Status: Accepted for implementation
- Date: 2026-10-30

## Context

R5 adds category suggestions and human review to the content groups established by R3/R4. A later
model result must not replace a human choice, and a delayed confirmation must accept exactly the
suggestion the user saw. Category decisions also need to survive model removal, catalog rebuilding
and portable recovery without changing manual albums, Favorites or organization progress.

[ADR 0034](0034-versioned-classification-contracts.md) owns encoder/rule compatibility and screening.
[ADR 0030](0030-durable-organization-decisions.md) owns durable content lineage and association.
[ADR 0031](0031-portable-organization-archives.md) owns portable framing and atomic restoration.

## Evidence and effective state

An immutable classification run binds an opaque run ID, model compatibility, primary rules and the
optional refinement rules/profile. Its configuration revision excludes the execution ID: two runs
can use the same configuration, but their results and accepted evidence remain distinct. Encoder
pipeline revisions include artifact, runtime and execution parameters under ADR 0034.

A prediction binds that run to versioned exact-content evidence and retains both admitted rankings
when refinement occurs. Rankings contain one to 256 unique category identities and finite cosine
scores. Refinement must preserve the model and category set and use the run's refinement rules.
An uncalibrated run cannot publish a calibrated review recommendation. Rebuildable prediction and
feature storage have no cascading ownership of human choices.

A saved confirmation retains the accepted prediction ID, run ID, configuration, primary rules,
content and category. A correction retains the explicitly selected category and rule revision.
These are durable values, so removing a prediction cache or model does not remove their provenance.
Display renaming preserves compatibility; category retirement retains older choices and never
remaps them to another category.

Resolve applicable human choices before inspecting machine evidence. Equal category IDs agree even
when their confirmation/correction provenance differs. Different current local choices produce a
human conflict with no effective winner. Imported choices participate when no current local choice
exists, matching the field-specific recovery precedence of ADR 0030. Conflicting imported choices
also retain a conflict. This reduction streams lineage decisions with constant working memory;
the persistence owner retains every source decision rather than discarding it during reduction.
Explicitly confirming or correcting an imported choice establishes local intent even when its
category is unchanged. A projection retains that origin so an equality shortcut cannot discard the
new authority.

Without human intent, only the current compatible prediction supplies a category. Its recommendation
selects optional review or routine sampling, never confirmation. Deferred work retains its usable
suggestion. Missing or incompatible evidence has no invented category. Human conflicts remain
distinct from machine uncertainty and from R4 album conflicts.

## Gallery projection

Smart albums are a typed gallery collection with an optional category and an independent review
filter. They always fold exact-content groups and cannot carry R4 organization queues. The existing
gallery owns physical scope, search, sorting, paging, selection, layout and viewer anchors; the
classification owner supplies only effective membership. All-results includes groups without usable
predictions. Pending includes optional checking, deferral and human conflicts; a routine suggestion
does not become a human confirmation.

Schema 70 adds rebuildable SQL views over human lineage choices and compatible prediction evidence.
Human reduction follows the same local-before-restored consensus as commands. Prediction projection
selects the current configuration/rules and the latest publication for the displayed content, using
the admitted refined ranking when present. These views neither copy decisions into R4 records nor
introduce a second publication lifecycle. Existing transactional catalog revisions invalidate query
snapshots after prediction, decision, group or source changes.

Reads use the gallery's published and explicitly retained display evidence so a pending verification
does not independently change the visible group's category. Retained evidence grants no command
authority: confirmation and correction still require the current content, membership and field
guards. Physical scope can choose another representative of the same group. Projection migration
adds no source reads, model execution or archive records; normal pre-upgrade snapshots protect
rollback, and all prior durable decisions remain unchanged.

## Command authority

A prepared classification command binds the exact or provisional content target, complete current
membership authority, independent category/review field authorities, category-rule revision and the
displayed prediction reference. These are application values, not paths or engine objects.

The transaction re-resolves that target before mutation. Changed content, group membership or an
affected field rejects the old command. Album/Favorite revisions are not classification authority.
Confirmation additionally requires the same displayed prediction and active model/rule configuration;
a newer prediction cannot be accepted by an old click even if it suggests the same category.

Correction requires an active explicit destination in the still-current taxonomy. A model-only
replacement does not invalidate that human selection. A human conflict requires an explicit
category choice; the model cannot pick its winner. Deferral changes only unfinished classification
review. It cannot reopen a confirmed human choice or require an unchanged model configuration.

Commands produce typed category/review changes. The repository records before-images, field
authorities and opaque operation targets and commits them atomically with a receipt. Retry checks
the immutable request identity before stale-state checks. Operation-specific undo validates every
affected lineage and field; later choices, cleanup, restoration or a separately handled late member
cannot be overwritten through a partial undo. Classification operations never enter R4's
chronological organization undo/redo cursor.

## Persistence and recovery boundary

Reuse R4's durable lineage bindings and configured-source ownership at the SQLite adapter boundary.
Classification values, review continuation and operation history have separate typed owners and
tables; they are not additional album/progress fields. Ordinary catalog loss retains these records.
Explicit removal/cleanup retires classification intent with the affected unsustained lineage and
keeps enough opaque operation provenance to reject resurrection through undo.

The persistence implementation must extend forward migrations and portable recovery together.
Archive records include category definitions/history, human choices, their association evidence,
classification continuation and applicable operation provenance. Model files, cached rankings and
features remain excluded. Old R3/R4 archives remain readable. Restore uses independent local
classification revisions and preserves current local choices; foreign revisions and sessions never
become live command authority. Recovery without installed model binaries preserves human choices.

Schema 65 keeps category definitions and immutable semantic versions alongside separate run,
prediction, human-choice, operation and continuation tables. Human choices reference the durable
organization lineage, never a derived asset or prediction row. Category and review fields retain
their own revisions and imported-intent markers. Classification writes publish the catalog revision
without advancing R4's organization history. A change to category descriptions retires the active
analysis configuration; changing only a display label keeps compatible predictions available.

Each operation retains its request digest, before-images and opaque targets. Target records survive
lineage cleanup even when their before-images are retired. Imported operation history retains a
restoration marker so its foreign receipt cannot authorize a new undo. New local operations receive
fresh local revisions. These rules also apply when explicitly recovering previously removed records.

Portable format 5 appends classification records to the existing ordered archive. Its classification
context carries a separate revision ceiling and the saved category-rule selection. Definitions,
semantic history, choices, operation provenance and the logical bookmark travel with the existing
association evidence. Active runs, active review sessions, model configuration and predictions remain
local. Restoration reserves a separate checked classification revision interval and commits within
the existing archive transaction. Existing category labels, active rules and handled local fields
take precedence; missing fields can recover imported intent. Versions 1 through 4 keep their original
record counts and acquire no classification state during restoration.

Classification review uses a separate session, monotonically increasing position sequence and typed
scope. Checkpoints retain the logical group and the gallery's existing ordering boundary. A review
command validates the active session and bookmarked group, then commits its field changes and next
position authority together. Identical retries return the original receipt; an old session or
position cannot replace later work. Confirmation stays at the current image when its scope still
includes it; deferral or leaving the selected category/review filter records an after-current anchor.
R5-05 owns resolving that anchor through the bounded smart-album projection. R4's session, bookmark,
album decisions and Favorite fields are independent.

Removing a source retires scopes bound to it. A global bookmark may use a surviving copy of the same
group. Compatible root reassociation remaps retained scopes without treating paths as identity.
Unresolved-record cleanup includes category/review fields and the classification bookmark in its
counts and guarded digest; retired lineages remain in opaque operation targets to reject partial undo.

The schema and archive extension are implemented and verified together in R5-03 under the
[roadmap](../roadmap.md#r5-implementation-sequence); this decision does not establish their completion.

## Alternatives and consequences

Storing classification in album assignments would couple independent user decisions and is rejected.
Choosing the newest model result over human intent, or the latest human choice during a genuine
late-group conflict, would silently discard intent and is rejected. Global catalog revisions alone
would invalidate classification commands after unrelated Favorite changes while failing to express
which evidence was confirmed.

Typed evidence and field authority add bounded translation at persistence/bridge boundaries. They
allow engine replacement and cache rebuilding without rewriting the decision contract. Rollback of
the future persistence extension must use its pre-upgrade snapshot; older software must not silently
drop new durable records.

## Verification

Boundary tests cover mixed/invalid rankings, incompatible runs/refinement, same-input run identity,
human precedence, conflict reduction, stale confirmation, content/membership/field changes, retired
destinations and model-independent correction/deferral. Persistence verification must additionally
cover restart, rebuild, source removal, cleanup, atomic/idempotent writes and undo, archive versions,
restore precedence and independent R4 state. Passing the domain checks alone does not establish those
storage or desktop workflows.
