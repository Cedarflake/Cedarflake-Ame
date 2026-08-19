# R2c-H large-library reliability acceptance

Status: complete and independently approved

Date: 2026-08-19

## Scope

R2c-H closes continuous-synchronization acceptance without changing the production catalog policy.
It combines deterministic fixtures, the existing 10,000-file synthetic performance gate, a real
Windows observer workload on disposable files, and a separately authorized read-only workload on
the retained two-root catalog.

The target roots are identified only as `local-primary` and `cloud-primary`. Machine paths,
accounts, and storage identities remain in ignored local context and are not acceptance artifacts.

## Guardrails

- The exact current R2c-H token and an explicit cloud read-only acknowledgement are mandatory.
- Both roots, the retained catalog, and isolated derived storage must be absolute, available,
  physically resolved, distinct, and non-overlapping. The isolated storage must be a pre-created
  empty directory; junction and equivalent-path aliases are rejected before any write.
- The retained source catalog is opened read-only and copied with SQLite's online backup API. All
  migrations, checkpoints, and queued catch-up intents are written only to that copy.
- The real-root phase performs downtime-catch-up discovery but does not lease or publish the
  resulting path, subtree, root, or freshness-unknown work.
- Source enumeration uses `symlink_metadata`, does not follow reparse directories, and is bounded at
  250,000 entries per logical root.
- Offline, recall-on-open, and recall-on-data-access files are counted but never opened. Source-byte
  hashes are selected deterministically from at most 32 locally available files under
  `local-primary`, each no larger than 64 MiB. The open handle is checked again and the reader is
  capped at 64 MiB plus one detection byte so replacement or growth cannot bypass the limit.
- Complete relative-entry metadata, Windows attributes, sampled hashes, and placeholder counts are
  compared before and after catch-up. A mismatch fails the gate.
- The wrapper enforces serial execution, elapsed-time and job-memory ceilings, assigns Cargo and all
  descendants to a kill-on-close Windows job, and keeps its report inside the physically resolved
  isolated storage root.

## Controlled evidence

The committed Windows fixture uses the production `notify` adapter and SQLite queue against a
temporary source root. It records:

- observer startup and 128 idle-poll samples;
- 24 create-to-visible samples with P50 and P95 latency;
- three rapid writes to each of 96 paths, normalized queue rows, and total coalesced observations;
- maximum synchronization-poll duration and catalog-family byte growth;
- observer shutdown latency;
- a second runtime consuming the durable backlog after the original observer stops.

The fixture requires event P95 below five seconds, observer shutdown below five seconds, normalized
storm work no larger than its affected-path set, evidence of actual coalescing, full final catalog
convergence, and a controlled catalog below 64 MiB.

The final post-audit-hardening Release-mode controlled Windows run passed on 2026-08-19. Observer
startup took 5 ms. Across 128 idle polls, P50 and P95 were both 1 ms. Across 24 create-to-visible
samples, P50 was 20 ms and P95 was 35 ms. The 288 rapid writes to 96 paths normalized to two retained
rows carrying 677 coalesced observations. Maximum poll duration was 234 ms, restart recovery took
237 ms, observer stop took 3 ms, and the SQLite family used 4,936,024 bytes.

The existing 10,000-file synthetic gate passed on 2026-08-19 before any target-root access. Fixture
creation took 12,485 ms, the cold scan 38,939 ms, the warm scan 33,775 ms, pause response 24 ms,
resume 31,629 ms, and cancellation 199 ms. The catalog family used 52,436,992 bytes and peak working
set was 17,981,440 bytes under the 512 MiB ceiling. The fixture verified its temporary source bytes
and entry count after completion.

## Target-library evidence

The final post-audit-hardening authorization-bound Release-mode rerun passed on 2026-08-19 and
recorded, without paths:

- `local-primary`: 35,084 entries, comprising 31,942 files and 3,142 directories;
- `cloud-primary`: 50,472 entries, comprising 49,643 files and 829 directories;
- zero entries carrying offline, recall-on-open, or recall-on-data-access attributes before or after
  the run;
- two complete metadata snapshots taking 31,011 ms and 22,599 ms, with identical UTF-16 relative
  paths, entry kinds, sizes, modification evidence, and Windows attributes;
- 32 deterministic `local-primary` byte samples with identical before/after hashes;
- an 8,327 ms read-only SQLite backup into new isolated storage;
- a 5 ms startup catch-up attempt. Both roots explicitly fell back with
  `usn_volume_open_failed`; no direct journal range or checkpoint was claimed;
- two previously persisted root evidence-gap rows in the isolated retained snapshot remained two
  after replay, with no duplicate row, authoritative lease, or publication. The original run had
  established the same rows from an empty queue;
- 128 retained queue-metric queries with 1 ms P50 and P95;
- SQLite family allocation grew by 20,632 bytes, from 277,921,792 to 277,942,424 bytes, while
  coalescing the replayed evidence;
- peak Windows job memory of 67,067,904 bytes under the 2 GiB wrapper ceiling;
- exact confirmation that source entries, metadata, placeholder attributes, and sampled bytes were
  unchanged.

The standard workstation token could not open either volume journal. This is accepted only as a
truthful recovery-ladder result: both roots retained their last catalog and received durable
authoritative evidence gaps in the isolated copy. The run did not elevate the process or relabel the
fallback as direct catch-up.

Because this authorization-bound phase intentionally did not lease or publish those gaps, it does
not measure target-library authoritative enumeration, media inspection, publication, convergence
time, or lease duration. Those target-scale timings remain extended R10 reliability evidence. R2c
correctness for slow authoritative ownership is instead covered by deterministic fake-clock queue
fixtures that cross the nominal lease deadline while foreground polling continues; target-library
queue and storage measurements must not be interpreted as an end-to-end recovery benchmark.

## Repository gates

The latest complete Daily gate passed on 2026-08-19. Rust reported 400 tests: 393 passed and seven explicit
authorization or manual-performance tests were ignored, including the two R2c-H wrapper-owned
tests. All Flutter widget tests, the controlled Windows scan integration, the native Windows
accessibility integration, generated bridge compatibility, formatting, Clippy with warnings denied,
Dart analysis, script guardrails, and tracked-diff whitespace passed. The Windows Release build and
packaged bridge smoke test also passed. No authorization-bound target workload was repeated during
this post-integration hardening; the retained source evidence above remains the separately
authorized record.

## Independent audit

The final independent review of implementation head `9911498` approved R2c-H with zero Critical,
High, Medium, or Low findings. Earlier review rounds identified and then verified closure of
physical-path alias isolation, process-tree ownership, bounded hash reads, and final resource-limit
sampling. The reviewer independently reran the R2c-H guardrails, focused Rust tests, Clippy with
warnings denied, and committed-range whitespace checks without accessing either real root.

## Remaining platform boundary

USN journal access depends on the current Windows volume and process token. Permission denial,
unsupported filesystems, journal discontinuity, or an unprovable range is an accepted explicit
fallback to durable authoritative work, not a silent success. R2c-H measures that outcome but does
not elevate the process, hydrate cloud content, or turn a fallback into a journal-hit claim.
