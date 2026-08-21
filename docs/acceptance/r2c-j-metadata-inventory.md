# R2c-J metadata-inventory persistence and discovery acceptance

Status: implementation complete; independent audit pending

Date: 2026-08-21

## Scope

R2c-J implements the persistence and discovery boundary defined by ADR 0023. It does not schedule
production startup inventory or continuity epochs; those orchestration responsibilities remain in
R2c-K.

## Implemented contract

- Schema v20 adds exact-shape inventory run, staging, cursor, completion-authority, cleanup, and
  comparison indexes outside source roots.
- The v19-to-v20 migration preserves change identifiers, queue state, retries, leases,
  authoritative scan ownership, self-references, catch-up evidence, and queue lineage while adding
  the `metadata_inventory` origin.
- Current-schema validation checks exact inventory columns and indexes, the active-root index
  predicate, cascade relationships, the queue origin contract, marker completion, root generation,
  published-catalog ownership, staged counts, scope containment, and foreign-key integrity.
- Local inventory walks root or subtree scopes in pages of at most 4,096 entries and records path,
  entry kind, size, modification time, optional Windows identity, reparse evidence, and placeholder
  state.
- Inventory does not inspect media signatures, decode images, generate previews, hash or read source
  bytes, follow intermediate or terminal reparse directories, or open identity handles for offline
  or recall placeholders.
- Positive differences enqueue the existing final-state path reconciler. Same-root identity evidence
  may create one paired rename candidate without reusing a previous path for another hard link.
- Missing paths are loaded only after complete enumeration establishes scope-wide absence authority.
  Missing subtrees produce an empty complete page; partial, failed, cancelled, stale-generation, or
  malformed runs never authorize absence.
- Any failure after durable run creation terminates the run with a bounded structured issue. A
  cancelled run remains non-authoritative unless enumeration had already completed; already
  enqueued final-state work remains safe and idempotent.

## Focused verification

- metadata inventory: 10 passed, covering fixed-bound all-entry enumeration, empty missing subtree,
  controlled closed-process create/modify/delete/rename/directory move, Chinese and long paths,
  placeholder evidence, cancellation, durable failure, and hard-link pairing;
- schema migrations: 29 passed, including v19 queue/lineage preservation, new-origin admission,
  exact-index and predicate rejection, missing-origin rejection, stale-generation rejection, and all
  retained v17-v19 compatibility fixtures;
- offline placeholder adapter: passed, proving inventory records the placeholder without file
  identity and preserves source bytes;
- complete Rust suite: 433 total, 426 passed, seven existing explicit ignores, zero failures;
- `./tool/quality_lint.ps1`: passed, including repository guardrails, formatting, Clippy with
  warnings denied, and Dart analysis;
- `./tool/quality_verify_daily.ps1`: passed, including the complete Rust and Flutter suites,
  Windows scan integration, Windows accessibility integration, bridge compatibility, and
  whitespace validation;
- `./tool/release_verify_windows.ps1`: passed, including the Windows Release build and two packaged
  bridge smoke tests;
- independent read-only audit: pending.

The focused fixtures use disposable directories and isolated catalogs. They do not access
`local-primary` or `cloud-primary`, hydrate cloud content, or modify any real source media.

## Next boundary

R2c-K owns production continuity epochs, cold-start and watcher-gap scheduling, durable page
continuation, live-event supersession, bounded backpressure, restart semantics, and fairness. R2c-J
does not claim those production lifecycle behaviors.
