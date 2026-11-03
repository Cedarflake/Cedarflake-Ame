# Project organization verification

The [roadmap](../roadmap.md#project-organization-after-r5) owns activation and exit decisions.
The [execution plan](../plans/project-organization.md) owns O1–O7 and
[ADR 0037](../architecture/0037-feature-module-dependency-boundaries.md) owns the target rules.

## Entry verification — 2026-11-03

The implementation branch is `codex/project-organization`, created from merged R5 baseline
`f74146a4`. R5 product behavior, retained catalogs and original media remain unchanged during entry
verification. Product module migration has not begun.

The final R5 [workflow run 41602861678](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41602861678)
reports a Windows Scan failure. Its diagnostic artifact records two passing workflows, a native
picker cancellation failure (`Cancel button was not found`), and a subsequent controlled import
remaining in `choosingDirectory`. The original script did not distinguish an absent dialog from an
unready button, and acknowledged a delivered click without observing dialog closure. These logs do
not establish why the first dialog action timed out.

On local-primary, the first diagnostic invocation stopped before native automation because the
Windows PowerShell helper lacked a process-scoped execution policy. A second invocation, with the
policy scoped to that test process and restored afterward, passed all four existing scan workflows
in 67,054 ms, including preparation and owned process retirement. The CI timeout was not reproduced.

### Native picker verification boundary

`integration_test/support/NativeDirectoryPicker.cs` owns one observed dialog's action and retirement.
The helper waits for a visible, enabled button, bounds native message delivery, and acknowledges
success only after the same process-owned dialog is destroyed. Failure retains its original stage
and attempts bounded cancellation of that observed dialog. An absent or ambiguous dialog grants no
cleanup authority over another window. The operation retains its 12-second deadline and has a
separate two-second failure cleanup bound inside the existing 15-second caller deadline.

`control_native_directory.ps1` compiles this owner before readiness and reports the action stage,
elapsed time and cleanup result. Its Dart launcher scopes execution-policy admission to the helper
process, following the existing native accessibility helper convention. It does not change machine
policy. No UI Automation tree is opened, preserving the Flutter semantics-ownership assertion.

The eight focused cases cover delayed readiness and closure for both actions, an ignored
confirmation, failed delivery with failed cleanup, a missing dialog, an unready button, closure
before the requested action and original-error preservation. They pass in both PowerShell 7 and the
helper's actual Windows PowerShell host. The existing Static gate runs the same regression owner;
the real Windows Scan gate retains all existing assertions.

The changed real Windows Scan workflow passes all four tests on local-primary in 67,965 ms,
including preparation and owned process retirement. Run `933f611439a54ce89cfde25904b5a55e` reports
exit code zero and no cleanup failures. It covers native cancellation, controlled directory
selection, scanning, preview/source checks and the retained organization assertions. The helper now
uses its own process policy; this run required no caller environment adjustment.

The support action/native owner has 179 implementation lines, no inline tests and 101 dedicated
test lines. The PowerShell wrapper has 34 lines, its Dart process launcher 81 and the focused gate
14. These are test infrastructure, not additional product orchestration.

Submitted-commit CI remains pending. This verification repair does not establish O1–O7 completion
or a passing final R5 hosted result; the original CI timeout remains distinct from the reproduced
false-success protocol defect and local script-startup failure.

## O1 Dart dependency foundation — 2026-11-03

The checker inventories 417 Dart modules: 362 handwritten modules and 55 explicitly named FRB
outputs. Its baseline retains 124 exact migration entries (46 cross-feature, 69 layer, seven
external-library, one shared-feature and one cycle), with named responsibility, reason, regression
owners and O2–O6 removal conditions. The baseline Git blob is pinned independently of the mutable
active-ID policy; an arbitrary fresh violation list cannot replace it. The source/target and
physical-size inventory is in the [execution inventory](../plans/project-organization-inventory.md).

The 17 focused regressions pass through `quality_test_flutter.ps1`. They cover admitted layers,
multiline/raw/package URI resolution, all conditional branches, transitive exports, parts, complete
cycles, explicit cross-feature contracts, shared visuals, desktop contract admission, exact generated
outputs, invalid syntax, new violations, stale/duplicate exceptions and immutable baseline identity.
Scoped analysis reports no issues; the repository dependency command passes against the current
124 active exceptions. These checks add no product behavior, catalog/schema or runtime changes.

`analyzer` remains at its existing locked version 10.2.0 and hash; only its dependency classification
changes from transitive to direct development use. The existing Static/Flutter workflow owners run
the submitted checks. By current user direction, their pending results do not hold subsequent work.
The Rust portion of O1 is recorded below.

## O1 Rust dependency foundation — 2026-11-03

The Syn-based check covers the existing native domain, ports, application and bridge API modules.
It resolves explicit references through grouped imports, renames, re-exports, module globs and
block-local aliases, including macro paths. Test-only declarations are excluded without excluding
potential production platform/feature branches. Source, owning function/type, resolved target and
rule are reported together. Existing locked Syn/quote dependencies are reused without a Cargo
manifest or lockfile change.

The fixed R5 baseline has 279 entries: 26 named catalog-session, synchronization and classification
composition edges; the held source-handle contract in `PreviewStore::materialize`; and 252 individually
recorded application integration edges for O7 review. Domain/API contain no remaining rejected
explicit references. Existing API `StreamSink` use stays inside the bridge boundary. The
[execution inventory](../plans/project-organization-inventory.md#rust-composition-and-boundary-inventory)
names the retained construction/resource owners and their lifetimes.

The original inventory attempt exposed a parser recursion defect when a module and its re-exported
function shared a name. Resolution now distinguishes a module path prefix from a value export and
tracks alias cycles; the same-name regression passes. The repository scan then completed normally.
The gate pins the baseline blob and rejects new symbols even inside an admitted source file, missing
responsibility metadata, stale/duplicate/unknown IDs and directory-wide exceptions.

All 16 focused tests pass, including the current repository inventory; scoped Clippy for the test
and inventory example passes with warnings denied. The parser has 311 reference-walking lines,
246 name-resolution lines and 184 source-loading lines; policy comparison has 106 lines. These
owners have no inline tests and share 391 dedicated parser, policy and repository regression lines.
The check is part of the existing Rust `other` partition; hosted execution continues independently
under user direction.
This slice changes verification infrastructure and its ownership inventory, with no product,
schema, generated bridge or original-media changes.
