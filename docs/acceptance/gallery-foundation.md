# Gallery foundation acceptance and regression boundaries

Status: historical R0/R1/R2a/R2b acceptance with retained regression requirements.

This record preserves the foundation decisions, frozen comparison revision, conditional adaptation
rules, and dated acceptance previously embedded in the roadmap. Historical passes do not establish
current-source verification. The [roadmap](../roadmap.md) owns current stage status;
[ADR 0005](../architecture/0005-storage-governance-and-budget.md) and
[ADR 0014](../architecture/0014-query-wide-gallery-layout-manifest.md) own storage and layout decisions.
No new retained-library run is authorized by these historical results.

## Preview and geometry regression contract

R2b owns two deliberately separate lifecycles:

- **Aspect-ratio evidence**: media inspection records orientation-corrected width and height with
  compatible source state and engine identity. Restart and preview-cache operations reuse those
  dimensions without decoding the source merely to recover layout. An unknown dimension uses one
  stable documented fallback until a complete newer catalog revision or compatible bounded
  geometry-evidence epoch supplies trustworthy evidence. Preview readiness never upgrades layout
  geometry on its own; first-time dimension recovery is coalesced and identity-checked separately.
- **Preview artifacts**: demand moves a compatible artifact through absent, pending, generating,
  ready, failed, stale, and evictable conditions without turning those conditions into layout state.
  Persistent representation may combine states where safe, but failure evidence, stale-publication
  guards, and recovery behavior remain explicit and testable.

R2b proves these contracts through initial scan, explicit rescan, restart, preview demand, cleanup,
and storage transitions. R2c reuses the same retain-or-invalidate semantics when it adds automatic
source-change detection; it does not create a second dimensions or preview lifecycle.

The preview-artifact lifecycle is complete only when all of the following hold:

1. viewer, visible, movement-direction-near, guard, and idle demand use the documented priority
   order with bounded generation and decode concurrency;
2. publication is atomic and guarded by the exact root, active scan, location, source generation,
   source revision, algorithm version, orientation contract, and requested size bucket. Query
   catalog revision remains a separate presentation guard, so an unrelated catalog write does not
   invalidate an otherwise exact source lease;
3. compatible unchanged files and identity-proven renames or moves reuse artifacts, while content
   edits, same-path replacements, and incompatible algorithm or orientation contracts invalidate
   them without exposing stale pixels as current;
4. the preview index can account for artifact path, byte size, bounded size bucket, file identity,
   source revision, source generation, and coarsened last-use evidence without writing persistent
   state on every scroll tick;
5. capacity uses a high watermark and a lower reclamation target so cleanup does not oscillate at
   the configured limit. Temporary and unreferenced files, obsolete algorithms, incompatible or
   superseded size variants, and then least-recently-used distant artifacts are reclaimed in that
   order;
6. the active viewer item, visible items, directional guard demand, and in-flight atomic publication
   are pinned for the current reclamation pass. Eviction never enters the pointer-to-scroll path;
7. startup reconciles reserved bytes, interrupted temporary files, missing ready files, and
   unreferenced artifacts in bounded work. A missing derived file returns to pending demand rather
   than becoming a permanent gallery failure;
8. manual cleanup and preview-location change expose progress, cancellation, completion, and
   failure honestly, preserve source media and durable dimensions, and leave one recoverable active
   storage configuration after restart;
9. size buckets and reclamation thresholds are selected from display-scale, quality, latency,
   storage, and churn measurements. They are bounded policy, not a per-pixel cache-key expansion;
10. fixed fixtures prove EXIF Orientation 1 through 8, unknown-dimension fallback and settled
    recovery, missing and failed previews, manual cleanup, automatic reclamation, restart recovery,
    and cache-boundary repetition without per-preview geometry churn or source-media mutation.

