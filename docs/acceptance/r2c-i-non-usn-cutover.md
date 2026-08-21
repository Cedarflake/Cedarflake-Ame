# R2c-I non-USN production cutover acceptance

Status: implementation verified on 2026-08-21; merge acceptance awaits final independent audit

## Scope

R2c-I establishes the production boundary required by ADR 0023 before metadata-inventory
persistence and discovery are introduced in R2c-J. The slice covers production source selection,
startup continuity scheduling, automatic recovery authority, full-scan authority, and schema v19
compatibility.

## Accepted implementation

- Production synchronization starts the Windows notification source before it persists startup
  continuity work.
- Production does not construct or schedule the Windows USN catch-up adapter. The historical USN
  reader and catch-up application service compile only for migration and regression tests.
- Startup and live evidence gaps persist root metadata-inventory work through the existing durable
  queue compatibility contract. They do not create a scan run.
- Bounded authoritative work that exceeds one page returns the structured
  `metadata_inventory_required` retry result without advancing the catalog revision or creating a
  full-scan request.
- Full scans have separate create-new and resume-existing persistence operations and separate
  Flutter/Rust bridge entrypoints. User-requested scans start new work with a fresh scan identifier;
  foreground restart and production recovery can only resume an existing checkpoint whose root,
  owner, generation, parameters, and running or paused state still match.
- Resume fails closed before emitting `Started` and never inserts a root, scan, frontier, or queue
  lease when the checkpoint has been removed or changed. Root unregistration racing a previously
  loaded recovery checkpoint therefore cannot recreate the removed root.
- Schema v19 tables, lineage, handoff evidence, catalog rows, assets, previews, and root-generation
  authority remain readable. Historical catch-up execution APIs are excluded from production while
  their persistence shapes remain available to migration and compatibility paths.
- No production path requests elevation or changes behavior based on USN availability.

R2c-I intentionally does not enumerate or publish a metadata inventory. Until R2c-J and R2c-K are
complete, work that exceeds the bounded authoritative page remains durable and retryable rather
than escalating to a full scan.

## Focused verification

- authoritative recovery: 7 passed, including oversized scope retry without publication;
- synchronization runtime: 19 passed, including watcher-first startup and live evidence-gap
  persistence without a recoverable scan;
- production recovery coordinator: 10 passed, including bounded scheduling, fairness, cancellation,
  and checkpoint-only scan resumption;
- scan library: 36 total, 34 passed and 2 explicitly ignored, including foreground and
  authoritative checkpoint resumption plus missing-checkpoint fail-closed behavior;
- SQLite catalog: 58 passed, including create/resume separation and unregister-before-resume
  transaction coverage;
- Flutter library controller: 39 passed, including typed foreground restart and paused-scan resume
  routing without invoking the create-new bridge;
- schema migrations: 24 passed, including v17, v18, prerelease v19, and current v19 fail-closed
  fixtures;
- historical catch-up compatibility service: 5 passed under the test-only boundary;
- `cargo check --all-targets --all-features`: passed without warnings;
- `cargo clippy --all-targets --all-features -- -D warnings`: passed;
- Rust complete suite: 421 total, 414 passed and 7 explicitly ignored.

## Repository gates

- `./tool/quality_lint.ps1`: passed; formatting checked 145 files with zero changes, Clippy passed,
  and Dart analysis reported no issues;
- `./tool/quality_verify_daily.ps1`: passed; complete Rust and Flutter suites, Windows Scan 2/2,
  Windows Accessibility 2/2, bridge compatibility, and whitespace checks completed;
- `./tool/release_verify_windows.ps1`: passed; Windows Release built and both packaged bridge smoke
  tests passed;
- `git diff --check`: passed.

No real-library root was accessed, no cloud placeholder was hydrated, and no source media was
modified during this acceptance run.

## Next boundary

R2c-J owns schema v20 metadata-inventory run and staging contracts, bounded metadata enumeration,
comparison against the active catalog, and complete-scope absence authority. R2c-K then connects
those contracts to continuity epochs, paging, live-event supersession, and production recovery.
