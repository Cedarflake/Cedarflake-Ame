# Engineering rule organization

Status: documentation organization; no product acceptance claim

Date: 2026-09-22. Baseline: `c3586612cf6bd0857bc393779a6801364935f762`.

This record traces the contract reorganization and its checks. The
[roadmap](../roadmap.md) and [R2c execution plan](../plans/r2c-closeout.md) still own delivery.
Readability observations do not establish the cause of C05 or authorize a repository-wide rewrite.

## Ownership and retained clauses

| Baseline clause | Current owner and treatment |
| --- | --- |
| §1 document boundaries, roadmap continuity, private-root mapping | Root §1, condensed; complete roadmap/active-plan reads and private-path restrictions retained |
| §2 local-first product, replaceable engines, original media and later physical operations | Root §2, condensed; no change in source authority |
| §3 precedence, pre-edit checks, §3.1 original-history recovery | Root §3/3.1, condensed; no summary-only resumption or silent material conflict resolution |
| §4 scope, delegation, narrow complete slices, owning-cause repair, physical size | Root §4, condensed; one active delegate by default, no nested delegation, explicit authority for concurrency, no symptom patches or cosmetic splitting |
| §5 layers, stable identities, replaceability, bounded background work | Root §5; prose condensed, contracts and isolation retained |
| §6 media/filesystem safety and §7 persistence/migrations | Root §6/7, retained |
| §8 dependency/license admission and GPL reference boundary | Root §8, condensed; no new dependency admission |
| §9 component selection, pinned SDK proof, framework ownership, bounded gallery | Root §9, condensed; Material/SDK evidence and accessibility requirements retained |
| First §10 encoding/language defaults and §10.1 pinned tools/serial locks | Root §10/10.1, retained; section 10 now has an inclusive title |
| Second §10 Rust engineering | Root §10.2, unchanged rules; duplicate numbering removed |
| §11 configuration owners, format/names/comments, generated exclusions | Root §11, condensed; no formatter/linter configuration change |
| §12 proportional evidence, categories, source checks, heavy serialization | Root §12, retained |
| §12.1 command catalog, script/workflow naming, CI security, hosted ownership | [Quality command reference](quality-gates.md#repository-command-reference), moved in full except its heading and two navigation references; root §12.1 retains mandatory pre-execution reading, canonical entrypoints, locks, real-root authorization, and CI security |
| §12.2 implementation, formatting ownership, focused/lint/Daily gates, warning policy | Root §12.2, condensed; document-only validation distinction matches the existing roadmap |
| §13 ADR content and scope | Root §13, condensed |
| §14 branch/commit/push, rollback, publication, PR wording, excluded artifacts | Root §14, condensed; explicit user branch instructions still take precedence |
| §15 completion criteria | Root §15, retained with readability/behavior-evidence requirement |

The migration comparison checks the complete §12.1 body, not just the presence of script names.
Every command description and guardrail remains in the quality document. Its existing detailed gate
sections remain the procedure owners. The Daily section's obsolete process-ID/path cleanup and
temporary-storage deletion descriptions are reconciled with the current owned-Job runner and its
retained nonempty evidence. This is a documentation correction, not a cleanup-policy change.

## Added standards and scope

Root §11.1 requires readable decisions, named domain meaning, explicit async authority, distinct
identities, and preserved SQL/resource semantics. §12.3 requires behavior-preservation evidence
appropriate to the changed owner. `Code Review Rules` separates demonstrated bugs from maintenance
risks. §4 prevents these standards from becoming permission for unrelated refactoring.

The status/task-kind getter and preview queue comparator remain distinct deferred examples in the
[complete scope map](../plans/r2c-closeout.md#complete-scope-and-retained-readability-findings).
They are not rewritten or closed by this documentation slice. Scan and viewport workflow complexity requires
owning-boundary analysis under ADR 0025, not blanket shortening, merged generations, or format churn.
No new lint framework, universal function-size limit, analyzer suppression, schema, bridge, or
business-code change is introduced here.

## Instruction discovery and verification

The baseline root contract is 48,358 UTF-8 bytes. The revised root is 28,593 bytes (about 27.9 KiB).
The proposed 16–24 KiB range is an advisory editing target; retaining the contract's safeguards takes
precedence over reaching it. The revised root is below the documented default 32 KiB project
instruction budget. Nested files are discovered along the path from project root to working
directory; a same-directory override takes precedence. Merely adding instructions below the working
directory does not prove they load. See the
[official instruction discovery guide](https://learn.chatgpt.com/docs/agent-configuration/agents-md)
and [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference).

Local checks use installed `codex-cli 0.155.0-alpha.9`. No local `project_doc_max_bytes` override or
same-directory global/repository `AGENTS.override.md` was found. No global configuration was changed.
The CLI's `debug prompt-input` performs fresh prompt assembly without starting model work. Its
assembled text contains the complete revised root contract, including §11.1 and the final completion
section, compared against the file itself. This proves this installed CLI's root loading, not every
hosted or desktop launch configuration. The separate global file did not appear in that diagnostic
output; global-loading behavior remains unverified by this command. The current desktop task supplies
global guidance separately. Neither a historical truncation nor any functional defect is attributed
to the size finding alone.

Document checks cover the exact moved command body, retained section/heading identities, local
Markdown link targets, UTF-8/LF bytes, absence of trailing whitespace, and `git diff --check`.
This slice does not rerun or claim Daily, native browsing, real-library, or R2c acceptance. C05's
native failure, C01/C02, and external acceptance obligations remain open.
