# Read-only real-library acceptance

Status: completed for the two logical real roots on 2026-08-07

This procedure is the R1 gate between controlled fixtures and the user's real image libraries. On
2026-08-07 the user explicitly authorized read-only access to `local-primary` and `cloud-primary`
and instructed the project to continue. Their exact paths remain in the ignored local context. The
authorization covered the ordered acceptance runs below. It does not authorize source mutation,
placeholder hydration, or an unrelated future run after this acceptance sequence.

The acceptance harness calls the production Rust discovery, metadata, reconciliation, checkpoint,
and SQLite publication path without generating previews or modifying source media. It has no item or
entry limit, so a complete run may take hours.

## Safety contract

`tool/acceptance_run_read_only_library.ps1` refuses to start unless all of these conditions hold:

- the exact authorization token is supplied;
- the source root and acceptance-storage root are explicit absolute directories;
- acceptance storage is outside the source tree in both directions;
- a nonempty storage root is used only with `-UseExistingStorage`;
- a path containing `OneDrive` also has `-AllowCloudBackedRoot`;
- cancel and pause injection are not requested together;
- the scan ID is a stable, explicit identifier suitable for recovery.

Cloud-only files remain rejected by the Windows filesystem adapter before content access. Acceptance
storage is never cleaned automatically. An interrupted scan can therefore be resumed with the same
scan ID, storage root, source root, and `-UseExistingStorage`.

The exact token is:

```text
CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1
```

It is a guard against accidental execution, not a substitute for current user authorization.

## Controlled harness

Run the guard and terminal-state regression before a real-root run:

```powershell
.\tool\acceptance_test_read_only_guardrails.ps1
```

The harness uses the repository's application icon as a controlled source and verifies:

- refusal before source access when the token is wrong;
- refusal for a cloud-like path without its second acknowledgement;
- refusal for overlapping or unexpectedly nonempty storage;
- a complete scan publishes exactly one active location;
- cancellation leaves no staged location and does not publish the scan;
- pause persists state, explicit resume completes, and the final scan becomes active;
- source bytes and source directory entries remain unchanged;
- the report contains process-memory and source-integrity evidence.

All harness storage is constrained to a process-specific directory under `build` and removed after
the assertions complete.

## One-root command shape

The exact source must be within the current authorization before replacing the placeholders below:

```powershell
.\tool\acceptance_run_read_only_library.ps1 `
  -RootPath "<approved absolute source root>" `
  -StorageRoot "<dedicated absolute acceptance storage>" `
  -ScanId "<stable scan identifier>" `
  -AuthorizationToken "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1"
```

Use `-CancelAfter <visited-entry-count>` for a terminal cancellation trial. A later complete run
must use a new scan ID and may reuse the same storage only with `-UseExistingStorage`.

Use `-PauseAfter <accepted-image-count>` to inject a persisted pause and then resume the same task in
the same command. If the process itself is interrupted, rerun the original command with the same scan
ID and add `-UseExistingStorage`.

For the approved `cloud-primary` root, add `-AllowCloudBackedRoot`. This does not permit placeholder
hydration; it only acknowledges that the named directory is cloud-backed.

## Evidence

The tool retains `acceptance-<scan-id>.log` inside the named storage root. The report records:

- terminal status and whether the scan became active;
- visited entries, accepted images, structured issues, and staged or published locations;
- hashes for sampled locally available files; a cloud-only cancellation window may validly have no
  eligible sample and reports zero explicitly;
- one representative path and message for every structured issue code;
- total duration and accepted-image throughput;
- injected pause, resume, or cancellation response time;
- SQLite main, WAL, and shared-memory file size;
- peak observed test-process working set;
- count of source files whose bytes were hashed before and after scan completion.

Integrity sampling always includes the first eligible accepted image, then deterministically samples
approximately one in 1,024 relative paths, up to 64 files no larger than 64 MiB. Hashing is streamed
through a 1 MiB buffer. An unchanged sample supports the read-only evidence but does not prove that
every unsampled byte in the collection was unchanged; source-mutation absence is also enforced by
the production adapter boundaries and controlled full-source tests.

For a large run, `observed=false` in the memory line is an evidence failure and must be investigated.
A short controlled scan may finish between operating-system samples and report no observation.

## Completed run evidence

Retained derived storage remains machine-local and outside the repository and both source trees.
Its exact path is intentionally not repository documentation.

| Run | Visited | Accepted | Issues | Elapsed | Peak working set | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| local-primary cancellation | 565 | 512 | 16 | 1.567 s | 18,616,320 B | cancelled in 66 ms; zero published locations |
| local-primary cold scan | 35,084 | 30,629 | 828 | 106.905 s | 19,578,880 B | completed and active |
| cloud-primary cancellation | 512 | 420 | 8 | 1.205 s | 24,223,744 B | cancelled in 231 ms; local-primary remained active |
| cloud-primary cold scan | 50,384 | 48,384 | 1,264 | 154.698 s | 66,826,240 B | completed and active |

The `local-primary` cancellation run predated the visited-entry cancellation correction and reached
its trigger at 512 accepted images and 565 visited entries. The `cloud-primary` trial and current
tool semantics use a visited-entry threshold. A restricted-process cloud-backed probe returned
access denied without publishing data; the accepted runs used the explicitly authorized external
read context.

The completed scans retained 35 and 44 deterministic source-hash samples respectively. Their final
catalog contains two active roots and 79,013 active locations. The production `load_catalog` API
then loaded every location through 155 bounded 512-item windows at catalog revision 2, with no
duplicate location identity or gap. Both roots were reported available and every preview remained
pending, confirming that the catalog-only run did not generate a full-library preview cache.

Retained reports remain in the ignored acceptance storage under their historical scan IDs. Their
machine-specific names and paths are intentionally not repository documentation.

## Ordered real-root gate

The gate completed in this order:

1. use the authorized roots and the isolated acceptance storage recorded in the run log;
2. run the controlled harness;
3. resolve `local-primary` from the ignored local context, run a cancellation trial, and inspect
   the retained report;
4. run a complete cold scan of `local-primary` with a new scan ID;
5. confirm the authorized `cloud-primary` root remains available;
6. run a cancellation trial with `-AllowCloudBackedRoot` and inspect
   placeholder issues before considering a complete scan;
7. run the approved complete `cloud-primary` scan into the same catalog only with
   `-UseExistingStorage`;
8. verify the combined catalog through the Windows application before closing R1.

Completion authorizes roadmap progression to R2 under the existing project contract. It does not
authorize file deletion, movement, renaming, copying, full-library preview generation, or cloud
hydration.
