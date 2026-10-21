# First-import performance evidence

Status: diagnosis in progress

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

The measurements identify useful boundaries but do not establish why the affected computer is
slower. Its same-source operation profile and the production repair remain pending.

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
The affected cloud-client timing remains to be measured.

Reviewability at this candidate: the catalog-delta owner has 2,215 production lines and no inline
tests; its existing test owner has 1,882 lines and the new dedicated lookup regression has 103.
The acceptance test owner has 5,521 dedicated test lines, with five operation-reporting lines added.
No schema, engine, dependency, bridge or source-access contract changes are introduced here.