Within the currently accepted ADR 0005 lifecycle, resource-safety work comes first: artifact
accounting, bounded variants, high/low-watermark reclamation, stale-publication guards, and bounded
startup recovery. User-facing manual cleanup and preview-root transition follow only after that core
is stable. Moving either later workflow out of R2b requires an explicit amendment to ADR 0005; this
roadmap does not silently weaken an accepted architecture decision merely to shorten the stage.

R2b does not require every optional ADR 0014 scale adaptation to be enabled merely to complete a
migration checklist. Its acceptance policy is:

1. freeze the current wheel, time-rail, jump, and resize behavior as the comparison baseline;
2. run resource-bounded Profile and long-session observation against a retained catalog without a
   new real-root import;
3. record retained detail count, process working set, garbage collection, page-publication copy
   time, frame timing, programmatic scroll writers, and flat-manifest cost;
4. separately run a bounded, read-only, source-readable preview workload and record cold and warm
   preview latency, cache-byte growth, bucket demand and reuse, reclamation duration, regeneration,
   and boundary churn. A retained-gallery Profile that rejects source-media materialization and a
   catalog-parity run that leaves every preview pending do not satisfy this evidence;
5. implement and validate ADR 0005's preview lifecycle before enabling target cleanup, reclamation,
   or preview-root transition behavior; the accepted aspect-ratio contract remains fixed;
6. change any remaining performance structure only when it exceeds its recorded budget, one
   variable at a time;
7. compare every change with the frozen baseline and reject a nearby-return, reversal, distant-jump,
   resize, or native-input regression;
8. pass current-authorized real-library parity and Windows Release verification before closing R2b.

Profile, builds, tests, scans, and acceptance runs remain serial on the project workstation. They
reuse the retained catalog where the scenario permits, start with bounded durations, and stop at an
explicit memory or runtime limit. Resource exhaustion is neither product acceptance evidence nor a
reason to hide an unexecuted gate.

The timeline slice is accepted only when focused geometry and widget tests plus a real large-library
interaction run prove that dragging moves the gallery every frame, unloaded ranges materialize
without changing the global position, rapid reversals retain the latest target, no stale window
overwrites the current query, and source media remains untouched. Passing analysis or rendering the
rail without this interaction evidence is insufficient.

## Retained acceptance and interaction baseline

R2b implementation, deterministic preview-lifecycle correctness, retained-catalog interaction
Profile, real-library catalog parity, Daily, Windows Release, and bounded source-readable preview
performance gates are complete. R2b was accepted on 2026-08-13. The former USN-based R2c reached its
recorded implementation and audit state on 2026-08-19, then was reopened on 2026-08-21 after the
target token could not use its primary continuity source and automatic fallback caused excessive
recovery work. The R2b interaction and source-safety contracts and accepted R2c incremental
publication contracts remain regression boundaries rather than migration work to repeat.

The frozen R2b interaction comparison revision is
`6d3f0686a91b85402251fe07fcc1690f268effd5`. It remains historical A/B evidence rather than a moving
current-status pointer. R2c must preserve the frozen native interaction contract, but new R2c
behavior establishes task-specific evidence against the current accepted implementation.

R2b Profile evidence reproduced retained-detail growth and triggered one guarded change. The
accepted controller now uses high and low watermarks with hysteresis rather than aggressive
page-by-page eviction; it did not replace the native scroll, time-rail, jump, or resize paths.

On 2026-08-10 the user reported that the current gallery interaction feels acceptable and directed
the project to avoid speculative or migration-driven performance changes that could create a
negative optimization. Remaining ADR 0014 slices are therefore implementation options behind
measured thresholds and behavior-parity gates, not authorization to rewrite the current scroll hot
path merely to complete the migration sequence. Bounded-memory requirements remain binding, but
their production implementation must preserve or improve the reported interaction baseline.

