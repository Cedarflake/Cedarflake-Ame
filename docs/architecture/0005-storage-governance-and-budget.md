# ADR 0005: Freeze active storage and enforce a preview budget

- Status: Accepted
- Date: 2026-08-07

## Context

Ame cannot assume that the system drive has enough space for previews derived from a roughly
259 GB image library. The user needs to see and configure catalog and preview locations without
placing application data inside source trees or requiring a second full copy of the collection.

Changing a live catalog path can split application state, while moving an existing catalog or
cache automatically can consume substantial space and make a failed restart difficult to recover.
The catalog also contains preview paths generated under the active cache root. Storage settings
therefore need a durable control plane that is independent of the catalog being configured.

## Decision

- A small SQLite settings database remains at the operating-system configuration location. It
  stores the configured catalog file, versioned preview-cache root, and preview byte budget.
- The active storage configuration is resolved once per process. Saving a changed location or
  budget never changes the paths used by the running process; the UI reports that a restart is
  required.
- Ame creates target storage directories only after validating the complete update. A catalog or
  preview path that overlaps an imported source root is rejected.
- Catalog relocation is rejected after the active catalog contains a library root. A later
  migration workflow must copy, verify, switch, and recover explicitly before this restriction can
  be removed.
- Preview relocation affects future preview generation after restart. Existing cache files are not
  moved or deleted, so existing catalog references remain usable while the old cache is retained.
- The preview adapter counts existing files when it opens. It atomically reserves space before
  publishing each new preview and returns a structured per-file issue when the configured budget
  would be exceeded. Existing artifacts remain readable even when the budget has subsequently been
  reduced.
- The supported preview budget is 64 MiB through 1 TiB. The initial default is 4 GiB.
- This slice does not evict previews, migrate catalogs, clean old cache roots, or modify source
  media.

## Consequences and risks

- Configuration survives a catalog relocation because it is not stored inside the configured
  catalog.
- Storage changes are predictable and reversible by editing the configuration again before any
  migration occurs.
- A pending configuration can differ from active storage until restart, so both paths are exposed
  explicitly through the application contract.
- Changing the preview root can leave rebuildable previews in more than one location. Ame must not
  call that space reclaimable until a verified cleanup workflow exists.
- A full cache causes missing previews to be reported as isolated issues instead of allowing
  unbounded growth. Eviction and viewport-priority scheduling remain future work.

## Validation evidence

- SQLite adapter tests initialize, update transactionally, close, and reload the settings database.
- Application tests verify budget bounds, Windows path overlap behavior, and pending-versus-active
  restart semantics.
- Preview adapter tests exhaust a small budget and verify that neither a temporary artifact nor a
  source-media change remains.
- Flutter tests verify the imported-library relocation guard, budget updates, and restart notice.
- The isolated Windows integration test reads the real storage status through the generated bridge
  after a scan and verifies that catalog relocation is locked.

## Replacement strategy

The settings repository, catalog repository, and preview store remain separate ports. A future
migration service may replace the relocation restriction only after it implements copy, integrity
verification, atomic activation, rollback, progress, cancellation, and interrupted-run recovery.
An eviction strategy may replace the admission-only budget without changing catalog ownership or
source-media safety.
