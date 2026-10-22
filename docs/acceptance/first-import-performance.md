# First-import performance evidence

Status: production client verified; current-head hosted verification pending

## Baseline

The affected Windows trial uses the functional source at
`df8ac5e4aad8d36b73f4f3ee5529525f63a68751`, subsequently merged through PR #43.
Its Release client ran on two logical processors and 8 GiB RAM. The installed Flutter SDK was
3.47.7 / Dart 3.13.5; seven SDK-related package resolutions differed from the repository lock for
that trial. The original lock and analyzer configuration were restored after building.

The source snapshot contained 19,037 files, including 19,036 image extensions and one unfinished
download, totaling 49,463,644,965 bytes. Attribute preflight found no unavailable placeholders or
reparse directories. The client catalog records 794.052 seconds from scan admission to completed
publication, with 19,028 locations. It records 16 invalid capture-time and 21 metadata-parse issues.
The catalog extension counts account for all rows: the nine AVIF files were not admitted by the
current scanner, while the unfinished-download extension was recognized as an image. Metadata
issues did not remove accepted rows.
Original media was not edited. The gallery stayed empty during traversal and displayed images
after completion. Another image application was briefly used during the last part of the import;
this baseline is an observed client result, not an isolated hardware benchmark.

## Local diagnostic comparison

The corresponding local-primary subtree has different membership after local organization. A
complete isolated first import accepted 17,854 images from 17,953 visited entries in 52.109 seconds.
The test uses the optimized dependencies with the Ame test package at opt-level 0; it is diagnostic
evidence, not a Release-client speed claim. Four capture-time and eleven metadata-parse issues were
retained, publication completed, and fourteen bounded source-hash samples matched.

| Operation | Seconds |
| --- | ---: |
| Source discovery | 6.922 |
| Prior selection | 4.189 |
| Media inspection | 16.326 |
| Location staging | 14.678 |
| Directory persistence | 3.611 |
| Checkpoint persistence | 2.282 |
| Admission, final validation/publication, and other work | 4.103 |

The local measurements identify useful boundaries but do not predict the affected computer.

## Affected-computer operation profile

With the client closed, the unchanged functional source scanned the complete cloud-primary trial
collection into separate derived storage in 246.853 seconds. It accepted 19,028 images from 19,142
visited entries, retained the same 37 metadata issues, and passed fifteen source-hash samples.
This diagnostic also uses optimized dependencies with the Ame test package at opt-level 0. It
omits the client and its synchronization runtime, and follows earlier reads of the source; its
difference from the 794.052-second client observation is not an implementation improvement.

| Operation | Seconds |
| --- | ---: |
| Source discovery | 8.666 |
| Prior selection | 9.184 |
| Media inspection | 54.327 |
| Location staging | 115.170 |
| Directory persistence | 26.772 |
| Checkpoint persistence | 24.787 |
| Admission, final validation/publication, and other work | 7.947 |

The existing SQLite diagnostic recorded 175 automatic checkpoints taking at least 100 ms each,
totaling 135.890 seconds. These durations are nested in the operation totals above, not additional
scan time. This identifies repeated database checkpointing as a major cost. A separate complete
trial changes only the automatic checkpoint threshold from 256 to 4,096 pages, retains FULL
synchronization and ordinary final close, and records the longest checkpoint as well as throughput.
The complete comparison took 132.448 seconds with unchanged counts, issues and fifteen matching
source-hash samples. The 28 slow automatic checkpoints totaled 45.074 seconds, with a maximum of
2.474 seconds. Staging fell to 45.235 seconds, directory persistence to 14.586 and checkpoint
persistence to 8.233. Media inspection also fell to 37.859 seconds without an inspection change;
source-cache warming therefore contributes to the total difference. Both diagnostic workers
retired and restored the cloud checkout to its original source.

The production candidate limits the larger window to an admitted first import with no published
roots. Its lifetime belongs to the scan; all connections sharing the catalog's writer admission
observe it before writing, and retirement restores the ordinary threshold on the next write.
The production-client comparison below verifies the full import and interruption path.

