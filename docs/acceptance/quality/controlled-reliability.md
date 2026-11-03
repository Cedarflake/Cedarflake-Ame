# Controlled Windows reliability admission

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## R2c-R controlled local reliability gate

Run the non-external Windows 11 x64 checkpoint as an ordinary, non-administrator user:

```powershell
./tool/acceptance_run_r2c_change_driven_reliability.ps1
```

This runner accepts no source-root or catalog arguments and does not trust caller `PUBLIC`, `TEMP`,
or `TMP`. It admits only an ordinary Windows 11 x64 client workstation by cross-checking trusted
registry, native version API, product type, and SKU evidence. Loading its common module performs no
dynamic compilation. Explicit initialization first emits only the minimal Win32/NT declarations in
memory, then performs a read-only logical and physical binding of the repository `tool` path. Every
existing component is opened relative to a held parent with no-follow semantics, and any reparse,
volume transition, or terminal identity mismatch is rejected before the high-entropy bootstrap is
created with `NtCreateFile` relative to the final held physical parent. Only then does it rebind
`TEMP` and `TMP` for compilation. It revalidates the held identities after compilation, restores both
variables, and deletes only an empty bootstrap reopened relative to the held parent with the expected
identity. Compiler failure, unknown children, reparse state, or replacement retains an identifiable
bootstrap without traversal. The runner applies the same pre-write binding to both the logical
`SHGetKnownFolderPath` LocalApplicationData path and its handle-resolved physical filter path. It
holds every component and requires terminal volume/file identity equality before creating one high-
entropy disposable root relative to the final physical handle. The component and root handles remain
open without delete sharing for the complete runner lifecycle until cleanup begins; no `Temp` or
predictable owner directory is trusted or created. Parent-relative opens honor the live per-
directory Windows case-sensitivity flag; case-folded path text is never used as identity evidence.
Every fixture source, catalog, report, and per-case log remains below that
root. The runner validates the exact unique 19-case / 15-normal / 4-ignored matrix against current
attributed Rust functions, including report-binding tamper rejection, runs each fully qualified test
one at a time with `--exact`, and requires the final outer Rust result to report exactly one pass.
The four live/closed/overflow/million-record manual cases are explicitly ignored outside this
wrapper.

Windows share compatibility does not allow the runtime replacement-blocking root handle to request
`DELETE` while the production publication namespace guard also opens the root without delete
sharing. Cleanup therefore starts only after every owned child has exited: it releases the root
blocker, reopens the unpredictable child name relative to the still-held parent with delete access,
rechecks the exact file identity and physical path, then deletes that exact empty handle. This is
not
claimed as an atomic cleanup transition. A same-user racer can make cleanup fail and leave the owned
root for investigation, but cannot redirect deletion to a replacement identity; mismatch fails
closed. Failure evidence records the owned leaf and expected file-identity token because the
original
path may now name an attacker replacement. Cleanup never recurses from the original path after a
native reopen, identity, path, reparse, or emptiness failure. The high-entropy name, held parent
chain, and post-open identity check bound
that residual cleanup-only namespace race.

Each Cargo process is created suspended and assigned to a private kill-on-close Job Object before
it runs. A ten-minute per-case and one-hour total parent wall-clock deadline terminate only the
owned process tree, and `finally` restores environment/location/lock state and removes reports and
the exact nonce fixture. Evidence covers P0 latency and non-O(N) affected-path reads against a
4,096-entry unrelated baseline, durable P1 restart with nonce/PID/phase-bound worker reports, typed
overflow and bounded P2 ownership, a production-maximum-buffer million-record parser stream, 100
complete production observer starts with one metadata-only availability probe per actual poll and
zero source enumeration, target-scale P1/P2 priority, reset/trim, reconnect,
cancellation, replacement, retained-session same-volume multi-root isolation, placeholder
behavior, and Chinese/long paths.

The non-accessing guardrail is:

```powershell
./tool/acceptance_test_r2c_change_driven_reliability_guardrails.ps1
```

