# Installed journal broker acceptance

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## Installed journal broker acceptance

The real SCM and `FSCTL_QUERY_USN_JOURNAL` / `FSCTL_READ_USN_JOURNAL` path is a separate,
authorization-bound Windows 11 x64 acceptance. It never targets `local-primary` or `cloud-primary`.
Obtain V1 and V2 pre-signed acceptance bundles from the protected production signing pipeline,
pre-create one empty workspace directly below the current temporary directory, then launch an
already elevated PowerShell and pass every disposable path explicitly:

```powershell
./tool/acceptance_run_windows_journal_broker.ps1 `
  -PreSignedBundleV1Path "<protected-path>\AmeBrokerAcceptanceBundleV1" `
  -PreSignedBundleV2Path "<protected-path>\AmeBrokerAcceptanceBundleV2" `
  -ExpectedPublisher "<exact production publisher subject>" `
  -AcceptanceWorkspace "<pre-created empty directory below the current temp directory>" `
  -DisposableRoot "<workspace>\root" `
  -ExternalSiblingRoot "<workspace>\external" `
  -AuthorizationToken "CEDARFLAKE_AME_WINDOWS_JOURNAL_BROKER_ACCEPTANCE_V1" `
  -AcknowledgeDisposableSystemChanges
```

Both bundles must already contain `Application\cedarflake_ame.exe` and
`Broker\cedarflake_ame_journal_broker.exe`, all signed by the exact expected publisher, and must be
rooted in a physical tree whose owner and ACL prevent an untrusted user from replacing any ancestor
or payload. The broker SHA-256 must differ between V1 and V2; the current broker-only upgrade gate
requires the client SHA-256 and compatible protocol contract to match. The runner installs and
repairs V1, upgrades to V2, then verifies the installed hash, protected identity manifest, protocol,
service restart, and limited V2-compatible client path. The repository runner has no certificate
creation, trust-store mutation, private-key, or signing path. A dedicated VM may prepare such a
bundle as an offline prerequisite, but that
workflow is outside this runner and must use independently protected signing input.

The harness does not auto-elevate. Before system mutation, it requires the exactly counted broker
integration matrix and installer rollback guardrail to pass. It rejects a pre-existing fixed
service, installs through the normal installer verifier, exercises missing-binary repair and a
same-publisher upgrade, then uses an exact one-shot Task Scheduler entry with InteractiveToken
and RunLevel Limited to run the production-pipe client without administrator elevation. The client
itself rejects an elevated token. The parent receives the bounded result only through its own
one-instance named pipe with a protected DACL, and validates a random nonce, task instance, pipe-
observed client PID, process creation time, fixed protected client path/hash/signer/protocol,
limited token, and current Task Scheduler result before acknowledging success. No user-writable
result file is acceptance evidence. The client exercises Query/Read/cancel and incompatible
protocol, restarts SCM and repeats the positive path, compares SHA-256, length, last-write time, and
entry counts for the disposable root
and an external sibling, then independently removes and confirms absence of the exact scheduled
task, service, installed Application bundle, and fixture paths. The external pre-signed bundle is
read-only input and is never removed by the harness. During system-bound work, the harness pins the
workspace ancestor chain against rename, replaces the workspace DACL with SYSTEM/Administrators
full control and limited-user read/execute only, deletes fixture entries through no-follow handles,
then restores the original workspace descriptor in `finally`.
`./tool/acceptance_test_windows_journal_broker_guardrails.ps1` exercises `-ValidationOnly` without
invoking Task Scheduler or SCM mutation. Without an externally pre-signed protected bundle,
production admission remains unexecuted and the guardrail is the only permitted local path.
The guardrail runs under both hosted Windows PowerShell 5.1 and PowerShell 7. It proves the
runtime-specific secure pipe constructor or ACL factory and ACL reader yield one protected DACL
with exactly SYSTEM/Administrators `FullControl` and current-user `ReadWrite|Synchronize`; it never
falls back to an unsecured pipe or repairs access control after creation.

The deterministic prerequisite matrix verifies unsigned watcher-first broker activation and explicit
`LiveOnly` on actual broker absence or protocol failure. Native regressions cover retained caller
process exit and directory-list permission revocation under captured impersonation. It also drives volume-wide root-unrelated records through bounded partial
pages until the captured end. The installer guardrail retains its eight install/repair/upgrade crash
cases. Its transaction-level matrix invokes the complete uninstall, injects crashes before and after
each of seven mutations and after each completion write, and resumes the persisted marker through
dead-owner recovery. Both present and initially absent install trees, committed recovery, marker
deletion before and after its boundary, exact parent-prestate restoration, and contradictory
phase/pending-operation failure are covered. Completed postconditions are re-verified unless a
strict persisted later operation specifically supersedes them. These checks do not invoke real SCM
or FSCTL operations and do not substitute for the elevated acceptance above.

The installer guardrail also invokes `release_test_journal_broker_identity.ps1` for unsigned and
explicit-publisher policy, passive inspection, V3 manifests and V2 rollback identity. Its
`release_test_journal_broker_uninstall_policy.ps1` companion verifies default broker-only removal,
explicit `-RemoveApplicationBundle`, schema-2 recovery and invalid schema-3 intent. Default removal
must make zero Application-tree accesses, including after interruption. These focused checks run
inside the canonical lint invocation as well as independently.