Eight focused connection-policy tests pass, including scope sharing across retained and newly
opened writers, separate-catalog isolation, last-owner retirement, published/paused admission,
FULL synchronization, rollback, independent reads, reopen durability and the ordinary 1-MiB window.
The complete production-scope candidate also passed the local-primary scan: 17,854 accepted images,
the same fifteen metadata issues and fourteen source-hash samples, in 44.717 seconds. Its staging
took 13.377 seconds; media inspection varied to 14.771 seconds. This does not establish a local
total-time improvement over the earlier warm run.

The scoped policy owner has 72 production lines and 187 dedicated test lines. The catalog facade
has 3,544 source lines, writer admission 357, connection configuration 51, and scan orchestration
750; these owners have no inline test bodies. The policy lives in its typed scope owner; the
facades only acquire that scope or apply its current connection setting.

## Compiled prior-query reuse

The scan prior lookup now uses the connection's existing bounded statement cache for active,
normalized-handoff and legacy-handoff reads. Query precedence, bindings and current-snapshot
execution remain unchanged. The two focused regressions pass: 256 distinct absent file lookups
compile three statements, and warmed queries observe first publication and subsequent commits
from another connection without retaining results or parameters.

Repeating the complete local-primary import into a separate empty catalog accepted the same 17,854
images, retained the same fifteen metadata issues, and passed fourteen source-hash samples.
Prior selection fell from 4.189 to 1.119 seconds. Total time was 42.583 seconds, but media inspection
also fell from 16.326 to 11.071 seconds without an inspection change, so the full 9.527-second
difference cannot be attributed to query reuse. Location staging remained 14.789 seconds.
The affected cloud-client result follows below.

Reviewability at this candidate: the catalog-delta owner has 2,215 production lines and no inline
tests; its existing test owner has 1,882 lines and the new dedicated lookup regression has 103.
The acceptance test owner has 5,521 dedicated test lines, with five operation-reporting lines added.
No schema, engine, dependency, bridge or source-access contract changes are introduced here.

## Production-client comparison

The cloud-primary Release client used production source
`7c880e5db79b23c703b3e398c2a3d30d44ea8519`, with the same installed Flutter client and SDK resolution
as the baseline. Its rebuilt native backend was verified before launch. The original catalog,
including its WAL and shared-memory files, was preserved together and hashed before starting an
empty comparison catalog. The complete directory was selected through the actual native picker.

| Observation | Baseline | Candidate |
| --- | ---: | ---: |
| Scan admission to completed publication | 794.052 s | 145.448 s |
| Accepted and published locations | 19,028 | 19,028 |
| Invalid capture-time issues | 16 | 16 |
| Metadata-parse issues | 21 | 21 |

The candidate visited 19,142 entries. Its recorded interval includes one interactive pause and
resume: the client displayed the paused state at 8,355 accepted images, then continued from that
progress. It remained empty until successful publication, displayed all 19,028 locations, loaded
thumbnails, opened a selected image, returned to the gallery, and scrolled to another viewport.
The client then closed normally. This is a complete production-client observation, including the
existing synchronization runtime, rather than an extrapolation from a few files.
After comparison, the original catalog files were restored to the client's default location and
verified against their pre-test SHA-256 values. The candidate catalog remains separate for inspection.

A read-only SQL comparison against a separate copy of the preserved baseline found zero differences
in both directions across relative path, file size, modification time, image dimensions, capture
time/offset/source, and filesystem identity. All 37 issue records retained their original categories
and counts. The existing nine-file AVIF admission limitation remains unchanged.

The observed interval is 648.604 seconds shorter, an 81.7% reduction. These sequential runs are not
an isolated cold-cache benchmark: earlier complete scans warmed source data, the baseline included
brief use of another image application, and the candidate included the pause/resume check and
remote diagnostics. The checkpoint-only profile above identifies an independent database cost
reduction; the entire client difference cannot be assigned to one optimization or generalized to
other machines. The first gallery publication still waits for the full validated import.

The strengthened WAL regression crosses the 4,096-page checkpoint window with 1,800 durable
transactions, then checks WAL reuse, rollback, independent reads, policy retirement and reopen.
It passes locally. The production source also retains the eight passing connection-policy tests,
two prior-query regressions and complete source-safe diagnostics described above. The test-only
follow-up is `db6c109`; current-head hosted results remain a delivery gate. Production source `7c880e5`
passed all fourteen applicable jobs in [Quality / CI run 39680852729](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39680852729);
the three signing-dependent jobs were skipped by their existing conditions.