It rejects Windows 10/Server, unresolved SKU or inconsistent build evidence, non-Windows, non-x64,
administrator, caller-path, worker-environment, matrix drift, and zero-test inputs before Cargo. A
fresh PowerShell child proves the common module does not preload native types, hostile `TEMP`/`TMP`
sentinels receive zero writes, forced compiler failure leaves zero safely deletable residue, active
bootstrap replacement is blocked, and pre-created terminal plus production KnownFolder intermediate
junctions are rejected before a write without changing their sentinels. Workspace-only held fixtures
release the runtime blocker, move the original root, and put both ordinary and junction replacements
at the old name. After each first validation the replacement is swapped again; the stale expected
identity must retain the unknown object. Cleanup can proceed only through a held-parent no-follow
reopen with the newly captured fixture identity. Sentinel and process-output files use held delete-
on-close streams. One default-deny PowerShell AST source/call-closure audit covers every top-level
and function scope in the common module, runner, guardrail, all recursively reached local helpers,
and every exact dot-source dependency. A dot-source must resolve to a no-follow, same-volume,
identity-held repository-tool script whose exact content is audited; variable or otherwise
unrecoverable loading fails. Path text is never deduplication authority: the ordinal path map is only
an index, every request opens and binds the native snapshot first, and only its held volume/file ID
may return an existing source. Duplicate snapshots close before return, unique snapshots transfer one
owner, and exceptional pre-transfer paths close locally. Source-count admission remains before the
open. A real case-sensitive NTFS control requires `Safe.ps1` and `safe.ps1` to open as independent
identities and rejects a forbidden lower-case source; a host that cannot enable the directory flag
reports an explicit controlled skip. Ordinary duplicate, case-disabled spelling, and exceptional-
close controls also run. The audit admits the nine-source guardrail closure and rejects a tenth
source; extracting Cargo admission and report execution adds two owners to its former seven-source
closure. The remaining limits stay at dot-source depth 8, 256 KiB per source,
512 KiB total source bytes, 32,768 AST nodes, 128 functions, 512 scopes, and a 512-entry queue. AST
nodes are visited without materializing a collection, the root counts once, and the limit-plus-one
visit throws immediately while recording its attempted high-water. Independent empty and literal
shapes exercise limit-minus-one, limit, and limit-plus-one without reusing the production counter.
Every checked-add overflow or exceeded limit identifies its exact budget key, limit, and actual
value. Trusted cmdlets use an explicit leaf/module allowlist, and every unresolved command,
call operator, unknown helper, external executable, alias definition, dynamic script construction,
reflection invocation, or path-addressed mutation fails in every scope. The only process-creation
boundary is an independently digest-locked wrapper and native Job Object suffix. Before launch it
validates one exact Cargo matrix case or recovers and audits the final actual PowerShell `Command`,
canonical UTF-16LE `EncodedCommand`, or native no-follow `File` snapshot and arguments; nested or
unrecoverable execution fails closed. A file-backed child retains the audited terminal and every
parent handle without delete sharing, denies terminal write sharing, and reopens each wrapper and
payload source relative to its held parent to revalidate volume/file identity immediately before
native process transfer. Terminal reparse, post-audit terminal replacement, post-audit parent
replacement, or an exceptional partial-open fails closed and releases only the handles opened by
that attempt. Forty-five adversarial source/runtime fixtures cover module qualification, remove
aliases, alias definition, variable call/dot-source in every scope, `IEX`, script-block and remote
invocation, nested commands, PowerShell/pwsh/cmd `Start-Process`, `System.IO`, `cmd del`/`rmdir`,
`robocopy /MIR`, reflection, unknown helpers, and the exact `Add-Type`, move, and create boundaries
while all real child payloads remain green. Injected faults require the volume handle to close
before retained transfer and the compiler bootstrap owner to close before later initialization,
both in the parent process before controlled native types exist. The failure itself must report the
owned leaf and expected identity. The owned-name contract admits only the exact fixed prefix and two
32-character lowercase-hex fields, plus the exact internal moved-fixture form. Malicious NT leaves,
including ADS, control, separator, dot, trailing dot/space, arbitrary ASCII suffix, uppercase hex,
and non-ASCII syntax, are rejected. The guardrail also proves a blocked owned parent/child tree is
killed without terminating an unrelated process, timeout storage is cleaned, and it actually runs
the report-tamper Rust test with `--exact` and requires one pass, zero failures, zero ignored, and
zero measured tests. The Rust suite additionally locks the availability classifier to opaque
metadata evidence. Its exact `syn` 2.0.119/`quote` 1.0.47 source contract begins at the three
production owners and requires 17 named cfg-qualified function items plus 16 named support items to
match their token digest, local-callee closure, and call/receiver provenance. Any missing, changed,
ambiguous, newly reached, or newly referenced item fails with its readable key and summary. A full-
crate check rejects `Drop` and overloaded side-effect operators only for the six closure-reachable
evidence types. The protected closure contains no macro expression, admits only its locked
attributes and built-in derives, and rejects relevant local, parent, and imported macro-resolution
changes. The same structured contract locks the exact crate, adapters, local, domain, and metadata-
domain module declaration topology, including visibility, cfg attributes, inline/external shape, and
the one exact test-only item macro. File attributes, source-changing `cfg_attr`/`path`, arbitrary or
procedural attributes, `include!` items, extern-crate aliases, and nested/generated modules fail by
source key; the test and non-test production cfgs therefore load the same availability implementation.
Fixtures cover callee and receiver shadowing, unchanged-owner helper I/O, `Drop`, operator overload,
static/lazy side effects, aliases, same-module and parent macros, renamed macro imports, every module-
loading mutation, unknown expressions, and enumeration while the current production AST remains
green. The runner
restores environment state. `quality_lint.ps1` and Daily run only
this lightweight guardrail; they do not run the manual reliability wrapper. Controlled local
evidence does not substitute for an externally signed Release bundle, elevated SCM/service
lifecycle, real broker FSCTL, or separately authorized retained-root immutability and no-hydration
acceptance. See
[r2c-r-change-driven-reliability.md](../r2c-r-change-driven-reliability.md).
