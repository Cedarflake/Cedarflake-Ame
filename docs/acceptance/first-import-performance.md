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