## One-minute target and continued diagnosis

The subsequent acceptance limit is 60 seconds for the complete cloud-primary first import in the
actual client, through validated publication. The 145.448-second candidate above does not satisfy
that limit. PR #44 remains draft while the remaining import costs are reduced and measured.

The next complete read-only diagnostic repeats production source `7c880e5` with the existing test
profile and isolated catalogs. A diagnostic-only inspection timer reports reads taking at least
20 ms without changing their behavior. Each experiment restores its modified source after retirement.

| Diagnostic | Total | Media inspection | Staging | Directory persistence | Checkpoint persistence | Slow automatic checkpoints |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Current 4,096-page window | 131.079 s | 41.739 s | 45.727 s | 15.245 s | 7.610 s | 45.664 s / 28 |
| Same window, 32 MiB page cache | 97.605 s | 26.522 s | 37.343 s | 13.637 s | 7.397 s | 44.023 s / 28 |
| 16,384-page window, same cache | 74.603 s | 27.269 s | 16.616 s | 6.606 s | 5.644 s | 15.264 s / 8 |

Automatic checkpoint time is included in the other operation columns, not additional time.
All three completed with 19,028 accepted locations and clean source restoration. Sequential source
cache warming affects the comparison: the cache-only run's 15-second media decrease is not
attributed to a database setting. The larger-window comparison retains similar media time and
directly observes substantially less checkpoint work; its longest measured checkpoint is 3.238 s.

The next production candidate scopes the 32 MiB cache to the importing connection, increases the
temporary window to 32,768 pages, and reuses the asset-insert and source-generation SQL statements.
It keeps the 128-entry durable traversal checkpoint, FULL synchronization, file validation and
atomic publication. The new window and actual client timing still require measurement; these
diagnostics do not establish the one-minute outcome.

The connection-policy regression passes all ten cases locally, including the two new cache-scope
cases. Its durable fixture now crosses the 32,768-page threshold twice with 16,384 transactions,
then verifies rollback, independent reads, bounded WAL reuse, policy retirement and reopen. The
initial-import owner contains 129 production/source lines and 269 dedicated test lines, without
inline tests. The scan facade remains 749 source lines, with no inline tests and its existing
5,521-line dedicated test owner. Location row writes have 137 source and 169 dedicated test lines;
source reconciliation has 343 source lines and 382 dedicated regression lines, without inline tests.
The 21 source-reconciliation, two row-write and one connected pause/resume regressions also pass.
The first sandboxed source-reconciliation invocation rejected preview fixtures at the existing
namespace ACL boundary; the same 21 cases pass in the ordinary Windows user context without any
permission or policy change. Hosted verification and the production-client comparison remain open.

### Continued production-client comparison

The cloud-primary Release backend at `d313d88210cdd5f21ca016fa5b73c3f9b7157754` completes the
same full import in **80.381 seconds**, measured from scan admission to completed publication.
The backend SHA-256 is `AA74E19DDCC9C6F7AB2744E848628AA9B1DC7D30C18F59AF8FD6EF8B18483FBA`.
It accepts and publishes 19,028 locations from 19,142 visited entries, with the same 16 invalid
capture-time and 21 metadata-parse issues. The actual gallery shows the complete catalog and
thumbnails. A read-only bidirectional SQL comparison across the same ten source/metadata fields
finds zero differences. The original catalog is restored with its recorded SHA-256 values;
the comparison catalog remains preserved separately. The client uses the same frontend and SDK
resolution described above. Its observed working set after completion is 248,496,128 bytes and
peak working set 330,448,896 bytes. This sequential warm-library observation still exceeds 60 seconds.

The source-inspection candidate at `b414384c86b7f33bff7e611650300f7534cd12a3` overlaps one scoped
reader with catalog writes through an eight-result queue. Its actual cloud-primary client import
takes **80.605 seconds**, with 19,028 published locations, 19,142 visited entries and the same
37 issues. The backend SHA-256 is
`28521AEB42B7C6A829935E874858A5434EA6459226C663D8F18C07AFBC82C37C`.
The complete gallery and thumbnails are visible after publication. Observed post-import working
set is 247,468,032 bytes and peak working set is 338,358,272 bytes. This sequential observation
provides no improvement over 80.381 seconds and still exceeds the one-minute limit.
The read-only bidirectional comparison finds zero differences in the same ten membership/metadata
fields. The original catalog is restored and verified against its pre-run hashes; the candidate
catalog is preserved separately.

