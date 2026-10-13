# R2c media and full-count publication follow-up

Date: 2026-09-06 to 2026-09-07

Status: implemented and independently reviewed; final full-head verification is tracked by
[PR #12 checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/12/checks). This record does not
declare R2c accepted.

## Why the earlier green gate was insufficient

The JPEG workload measured one high-resolution codec path. The scan workload used tiny PNGs;
neither established correct previews for all admitted encodings. Small import fixtures did not
hold the final publication transaction long enough to reveal periodic maintenance preemption.
The full-head result for `8bb974e` remains valid evidence for its executed cases, not proof that
the larger user workflows had no defects.

The reported runtime has 48,514 of 48,514 images checked, 50,515 visited files, and no completed
event. Its 398 `fullScan` polls extend to 1,427,579 ms. Most show no pending changes; pending/gap
counts become one near the end. These are synchronization polls, not internal scan-stage traces,
so they do not identify the exact SQL statement or time spent after the final image was checked.

## Owning findings

| Boundary | Confirmed mechanism | Required verification |
| --- | --- | --- |
| Idle journal maintenance | First-import completion requested Journal write priority before finding a candidate; this can interrupt and roll back Recovery publication even when completion returns false | Empty/running/uncovered cases do not acquire writer admission; real readiness still preempts and is revalidated inside the transaction |
| Full-count publication | After the last image check, one atomic transaction verifies staging and reconciles each physical identity; preemption restarts this work | At least 50,000 unique generated identities, overlapping real no-change completion calls, atomic final projection and preserved queued changes; separate timing from filesystem and UI acceptance |
| Completed result | Exact-path reconciliation can add a recovered image before publication, but the event used the earlier enumeration count | Return the actual count from the committing transaction; completed event and published snapshot agree without a second snapshot read |
| Finalization paging | Nullable-OR cursors can walk previously visited index prefixes instead of seeking from the last key | Preserve ordering, payload and limits; compare complete results and actual bundled-SQLite work for first and subsequent pages |
| Content admission | The hand-maintained signature list omitted ICO even though its decoder was admitted | Valid ICO and the other six formats work with correct, incorrect, unknown, and absent extensions; no additional codec is admitted |
| Negative media observation | Decoder EOF was treated like transient access failure; changing it to terminal without source-version revalidation would introduce another race | Versioned rejection evidence, final namespace/source checks, changed negatives handed to P0, and unversioned resumed evidence handled conservatively |
| JPEG fallback | Scaled-decoder errors were discarded and retried through a tolerant decoder that fills absent entropy with gray pixels | Original no-entropy failure remains a regression; malformed input produces no artifact; complete CMYK conversion and allocation limits remain tested |
| Retrying an old ready artifact | A failed forced regeneration could preserve an older, decodable but falsely successful preview | Conclusive current-source damage retires only its exact derived ownership; transient access/storage failures preserve healthy results; stale failures cannot downgrade newer publications |
| Native cache installation | Long temporary paths reach unextended Windows APIs; a missing-target probe masks the first installation error | Extended native paths independent of host opt-in; real cold installation, replacement, rollback, collision and sharing-error accounting |

The publication fix does not raise scan priority above live changes or disable actual preemption.
Candidate reads are only hints and release their snapshot before write admission. The authoritative
selection and multi-table completion remain atomic. No user catalog or source tree is modified by
these verification fixtures.

## Coverage and limits

Generated fixtures contain actual JPEG, PNG, WebP, GIF, BMP, TIFF, and ICO encodings. Adapter tests
decode resulting preview bytes and inspect image geometry and colors, staging cleanup, reservation
release, warm reuse, and unchanged source bytes. Application tests exercise catalog publication,
same-path replacement with restored modification time, old reference retirement, explicit retry,
and recovery after valid contents return. Positive ownership must be proven before a zero-reference
retirement assertion can establish that anything was retired.

The larger media workload uses six 3000 by 2000 sources and an ICO at its 256 by 256 container limit,
with explicit cold/warm and bounded-storage evidence. The separate publication workload generates
catalog observations, not 50,000 real source files; it does not claim a full native UI import or
retained-library acceptance. Both run on isolated hosted workers through exact workload admission.

Current thumbnails are static JPEG posters. Transparent-input and two-frame GIF cases characterize
the existing RGB/first-frame behavior, not alpha-preserving output, animation playback, every frame,
or every TIFF/WebP/ICO variant. Invalid headers and deterministic missing payloads are detectable
failures; decodability alone cannot certify that every bit matches an unknown original.
Ordinary warm cache hits must not decode the entire source again. Previously generated artifacts
therefore require explicit regeneration or normal source-version invalidation to reapply new
decoder policy; automatic global cache deletion is not part of this repair.

## Scan-cost boundaries

Foreground import currently persists bounded directory path batches, then visits and inspects
entries serially. It already reuses compatible metadata and stages locations in batches; preview
generation is separate. Replacing this with an extension-only filter or removing final validation
would change correctness, not merely improve performance.

The immediate paging correction keeps the existing indexes and divides first-page selection from
strict subsequent-key selection. An index scan is not evidence of cursor seeking: SQLite's
[query-plan documentation](https://sqlite.org/eqp.html) distinguishes the two. A system SQLite
3.51.1 memory-only diagnostic found repeated identity-page prefixes; the project uses 3.53.2,
so that diagnostic is not substituted for project-version regression or whole-import timing.

The existing Windows metadata iterator is a credible reuse boundary: its directory records expose
identity, size, modification/change times and attributes through
[FILE_ID_EXTD_DIR_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_extd_dir_info).
Those observations are hints, not permission to remove no-recall opens, namespace checks or final
source-version validation. Integrating them into foreground checkpoints, batching old-record
lookups and overlapping media workers requires separate measured evidence and a coherent task
completion protocol. It is not included as an unmeasured scanner rewrite in this repair. No MFT
authority, additional traversal dependency, media-first completeness shortcut or delayed-layout
placeholder behavior is introduced.

## Verification ledger

- The first compiled focused run passed four signature tests and two failure-classification tests.
  Eight of ten initial format tests passed. The two failures exposed an invalid RGB-in-ICO fixture
  and the real JPEG missing-entropy fallback defect. Neither failed result is counted as acceptance.
- The next compiled run passed the five media lifecycle tests, including fresh import and existing
  `.data`/extensionless replacement, six no-op/ready journal completion tests, two negative-roster
  tests, two retained-issue paging tests, and the inspection-engine revision regression. The
  completed-count equality assertion remains enabled and passes after the transaction receipt fix.
- The precise preview error classifier exposed an overbroad fixture expectation for forged ICO:
  unsupported bitmap features are not conclusive corruption. A separate source-pixel assertion used
  extension-based decoding despite intentionally mismatched JPEG bytes. These fixture failures are
  corrected without relaxing no-artifact, source-byte, pixel or ownership assertions. The failed
  preview suite's later poisoned-lock failures are not counted as independent production findings;
  the complete suite must be rerun. Independently executed corrupt-ready retirement and transient
  capacity/atomic-replacement retention tests pass.
- The five-case synthetic protocol, resource/process guardrails, summary-failure precedence and
  hosted extended-coverage contract pass under Windows PowerShell. Actual Release workloads remain
  separately required; compiler-free protocol success is not performance evidence.
- The frozen follow-up passes all 12 format adapter tests, 42 preview/application lifecycle tests
  with one separately authorized acceptance ignore, 62 existing scan regressions with two explicit
  performance/real-source ignores, and the pause/cancel finalization issue-count regression. The
  repeated preview suite has no poisoned-lock failures. Source replacement before the final guard
  cannot downgrade a newer ready result; this test is not claimed as a new failure-branch SQL-CAS
  fault injection.
- Bundled SQLite 3.53.2 passes both real-schema pagination regressions. For an exhausted cursor,
  directory/identity/positive-roster/negative-roster production progress observations are
  61/91/16/13 operations. The respective original nullable-OR statements execute
  3,853/5,961/3,885/3,109 VM steps and fail the same 128-operation seek budget. The tests also
  verify complete multi-page payloads, duplicate/NULL identities, other-scan isolation and the
  unchanged directory readiness/zero-limit rules. These are query-work observations, not a
  whole-import speedup claim.
- The actual Release publication workload passes with 50,000 staged and published identities,
  297 overlapping production completion polls, one publication call and one commit, no partial
  observations, and the original pending P0 intent retained. Fixture preparation takes 105,097 ms;
  the publication transaction takes 4,989 ms. The complete worker takes 111,714 ms with a measured
  primary-process peak working set of 19,255,296 bytes. These are synthetic catalog observations,
  not filesystem enumeration or real-library import timings.
- The first actual Release media workload fails during cold JPEG artifact installation, before
  multi-format acceptance. Its nested cache has a 257-unit target path and a longer temporary
  staging path. The production Windows installation passes unextended paths to native file APIs;
  its later missing-target metadata check also masks the original installation failure. The
  original long-path workload remains required rather than being shortened to bypass this case.
  The Flutter runner already declares `longPathAware`; this failure proves an independent native
  adapter's host-opt-in dependency, not the cause of the user's earlier illustration failure.
  The corrected adapter resolves only its existing cache parent, preserves the unchanged leaf and
  original installation failure, and leaves move flags, backup ownership and source-access rules
  intact. Independent review covers its four dedicated regressions. All nine native installation
  regressions pass, including the existing replacement-failure suite. The first run's fixture
  expected an unscaled 64 by 48 output from the existing 128-pixel bucket; correcting the expected
  128 by 96 geometry changes no production resize policy or long-path assertions.
- The local lint run passes its compiler-free guardrails and formatting check. Clippy identifies
  one collapsible conditional in the benchmark poll-worker cleanup; the equivalent let-chain
  correction passes the repeated all-target/all-feature Clippy check with warnings denied. Dart
  analysis then passes with fatal warnings and information enabled. This resumed sequence is not
  reported as a second uninterrupted invocation of the entire lint script.
- The local native scan partition passes all three controlled interactions, including manual
  continuation and durable cancellation. Native accessibility passes both tests and all ten UI
  Automation phases without an invalid AXTree update; its completion records no run, cleanup or
  probe-cleanup failure. These executions use isolated fixtures, not a retained source library.
- Hosted run [34047864932](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34047864932)
  for `12e36da` passes Flutter, both native partitions, unsigned Windows x64 Release and all five
  exact synthetic workloads. Seven-format evidence records seven cold generations, seven warm
  reuses and seven unchanged sources. Publication records 50,000 identities, 145 overlapping
  polls, one call and one commit, zero partial observations and the original pending P0 intent;
  fixture preparation takes 20,078 ms and publication takes 2,218 ms. These are bounded synthetic
  results, not actual-library timings or a complete green gate.
- That run fails exactly one Rust test after 1,265 passes and 19 separate-workflow ignores: the
  exact availability source contract had not registered the new `media_signature` and test-only
  `media_fixtures` module declarations. A local invocation reproduces the same failure. The
  correction registers their exact visibility, attributes, paths and external-module shapes only
  inside the existing test contract. Production availability code, protected digests, validator,
  call closure and access capabilities remain unchanged. All seven focused availability tests
  pass, including the original failing test and 22 real source mutations covering missing,
  duplicate, broader, alternate, nested and generated loading. Independent review confirms the
  original fail-closed rules remain intact; the correction is not a test exclusion.
- Complete Daily and unsigned hosted gates still must pass for the final head through the linked
  checks. Independent cross-owner review covers the final keysets, source-negative lifecycle,
  decoder policy, committed counts, control counts and five-case CI boundary. Implementation or
  a prior green head does not substitute for execution evidence.

## Physical review

Counts include blank lines and comments. The non-inline section contains production code and any
test-only hooks colocated with it; it is not presented as pure production size. Separate test files
are listed explicitly so moving a test out of an owner does not hide its physical cost.

| Affected owner | Non-inline section | Inline test section | New focused boundary or dedicated tests |
| --- | ---: | ---: | --- |
| `jpeg_preview.rs` | 142 | 112 | `decode_tests.rs`: 87 |
| `preview_cache.rs` | 887 | 894 | Error classifier: 42 + 87 inline; format tests: 364; performance: 184 |
| `preview_cache/installation.rs` | 280 | 174 | Long-path installation tests: 243 |
| `persistent_journal.rs` | 3,571 | 4,710 | First-import completion owner: 220; dedicated tests: 279 |
| `scan_publication.rs` | 1,157 | 0 | Pagination tests: 407; performance: 328 |
| `application/preview.rs` | 670 | 1,047 | Failure policy: 29 + 42 inline; lifecycle tests: 311 |
| `scan_library.rs` | 1,213 | 0 | Media lifecycle tests: 679; finalization control tests: 96 |
| `scan_library/finalization.rs` | 438 | 33 | Rejected-input owner: 125; retained-issue owner: 118; dedicated tests: 157 |
| `local_files.rs` | 4,253 | 3,396 | Module-loading contract tests: 119; this correction adds no production behavior |

The 5,528-line SQLite facade and older large inline suites remain physical debt. The new
679-line application media suite is also explicitly visible: further expansion should separate
admission from replacement/retry fixtures rather than append another unrelated workflow. This
repair extracts owning invariants and tests, not an assertion that all oversized files are now
small or that the broader scanner pipeline has been reorganized.

The single delivery plan remains [the roadmap](../roadmap.md).
