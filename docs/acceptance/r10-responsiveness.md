# R10a responsiveness evidence

The [roadmap](../roadmap.md#r10--beta-readiness) owns delivery status. This record supports
[R10-01](../plans/r10-beta-readiness.md#r10-01--responsiveness-and-release-baseline).

## Retained-library baseline — 2026-11-14

Baseline: `f1dcd2e27098b9342c8e462ff81ab752331d355d`, Windows x64, retained `local-primary`
catalog with 63,170 locations and 47,926 merged entries. Native measurements use the existing
optimized Profile-bundle DLL through the production Dart/Rust read API. Sources remain unchanged.
These timings locate backend costs; they are not input-to-rendered-content acceptance.

| Read | First process/read (ms) | Subsequent reads (ms) |
| --- | ---: | ---: |
| Merged gallery snapshot | 5,777 | 1,080 / 1,061 |
| Expanded gallery snapshot | 324 | 284 / 275 |
| Smart-album navigation | 1,763 | 901 / 920 |
| Three smart-category snapshots | 1,572 / 1,410 / 1,458 | 1,302–1,444 |
| Pending organization queue | 1,455 | 1,477 / 1,494 |
| Exact groups | 1,585 | 811 / 789 |
| Similar groups | 456 | 413 / 451 |
| Ignored exact / similar groups | 6 / 21 | 6–18 |

The first cold read in the session took 17,237 ms; later fresh-process reads were faster.
Temporary native instrumentation attributed the long startup to initial catalog validation and
the merged layout projection. In the instrumented run, projection cost 1,028 ms, its detail window
another 11 ms, and preview-state reconciliation 82 ms. Instrumentation was removed after diagnosis.

## Scoped similarity relationship validation

Similarity validation previously ran the global `pragma_foreign_key_check` and filtered its output
by table name. Filtering the output still traversed unrelated tables. It now selects similarity
tables first and passes each name into SQLite's table-specific check. The complete catalog check
remains in the catalog's existing validation path, and similarity relationship rejection is unchanged.

The focused upgrade, rollback and orphan-reference cases passed. The new case checks that an
unrelated orphan remains outside similarity ownership while an orphan similarity variant is rejected.
Local fixtures ran under the managed acceptance scratch directory after the host's default Temp
directory denied fixture creation. No fixture fallback or assertion was changed.

## Revision-bound membership reuse

The validated session retains at most eight folded selections within 64 MiB. Each hit refreshes
geometry and details in its current read transaction. Metadata-filtered, group-filtered, semantic
and unfolded queries keep their direct path. A catalog revision change discards retained membership.

The same retained-library API comparison after this change produced:

| Read | Subsequent reads before (ms) | Subsequent reads after (ms) |
| --- | ---: | ---: |
| Merged gallery | 1,080 / 1,061 | 548 / 287 |
| Three smart categories | 1,302–1,444 | 39–84 |
| Favorites | 406 / 400 | 21 / 17 |
| Pending organization queue | 1,477 / 1,494 | 655 / 660 |

Expanded-gallery reads remained 277–281 ms. First-use category grouping and smart-navigation
aggregation remain on their original SQL path; these figures do not claim an improvement there.
The fresh native process's first merged read was 5,690 ms. Release UI acceptance is still pending.

The focused snapshot cases passed, including reuse across independently opened session readers,
query-key separation, publication invalidation, geometry recovery without a revision change,
concurrent publication and stale ordinary pagination. The work comparison uses a folded query;
the direct unfolded query remains outside this cache.

## Indexed source-state validation

The source-revision row validator previously grouped full source state for every identity. It now
uses the existing identity indexes to select repeated identities before comparing alias state.
Generation, known revision, size, modification time, ready-preview proof and allocation-clock
checks retain their existing semantics, including independent unpublished scans and unknown revisions.

On the same read-only retained catalog, the original and revised SQL both accepted the data. The
first paired reads took 818 / 450 ms; subsequent pairs took 408 / 264 and 403 / 265 ms. These are
isolated SQLite query measurements, not complete startup measurements.

The focused source-revision checks passed: active cross-root and staged within-scan disagreements,
unknown revisions, independent scan observations, precontract repair, damaged markers/structure,
rollback and the v30-to-v31 migration. The extracted validator has 75 lines with no inline tests;
its dedicated boundary fixture has 77 test lines. The existing repair/migration tests remain in place.

## Shared startup relationship audit

The full catalog validator already checks every foreign key in its journal proof. Classification
and dispatch now validate their content after that proof, in the same read transaction, without
repeating the large relationship traversals. Their singleton, payload, dispatch and lease checks
remain unchanged. The startup regression rejects orphan classification and work references; the
clean/read-only and damaged-schema cases also passed.

Five Release launches on the retained catalog took 5,109–5,175 ms from process launch to the first
decoded gallery image. Dart-entry-to-content time was 4,678–4,749 ms, compared with 5,111–5,315 ms
before removing the duplicate audit. The earlier instrument started at Dart entry and only required
gallery tiles, so its figures are not interchangeable with the stricter process-to-decoded-image
measurement. The five-second process target remains open.

## Semantic candidate and result reuse

Completed searches may retain one candidate list within 16 MiB, keyed by the full query and catalog
revision. This contains source identifiers and fingerprint evidence; vector pages keep their existing
bounded reader. Only scopes admitted by the gallery projection cache qualify. Cancellation, partial
reads and oversized lists publish no entry, and late completions cannot replace a newer revision.
Result rendering can reuse the scope's duplicate representatives when every manifest member belongs
to that exact revision's selection. Other queries retain their original filtering path.

The real Release window's two subsequent semantic searches took 700 and 854 ms, compared with
3,079–3,132 ms before reuse. They returned 412 and 493 results with no search error. The first query,
including initial candidate retrieval, took 2,060 ms and returned 1,515 results. These are exploratory
input-to-content readings; the 20-switch acceptance remains separate. The session finished without
Flutter errors at 420 MB client working set. No image vectors or original media were retained here.

The existing relevance/paging and concurrent-publication regressions passed with the cached scope.
They retain old-snapshot consistency, fresh publication visibility and cancellation between bounded
windows. A separate cache boundary check covers its size budget and late older completion.
The new candidate cache has 114 production and 39 inline-test lines; the shared semantic catalog
fixture has 270 dedicated-test lines. The snapshot reader remains 205 production lines.
The existing catalog facade has 2,560 production and 445 test-support lines, with no inline test
bodies; this change only connects the session caches and existing read entrypoints there.

The first process after rebuilding took 12,002 ms, while five subsequent fresh processes took
5,116–5,190 ms. The preceding rebuild also had a 13,979 ms first launch. These outliers remain part
of the startup record; their cause is not yet isolated, and the process startup target remains open.

## Active-root representative lookup

The duplicate representative subquery now joins each root to its active locations using both root
and scan identity. The root-first access path uses the existing classification-publication index,
avoiding a scan of retired locations or the former scattered primary-key reads. Unfingerprinted
items retain their short-circuit path. No index, schema, preferred-image rule or query scope changed.

Paired read-only SQL runs returned identical 47,926-entry sequences. Warm original reads took
1,204 / 1,220 ms; root-first reads took 899 / 889 ms. A plain root predicate also improved this
catalog but selected a full location scan, so the active-root path was retained for catalogs with
historical scans. Existing scoped-representative, publication and unfingerprinted-page checks remain
the correctness coverage for this query adjustment.

Five subsequent fresh Release processes reached decoded gallery content in 4,746–4,846 ms. The
first launch immediately after rebuilding still took 12,378 ms; that separate cold-build condition
needs diagnosis before claiming uniformly fast first use. The query owner has 159 production lines
and no inline tests; no new test body or user-facing behavior was added for the connection predicate.

## Continuous resize and concurrent publication

The initial retained-library Profile run found a 198-pixel equal-height drift during a six-second
window-size round trip. Repeated reflow recaptured a different center card, and a simultaneous query
publication could acknowledge its correction against the preceding width before the queued layout
arrived. The resize owner now retains the same logical card while the scroll position matches it,
exposes that anchor to query capture, and publishes pending geometry and correction together.
Native scrolling still invalidates the retained position; compatible publication rebinds its revision.

The existing resize case now includes incremental shrinking/expansion and a query publication midway.
It reproduced the missing original card before the atomic-publication repair and passed afterward.
All 24 affected resize/header cases passed, including wheel input before and during deferred reflow.
The wall has 1,596 production lines and the reflow owner 377, with no inline tests; their existing
resize test file has 1,364 dedicated-test lines and its shared fixture 241.

The real Profile client exercised 180 size changes per layout through the normal window API.
Catalog revisions advanced during both runs. The original card remained selected as the anchor,
and final geometric drift was 0.8 logical pixels for equal-height and zero for square. The measured
build/raster P95 was 16.88/9.28 ms and 9.33/8.94 ms respectively, with no frame over 50 ms.
This identified temporary per-cell allocations as the remaining equal-height resize cost.
Application-level synthetic held timeline input, wheel input and actual window sizing are recorded
separately from native mouse automation; no hardware-held-drag claim is made for this instrumentation.

The row calculator now stores contiguous 64-bit widths and a row start ordinal instead of one
object per cell. Both loaded and query-wide layouts consume the same values; packing, minimum
widths and 32-bit snapshot storage remain unchanged. The existing geometry/resize cases passed.
A repeated retained-library run measured build/raster P95 of 15.13/9.25 ms for equal-height and
8.82/7.79 ms for square, meeting the 16.67 ms targets. Maximum build/raster was 28.98/11.62 ms
and 15.64/10.70 ms; neither run had a frame over 50 ms. Background revisions advanced in both;
final same-card drift remained 0.8/0 logical pixels, and the client reported no Flutter error.