The 50 focused local regressions cover the new bounded reader and failure protocol, entry metadata,
retained hard links and repeated resumption, pause/cancel, finalization windows, live file changes
and both serial/prefetched operation accounting. The diagnostic expectation was updated to charge
prefetch waiting separately while preserving exact membership and source-directory assertions.
The prefetch owner has 140 production lines and 148 dedicated test lines; traversal has 421
production lines, and the scan facade has 754 source lines with its unchanged 5,521-line test owner.
Entry processing has 212 production lines and 619 dedicated test lines across its existing files.
File preparation has 191 production lines. Operation accounting has 175 test-support lines and
348 dedicated test lines. None of these owners adds inline tests.

The ineffective prefetch machinery is removed after this comparison, preserving the earlier
catalog improvements. Fourteen serial operation-accounting regressions and the connected pause
and cancellation regressions pass after removal. Product Rust source returns to the measured
`d313d88210cdd5f21ca016fa5b73c3f9b7157754` implementation.

The prior candidate's [CI run 39720418839](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39720418839)
passed thirteen applicable component jobs but failed the ordinary-user real-journal restart test
with repeated `identity_journal_file_attributes` failures. Its 402 other synchronization/scan tests
passed. This failure remains under diagnosis; no rerun or reduced assertion establishes a pass.
The subsequent changed-head [CI run 39723215869](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39723215869)
passes all fourteen applicable component jobs and the aggregate gate, including the same journal
restart case. A focused ordinary-user local run also passes; the earlier failure has not reproduced.

### Serial import cost and progress commit boundary

The full cloud-primary read-only diagnostic on `257e02511c770e0fec9d3e79631e1f47b478240a`
takes 75.890 seconds with the same 19,028 published locations and 37 issues. The application crate
uses test-profile optimization level zero and dependencies remain optimized; this is a diagnostic,
not the Windows client acceptance result. The 15 sampled source hashes remain unchanged.

| Operation | Elapsed |
| --- | ---: |
| Source discovery | 4.830 s |
| Prior selection | 1.664 s |
| Media inspection | 33.154 s |
| Location staging | 9.336 s |
| Directory persistence | 8.958 s |
| Progress persistence | 4.860 s |
| Final validation | 4.856 s |
| Publication | 4.288 s |

The remaining 3.944 seconds include admission, events and scope retirement. Four observed slow
automatic checkpoints total 7.404 seconds, already included in the enclosing operations rather
than added to this table. The longest checkpoint takes 3.248 seconds.

A full 128-location batch previously committed immediately before its progress checkpoint. A new
regression observes that premature commit on the previous implementation. Staging now leaves the
full batch for the atomic progress transaction; an incoming 129th item flushes it before admission.
Root/size/source-state checks remain in `scan_staging.rs`, with the catalog facade delegating there.
No transaction spans source reads, and rejected commits retain the bounded batch for retry.
The staging owner has 120 production/source lines and no inline tests. The catalog facade has
3,500 source lines and no inline test cases. The dedicated checkpoint tests have 528 lines;
the 172-line row-write test owner retains its compiled-statement assertions with an explicit final
flush. Eight checkpoint, 21 source-reconciliation, two row-write, one connected pause/resume and
one cancellation regression pass.

### End of the optimization round

Further optimization stops at user direction on 2026-10-22 with candidate
`87180419a8a7e1f2946f1c3a1b8c92ebe175fa36` retained. Its isolated cloud diagnostic completed before
cleanup; the original media-inspector source is restored and the cloud checkout is clean. The
client's original catalog was already restored and verified against its recorded hashes after the
last client comparison. No additional client import is scheduled.

The latest progress-commit candidate has 33 passing focused regressions but no new actual-client
timing. The retained measured implementation's 80.381 seconds remains the applicable observation;
the earlier one-minute request has no passing client result. PR #44 remains draft.
