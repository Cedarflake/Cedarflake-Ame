# Read-only real-library acceptance

Status: reusable source-safe verification procedure.

Resolve authorized `local-primary` and `cloud-primary` roots through the ignored local context.
Keep source media read-only and reject cloud-only placeholders before content access.

Current executions follow the standing read-only verification policy in `AGENTS.md` section 12.1:
known roots, explicit bounded scope and isolated derived storage, without separate per-run approval.

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

The retained combined-catalog verifier independently requires two distinct available roots and
rejects any overlap between its retained storage and either source root before opening the catalog.
It reads bounded production catalog pages; it does not start another source scan or materialize
previews.

Cloud-only files remain rejected by the Windows filesystem adapter before content access. Acceptance
storage is never cleaned automatically. An interrupted scan can therefore be resumed with the same
scan ID, storage root, source root, and `-UseExistingStorage`.

The exact token is:

```text
CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1
```

It guards against accidental execution and binds deliberate use of the prepared scope. Supplying it
under the standing read-only policy does not require an additional confirmation.

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

Bind the exact known source and isolated storage in the prepared scope before replacing these placeholders:

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
