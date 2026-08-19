# R2c-H large-library reliability acceptance

Status: acceptance evidence complete; independent audit pending

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
  distinct, and non-overlapping. The isolated storage must be empty.
- The retained source catalog is opened read-only and copied with SQLite's online backup API. All
  migrations, checkpoints, and queued catch-up intents are written only to that copy.
- The real-root phase performs downtime-catch-up discovery but does not lease or publish the
  resulting path, subtree, root, or freshness-unknown work.
- Source enumeration uses `symlink_metadata`, does not follow reparse directories, and is bounded at
  250,000 entries per logical root.
- Offline, recall-on-open, and recall-on-data-access files are counted but never opened. Source-byte
  hashes are selected deterministically from at most 32 locally available files under
  `local-primary`, each no larger than 64 MiB.
- Complete relative-entry metadata, Windows attributes, sampled hashes, and placeholder counts are
  compared before and after catch-up. A mismatch fails the gate.
- The wrapper enforces serial execution, elapsed-time and working-set ceilings, and keeps its report
  inside the isolated storage root.

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

The final Release-mode controlled Windows run passed on 2026-08-19. Observer startup took 5 ms.
Across 128 idle polls, P50 and P95 were both 1 ms. Across 24 create-to-visible samples, P50 was 20 ms
and P95 was 49 ms. The 288 rapid writes to 96 paths normalized to two retained rows carrying 681
coalesced observations. Maximum poll duration was 226 ms, restart recovery took 229 ms, observer
stop took 1 ms, and the SQLite family used 5,001,944 bytes.

The existing 10,000-file synthetic gate passed on 2026-08-19 before any target-root access. Fixture
creation took 12,485 ms, the cold scan 38,939 ms, the warm scan 33,775 ms, pause response 24 ms,
resume 31,629 ms, and cancellation 199 ms. The catalog family used 52,436,992 bytes and peak working
set was 17,981,440 bytes under the 512 MiB ceiling. The fixture verified its temporary source bytes
and entry count after completion.

## Target-library evidence

The authorization-bound Release-mode run passed on 2026-08-19 and recorded, without paths:

- `local-primary`: 35,084 entries, comprising 31,942 files and 3,142 directories;
- `cloud-primary`: 50,472 entries, comprising 49,643 files and 829 directories;
- zero entries carrying offline, recall-on-open, or recall-on-data-access attributes before or after
  the run;
- two complete metadata snapshots taking 9,258 ms and 9,207 ms, with identical UTF-16 relative
  paths, entry kinds, sizes, modification evidence, and Windows attributes;
- 32 deterministic `local-primary` byte samples with identical before/after hashes;
- a 6,451 ms read-only SQLite backup into new isolated storage;
- a 5 ms startup catch-up attempt. Both roots explicitly fell back with
  `usn_volume_open_failed`; no direct journal range or checkpoint was claimed;
- two durable root evidence-gap rows in the isolated copy, up from zero, with no authoritative lease
  or publication;
- 128 retained queue-metric queries with 1 ms P50 and P95;
- unchanged 358,528,856-byte SQLite family allocation across catch-up;
- peak working set of 52,543,488 bytes under the 2 GiB wrapper ceiling;
- exact confirmation that source entries, metadata, placeholder attributes, and sampled bytes were
  unchanged.

The standard workstation token could not open either volume journal. This is accepted only as a
truthful recovery-ladder result: both roots retained their last catalog and received durable
authoritative evidence gaps in the isolated copy. The run did not elevate the process or relabel the
fallback as direct catch-up.

## Repository gates

The complete Daily gate passed on 2026-08-19. Rust reported 394 tests: 387 passed and seven explicit
authorization or manual-performance tests were ignored, including the two R2c-H wrapper-owned
tests. All Flutter widget tests, the controlled Windows scan integration, the native Windows
accessibility integration, generated bridge compatibility, formatting, Clippy with warnings denied,
Dart analysis, script guardrails, and tracked-diff whitespace passed. The Windows Release build and
packaged bridge smoke test also passed. The target-library run and the final Daily rerun subsequently
passed; only the independent PR audit remains outstanding.

## Remaining platform boundary

USN journal access depends on the current Windows volume and process token. Permission denial,
unsupported filesystems, journal discontinuity, or an unprovable range is an accepted explicit
fallback to durable authoritative work, not a silent success. R2c-H measures that outcome but does
not elevate the process, hydrate cloud content, or turn a fallback into a journal-hit claim.
