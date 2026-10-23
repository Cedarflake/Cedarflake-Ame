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

An organization record represents a handled content lineage. Its bindings retain the asset identity,
source generation, physical identity/revision evidence and, when available, the versioned exact
content fingerprint. Root associations are retained separately so deliberate source removal can
retire only its associations and the records with no surviving owners. A rebuilt catalog can
reassociate a record through compatible identity and content evidence; a filename is insufficient.
Unresolved associations retain their first-unassociated time for explicit cleanup.

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