Every conditional interaction change is evaluated one variable at a time against the frozen
baseline in the same build mode. It is rejected when P95 build or raster time leaves the 60 Hz frame
budget, regresses by more than 10 percent, adds UI-thread stalls above 50 ms, increases nearby-return
or reversal placeholder exposure, adds avoidable catalog requests, delays the time-rail position
line beyond one display frame, or exceeds ADR 0014's two-logical-pixel settled resize drift. Profile
evidence compares Profile with Profile; final hand-feel acceptance uses Windows Release. A rejected
change is rolled back instead of being retained behind compensating debounce or synchronization.

The visible Flutter shell is the production surface and must not be discarded or treated as a
fixture-only prototype. R2c is now limited to freshness state, durable incremental change capture,
bounded catch-up, revision-safe delta publication, and explicit fallback reconciliation. It must not
use synchronization work as a reason to rewrite the accepted gallery, preview cache, classification,
or later analysis workflows.

## Historical acceptance checkpoints

R0 acceptance:

```text
Windows Debug and Release launch
-> production native picker cancellation and controlled fixture import
-> Rust bridge scan, per-file issue isolation, preview rendering, and atomic catalog publication
-> catalog and previews outside source trees
-> unchanged source bytes and entries
-> accepted
```

R1 acceptance:

```text
resumable multi-root catalog and explicit-rescan reconciliation
-> authorized local-primary and cloud-primary read-only scans
-> two active roots and 79,013 active locations
-> bounded revision-safe catalog traversal without duplicate or gap
-> unchanged sampled source bytes and no forced full-library preview generation
-> accepted
```

R2b completed foundation:

- accepted production shell and unified-gallery interaction contract;
- complete-result timeline, keyset sort and search, stable selection, viewer, source actions, and
  scan feedback;
- query-wide final geometry with unloaded, failed, and ready states sharing the same rectangles;
- identity-keyed preview publication and bounded center-out demand priority;
- latest-wins time navigation, native relative scrolling, and actual-card anchor preservation;
- EXIF Orientation 1 through 8 reflected in durable dimensions and preview pixels;
- scan finalization, failure publication, and Explorer reveal corrections present in current
  history;
- deterministic fixture coverage and a successful hosted Daily gate for the frozen comparison
  revision.

R2b acceptance: **accepted on 2026-08-13**

1. **Complete** - frozen interaction Profile and long-session evidence on the retained catalog;
2. **Complete** - ADR 0005 preview-lifecycle implementation and deterministic recovery, cleanup,
   storage, geometry, and source-safety tests;
3. **Complete** - current local Daily and Windows Release gates;
4. **Complete** - currently authorized retained-catalog real-library parity without source mutation
   or cloud-placeholder hydration;
5. **Complete** - the separately authorized, bounded, source-readable preview workload required by
   ADR 0005 and acceptance-policy step 4, with cache pressure, reclamation, regeneration, memory,
   source-entry, and source-byte evidence recorded above.

R2b conditional-adaptation decisions:

1. **Triggered and complete** - retained-detail growth exceeded the stable-range requirement. The
   guarded high/low-watermark detail cache passed reversal, resize, viewer, native-input, and frozen
   Profile frame gates without changing the scroll, time-rail, or resize implementations.
2. **Not required for the target workload** - the 79,013-item flat manifest remains inside budget;
   the existing hierarchical fallback remains conditional scale validation.
3. **Not required** - traces did not reproduce competing programmatic position writers. Native
   Flutter `Scrollable` movement remains immediate and outside an asynchronous intent queue.

An untriggered conditional adaptation is a resolved **not required** decision, not unfinished R2b
work. Do not implement one merely to complete an ADR migration sequence or count it as a blocker
without the corresponding measurement or trace evidence.

## Historical reference observations

Known reference evidence from the real library:

- Lap v0.3.0 terminated twice at the same scan position with Windows `0xc0000409`.
- The recovery point was near a file named as JPEG whose content was valid PNG.
- A read-only header probe found thousands of JPG/PNG extension-content mismatches.
- A large group of reference-library files returned access-denied during content reads.
- Ame therefore requires per-file failure isolation, format detection based on evidence rather than
  extension alone, structured issue reporting, and recoverable tasks.
