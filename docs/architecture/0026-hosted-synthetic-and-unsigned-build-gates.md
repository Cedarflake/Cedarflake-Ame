# ADR 0026: Separate hosted workload coverage from release authority

- Status: Accepted
- Date: 2026-09-06

## Context

The serial workstation Daily gate intentionally omits expensive benchmarks and authorization-bound
acceptance. Reusing only that gate in hosted PR checks left synthetic workload and optimized Windows
build evidence manual. Workstation resource limits are not a reason to omit safe workloads from
isolated hosted runners. An ignored test is not necessarily a runnable benchmark: some are child
entrypoints, historical scenarios, or consumers of private source-library inputs.

## Decision

Ordinary PR, main-push, merge-queue, and manual quality runs require three independent synthetic
workloads and a credential-free unsigned Windows x64 Release build alongside every Daily component.
The stable Windows Gate aggregate fails if any required job fails, is cancelled, or is skipped.
Workloads run in parallel on separate runners; cases within one workload remain serial. Workstation
Daily stays serial and does not start these expensive jobs implicitly.

The synthetic entrypoint admits exactly three named cases: a generated high-resolution JPEG,
10,000 temporary images with scan/pause/resume/cancel checks, and one million generated USN records
through the production parser and injected backend. Each case must be discovered and executed as
exactly one test with workload evidence and zero failed or ignored results. No blanket
`--include-ignored`, arbitrary filter, real-library path, or authorization token is accepted. The
million-record case may use this additional synthetic entrypoint without claiming the complete
ADR 0024 Windows 11 ordinary-user reliability gate. Its acceptance-matrix membership is unchanged.

Execution has bounded parent deadlines, an owned kill-on-close process tree, bounded captured output,
and explicit workload peak-working-set checks. Compilation and direct workload memory observations
are reported separately, not misrepresented as complete compiler-tree or job commit-memory limits.
Measurements retain their original assertions: JPEG speedup output does not invent a latency
threshold, while the 10,000-image case retains its duration and storage limits. Each hosted case
preserves diagnostic output and structured evidence even on failure.

The pause/resume workload uses an unpublished first import in isolated fixture storage. A scan
updating a completed baseline has cancellation semantics and cannot substitute for that checkpoint
test. Explicit continuation must publish the complete fixture inventory and leave no unfinished
scan, independently of the cold/warm replacement measurements.

The unsigned gate builds the application and broker from the checked-out revision, verifies x64
payload, dependency freshness, and packaged Rust DLL identity, then tests only the isolated Rust
bridge and native accent channel. It never loads a retained catalog. Its evidence is not a signed
candidate, installability result, or Windows 11 client acceptance. It has no signing secrets,
Environment, SCM operation, publication, or artifact handoff into the protected signing chain.
That chain remains unchanged under ADR 0015.

### Tests outside hosted workload admission

- Historical R2c-H controlled execution uses an initial-inventory test runtime. R2c-M controlled
  restart assumes recovery without a persistent journal session. These scenarios need ADR 0024
  adaptation; enabling an old automatic scan to make them pass is unacceptable. Ordinary M metadata
  fixtures and their owned children remain in Daily.
- The three R2c-R live/closed/overflow scenarios retain their Windows 11 x64 ordinary-user,
  fixed-NTFS, known-folder, nonce, and owned-process acceptance entrypoint. The hosted Server image
  is not that client environment. Platform bypass or simulated success cannot replace it.
- Real source libraries, retained catalogs, real cloud no-hydration, signed bundles, and installed
  service/FSCTL acceptance keep their explicit input and authorization requirements.
- Child entrypoints execute only through their parent; an outer ignored count alone is not proof
  that a child was skipped.

## Alternatives and consequences

Running every ignored test was rejected because it mixes incompatible evidence and authority.
Weakening Windows 11 or protected-main signing admission was rejected. Keeping all heavy checks
manual was rejected because synthetic coverage would continue to drift behind routine CI.

Separate runners cost more hosted minutes and compile independently. No compiled application or
library output is restored as trusted evidence. The new workflows compose existing owners instead
of adding another workload state machine to the large quality/release workflow. Windows 11 client
execution needs a separately provisioned matching runner; a billed runner or self-hosted service
installation is not implicit in this decision.

## Verification and rollback

Focused guardrails prove exact admission, empty or malformed result rejection, failure propagation,
output/resource supervision, and payload identity/freshness. Hosted checks must execute the new
jobs on the current revision; code existence alone is not completion. Independent review covers
process ownership, source safety, artifact authority, and the stable aggregate. Rollback removes the
two added reusable workflow calls and their requirements together without changing application
behavior or weakening the signed release chain.

## References

- [GitHub hosted Windows Server image](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md)
- [GitHub needs context](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#needs-context)
- [ADR 0015: Windows distribution](./0015-windows-release-distribution.md)
- [ADR 0024: Change-driven continuity](./0024-windows-change-driven-library-continuity.md)
- [ADR 0025: Invariant-owned modules](./0025-invariant-owned-workflow-modules.md)
