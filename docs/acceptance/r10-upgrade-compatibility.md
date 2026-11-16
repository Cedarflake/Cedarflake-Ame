# R10 upgrade and compatibility

Date: 2026-11-16

Scope: R10-03 in the [Beta readiness plan](../plans/r10-beta-readiness.md#r10-03--upgrade-and-compatibility).
The [roadmap](../roadmap.md#r10--beta-readiness) owns stage disposition.
[ADR 0015](../architecture/0015-windows-release-distribution.md) owns build identity and distribution;
[ADR 0027](../architecture/0027-windows-installer-lifecycle.md) owns installation and rollback.

## Build identity and runtime payload

About, licenses and startup-error copying share Flutter's compiled build name and number. The
candidate uses `0.1.0-beta.1+4`; its MSI version is `0.1.4`, following Alpha 3's `0.1.3`.
The existing tag/manifest and versioned-note admission remains the publication owner. This scope
does not publish a Beta or add the R10-04 diagnostics-export feature.

The Windows build supplies the selected MSVC toolchain's redistributable libraries beside the
application and the four required dependencies beside each CPU/GPU model runtime. The MSI also
supplies its broker's runtime in the protected broker directory, with
stable component identities and restricted-service read/execute permissions. Candidate, installer
and ZIP checks require the runtime files; installer policy checks both copies and their permissions.
The release reuse policy requires a fresh candidate when the runtime policy or CMake input changes.

The focused checks cover the compiled version presentation and error prefix, Alpha-to-Beta MSI
ordering, missing runtime rejection, broker runtime placement/permissions, and artifact reuse.
Twenty focused Flutter checks and the installer-payload, portable-archive, unsigned-payload and
reuse fixtures passed during implementation. The production Release executable reports
`0.1.0-beta.1+4` in both Windows version fields and includes the selected Release runtime DLLs.

The settings owner remains 275 production lines with no inline tests; its dedicated widget suite
is 1,276 lines. This change replaces its two version literals with a shared read-only build fact.
Installer layout admission is 138 production lines, with no inline tests and a 110-line dedicated
policy regression owner. Broker runtime component generation has its own 27-line owner. Native
admission remains a 135-line facade; service configuration, startup and previous-version recovery
are separate 186-, 143- and 104-line owners with no inline tests. Recovery admission has 57 dedicated
test lines alongside the retained native identity, privilege and lifecycle tests.

The namespace-authority owner has 43 production lines, its compatibility proof has 49,
and the historical repair transaction has 36. Their dedicated regressions have 110 and 143 lines,
with no inline tests in these owners. The existing 1,463-line queue-persistence owner receives
only a composition call. The migration coordinator has 4,213 production/declaration lines and 2,289
inline-test lines; new repair SQL remains outside it. Neither facade gains a separate state machine.

## Published baseline and disposable guest

The upgrade baseline is the published `v0.1.0-alpha.3` from
`e70b10e87936f335d7c85aa31a9ac17838c91980`:

| Artifact | SHA-256 |
| --- | --- |
| MSI | `7232636a4d5a3588ea18179a1e08c0cdeda771d997b4d28414456db36fd735c1` |
| Portable ZIP | `f11f62f0c87c0d25f20e377040a989d4e117b2740792f991896e76538e38e2a1` |

Installation and source mutation are confined to a disposable Windows 11 Sandbox. Networking and
host clipboard redirection are disabled. The guest uses four generated images from the existing
acceptance corpus, including an exact duplicate and three visually similar contents. No retained
real-library catalog, media, host service or host filesystem permissions participate.

The original Alpha 3 client created these records through its actual interface before shutdown:

- one favorite and its membership in one custom album;
- one deferred organization decision and its undo/history records;
- an exact-group preferred physical copy and an ignored-group decision;
- two retained contents in one similarity group;
- one completed copy, including one export, one item and four operation events.

The closed baseline catalog is schema 81 and passes SQLite `quick_check`. A bounded read-only
snapshot records the durable rows and SHA-256 of all four source images for subsequent comparison.
After retiring the first guest for compilation, a fresh guest reinstalled the unchanged Alpha 3 MSI
and restored the sealed four-image fixture. Its folder was reassociated through the existing picker
because the guest volume identity changed. All 13 compared durable tables matched their prior row
snapshots exactly; the gallery, one-image favorite and one-image album opened.
That relocated catalog is retained separately as a historical compatibility fixture. The final
package matrix starts from the sealed original fixture in another fresh guest; Alpha 3 opens its
retained gallery, favorite and album before the within-guest snapshot, without relocating its root.

### Baseline environment findings

The Sandbox system-volume root initially granted authenticated users object-level delete rights.
The existing installer correctly rejected that ancestry before modifying the installation. The
guest fixture removed that root permission while retaining the other entries; host ACLs and the
product's path-admission policy were unchanged.

The next installation reached service startup but failed with Windows error 1053. Direct execution
of the published broker returned `0xC0000135`; its PE imports require `VCRUNTIME140.dll`, which the
published package omits. Installing Microsoft's signed VC++ prerequisite in this guest allowed the
unchanged Alpha 3 MSI to install with exit code 0 and the client to populate the baseline. The new
app-local runtime payload addresses this demonstrated packaging omission.

Concurrent and single-job broker builds exhausted host commit memory while Sandbox was active.
The closed Alpha 3 fixture was archived and hashed before retiring that guest; the subsequent build
uses a process-scoped single Cargo job with Sandbox stopped. No user applications, global compiler
settings or paging configuration were changed. That build passed, including all three native runner
cases, both engine retirement cases and the actual Release-DLL/Windows-channel smoke without opening
a catalog. The packaging policy executable could not create its fixture in the default temporary
directory; the same 57 cases passed with process-scoped temporary storage under the leased acceptance
directory, which the subsequent package build also uses.

## Upgrade and recovery matrix

The first controlled failure immediately after old-product removal returned MSI exit 1603. Old
application and broker hashes and the exact catalog bytes were restored, but SCM reported SID type
`NONE` and an empty required-privilege list. Service type, account, path, description and ACL matched
the baseline. This exposed a missing native service-configuration rollback rather than a catalog
migration failure. The isolated fixture was restored to its admitted Alpha 3 configuration before
retesting; the failed candidate and evidence remain separate from the corrected package.

The local candidate uses base `48fddbe7fef45ab7dc95f376562091365dd1db48` plus this scope's
uncommitted changes; its receipts explicitly retain `dirtyWorktree: true`. The MSI contains 62
manifest files plus three protected broker runtime copies. Its real database passed policy admission
and 35 rejecting mutations; all 16 native checkbox transitions passed without installing on the host.
The corrected native installer passed 75 policy cases, including restoration admission and rejection
of unrelated identities, paths, accounts, service types, start modes and privilege changes.
The portable archive passed version, layout, runtime and broker admission.

Both controlled MSI failures subsequently restored Alpha 3: immediately after old-product removal
and after new-service configuration. Each returned 1603 at the intended fault, restored the old
executable and catalog hashes, and restored the broker registration, restricted SID, required
privileges and running state captured at upgrade admission. The successful upgrade then installed
all 62 manifest files and three broker runtime copies and started the new broker.

The first upgraded client exposed an additional catalog compatibility defect. Alpha 3's explicit
directory relocation had superseded a generation-one recovery queue before its inventory started,
but left its authority unretired. The schema-81 catalog remained intact and failed the existing
recovery-execution row check. Namespace retirement now retires these unstarted authorities in the
same transaction. A narrow compatibility repair requires matching superseded P2 work, no inventory
or old publication namespace, and a newer active generation of the retained root; full schema and
row validation must succeed before commit. The existing corruption checks remain unchanged.

| Catalog-repair candidate, before model-directory dependencies | SHA-256 |
| --- | --- |
| MSI | `d5f3a7281abc37b2220e79beb87c20b4f49286643e790184cde4ccfafebe0b59` |
| Portable ZIP | `f6d921812e72eb14fa1e31cff68650450c695ca1324680cdd75849eecd176bcd` |

The native installer DLL has SHA-256
`d6a567e406053ff582104bbaa04dc9e550440dc99820fa7b414135757c9292d9`, identical to the
rollback-qualified candidate. The catalog-repair package retains its admitted action ordering. The two
installer fault results therefore remain applicable after the separate catalog repair rebuild.

### Upgrade and recovery observations

The catalog-repair MSI upgraded Alpha 3 with exit code 0. All 62 manifest files and three broker runtime
copies matched their receipt hashes; the restricted broker started. The actual client opened the
four-image gallery, one-image favorite and one-image custom album. About and licenses both displayed
`0.1.0-beta.1+4`. All 13 durable tables and all four source hashes matched the pre-upgrade snapshot;
the catalog reached schema 83 with `quick_check = ok`.

The retained schema-81 relocation fixture also opened with that executable. Its unstarted
generation-one authority was retired while the active generation-two authority remained unchanged.
The same durable-table and source comparisons passed. Its derived previews refer to the preceding
guest volume; this fixture proves migration and retained decisions, not current-volume source binding.

A controlled SQLite trigger interrupted the 81-to-82 migration. The client showed its startup error
and copied `Cedarflake Ame 0.1.0-beta.1+4` plus the exact injected migration error. The catalog remained
at schema 81 with valid integrity, and a completed 81-to-83 backup receipt identified the recoverable
snapshot. Restoring that snapshot and removing only the injected trigger let the published Alpha 3
executable open the retained gallery. A subsequent Beta launch upgraded it successfully and preserved
the 13 durable tables and four source hashes.

Launching Alpha 3 against the upgraded catalog displayed the unsupported-version explanation and
copied `catalog_schema_unsupported: Expected catalog schema 81, found 83`. The catalog remained at
schema 83 with valid integrity and reopened with the Beta executable.

Repair restored a deliberately removed application `msvcp140.dll` and broker `vcruntime140.dll` with
exit code 0. All payload hashes matched again, the broker started, and the client reopened its
retained gallery. Durable records and source hashes were unchanged. Uninstall returned 0, removed
the service and owned installation directory, and preserved the exact closed catalog and source
bytes.

Portable replacement started from the published Alpha 3 directory, retained it as the preceding
package, and put the catalog-repair Beta ZIP in its place. With no installed broker service, the portable
client opened the retained gallery. Its schema/integrity, 13 durable tables and four source hashes
passed the same comparison after normal shutdown.

### Clean-runtime packaging

A fresh guest without a system VC++ installation could install the catalog-repair MSI, start its
restricted broker, import a generated image and open its preview. Model analysis then failed because
the worker admits only System32 and the selected model runtime directory for native dependencies.
Application-root MSVC libraries were outside that search. Native import inspection identified
`msvcp140.dll`, `msvcp140_1.dll`, `vcruntime140.dll` and `vcruntime140_1.dll`; their dependent imports
require no further non-system libraries. Supplying these dependencies in the guest's runtime
directories let actual classification finish without a system runtime installation.

CMake now deploys those four selected-toolchain files to both runtime directories. Candidate, ZIP
and MSI admission require every copy; portable admission also compares their lengths and hashes
with the application root and rejects extra directory entries. Archive preparation still admits
only its pinned engine files, and worker DLL-search restrictions are unchanged.

| Final local artifact | SHA-256 |
| --- | --- |
| MSI | `1fff74d348d1dcdf1c3a3f45c21676aa4aa1012825e5f5b99d583dd16f174fa4` |
| Portable ZIP | `0a92e126ddc0dbff8b30b12fb3ca2f201c9ac5a4dab272fcf9cb63f4acf4bc64` |

The final receipt contains 70 files plus three protected broker runtime copies. All 62 preceding
payload hashes and the native installer hash are unchanged; the eight additions are the four model
dependencies in each runtime directory. The preceding upgrade, migration, rollback and portable
observations therefore retain their executable inputs. The rebuilt candidate passed its actual
Release-bridge smoke and package admission.

A new guest installed this final MSI with exit code 0, verified all receipt hashes and started the
restricted broker. All four corresponding System32 runtime files were absent before and after
installation. Through the actual interface, the client imported the generated image and opened its
640-by-480 preview. Classification published one prediction and completed its work without a runtime
copy or prerequisite installation. The source hash remained
`8573d2870e8f89fcf96b35943ccbec70ce2218777308a8b6f006c4c86bd431de`.

Repair then restored deliberately removed runtime files from the application, broker, CPU runtime
and GPU runtime directories with exit code 0. All 70 receipt files and the three protected broker
copies matched again. The closed catalog and original image retained their exact hashes. Both
runtime DLLs loaded with the worker's existing restricted search, without adding the application
directory. The actual client reopened, discovered a second generated image and published its second
classification prediction; all four System32 runtime files remained absent.

Local qualification uses the receipt-bound artifacts above. Hosted checks attached to the delivered
revision own the exact-commit regression, analysis and build results; this local matrix does not
substitute for them.

### Combined historical recovery

A focused Windows SQLite fixture reproduced an additional schema-81 upgrade failure after local
package qualification: a replaced namespace's unretired authority and a completed journal recovery's
retained handoff could coexist. Each separate repair validated the whole catalog before the other
repair ran, rejecting the second known legacy residue with
`catalog_change_catch_up_contract_unverifiable`.

Historical repair now detects and applies both eligible changes in one immediate transaction,
rechecks the schema after acquiring that transaction and validates all rows before committing.
The same production `SqliteCatalog::open` fixture passed for schema 81 and current schema 83;
an additional unproven handoff rolled both repairs back, and reopening after successful repair
preserved the retired authority unchanged. All nine terminal-journal recovery cases passed,
including the existing older-schema, active-owner and failure boundaries.

This follow-up changes only historical repair composition. The package hashes above identify the
earlier binaries; installation, rollback, runtime packaging and UI observations retain their
unchanged inputs. The native SQLite regressions and current-head hosted checks cover this later
migration change.

## R10c integration — 2026-11-16

[PR 68](https://github.com/Cedarflake/Cedarflake-Ame/pull/68) was squash-merged as
`f26b6132fdef213c06cf2bdce1936d0ca8098162` from head
`391e3745093189af2d9c0fd75c5414ed20cfa4ba`. The merged tree matches that head exactly.
The local and remote `codex/r10c-upgrade-compatibility` branches were deleted, and the checkout
returned to `main`.

The user explicitly authorized integration before CI completion. At merge time,
[Quality / CI 1163](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43863270308)
had passed Flutter tests, Static, Windows scan/accessibility, production build verification and
the Rust Remaining partition. Rust Catalog, Schema/Queue/Organization and Scan/Sync/Classification
were still running. The linked run and
[post-merge checks](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43869904990)
own their final results; integration does not record those pending checks as passed.
