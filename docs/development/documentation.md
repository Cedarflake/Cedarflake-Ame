# Documentation standard

Status: binding repository writing and ownership requirements

## Choose the owner before writing

| Content | Owner | Keep out of this owner |
| --- | --- | --- |
| Durable repository rules and precedence | `AGENTS.md` | Task status, implementation rosters, command manuals and repair history |
| Current priorities, stage disposition, blockers and exit decisions | `docs/roadmap.md` | Completed maintenance narratives, detailed slice definitions, measurements and copied requirements |
| Product behavior, scope, wording and interaction rationale | `docs/product` | Execution diaries, temporary diagnostics and verification results |
| Accepted technical decisions and general ownership principles | `docs/architecture` ADRs | Delivery status, repeated review reports and unrelated implementation details |
| Scoped steps, dependencies and rollback units | `docs/plans` | Competing stage status or copied product/architecture contracts |
| Verification procedures, dated evidence, failures and provenance | `docs/acceptance` | Authority to expand scope, reopen a stage or override stable requirements |
| Versioned user-facing changes, download guidance and known limits | `docs/releases` | Implementation diaries, CI logs and audit records |
| Contributor workflows and repository/document layout | `docs/development` | Duplicate architecture decisions and delivery tracking |

Give each rule or fact one canonical owner. Other documents link to its specific section and explain
only why it applies locally. Update an existing owner before creating another document. A moved rule
keeps its authority, qualifications, rationale and applicable exceptions.

## Entry documents and scoped reading

An entry document contains its purpose and authority, shared rules, and a task-to-document index.
Put detailed procedures, surface contracts and subsystem rosters in cohesive linked files. State
that scoped files continue the parent contract and when they must be read. Keep common prerequisites
explicit; readers should not have to load every sibling file to find an applicable rule.

Split when independent tasks require different sections, a detail roster repeatedly grows, or finding
one constraint requires reading unrelated material. Split by workflow, surface or invariant owner,
not an arbitrary line limit or one file per class. Keep a complete procedure, transaction or lifecycle
together. An index must make the required owner easier to find rather than add another link chain.

Use searchable module names and narrow responsibility statements in owner rosters. ADR bodies retain
the decision, reasons, alternatives, consequences and verification/replacement strategy; scoped owner
files retain concrete implementation boundaries. Add new details to the matching owner.

## Write for the current decision

Start with the rule, behavior or result. Use concrete subjects and verbs; explain causes, constraints
and tradeoffs when they affect implementation or review. Prefer short paragraphs. Use a table for
lookup/comparison and a list for parallel requirements or ordered steps. Avoid repeated summaries,
introductory filler, slogans and sections that restate their heading.

Keep names and identifiers stable. Follow the existing document language and UTF-8/LF convention.
Commands and examples must work within their documented preconditions. Include examples when they
clarify a rule or meaningful exception. Do not restate code line by line or narrate editing, review
rounds, conversations or agent actions as policy.

Distinguish requirements from evidence and unresolved observations from demonstrated causes. Record
dates and exact revisions where evidence depends on them. Never copy a historical pass into a
present-tense status. Preserve meaningful failures and superseded-decision reasons in their owning
record while replacing obsolete active instructions.

## Update rather than accumulate

Edit the owning paragraph and relevant links when changing a rule. Do not append a dated amendment
that leaves contradictory instructions above it. An ADR may retain necessary historical context,
with the active decision stated clearly and the superseded scope identified.

Update the current roadmap row or queue entry. On completion, retain the disposition and evidence
link; move useful delivery details to the owning acceptance record. A roadmap blocker identifies
the affected outcome and resolution condition. Detailed troubleshooting belongs in evidence, and
raw temporary logs stay outside tracked documents.

Add acceptance evidence when inputs, results or limitations change. Group related checks under one
outcome and reuse the existing record. Retain its meaningful initial failure; avoid a report for each
command or copying results into the plan, ADR and roadmap.

Do not append status, chronological fixes or command catalogs to `AGENTS.md`. Add a short scoped
reference there when a durable requirement needs discovery, and keep its full rule in the owning
document. Update the documentation index when a new owner changes the reading route.

## Verify a documentation change

- Check that every moved requirement, guard, exception and rationale remains in one owner.
- Check local paths and section anchors from entry documents and affected callers; preserve useful
  old entry anchors or update their references together.
- Walk representative tasks from `AGENTS.md` or the index to the exact applicable rule without
  loading unrelated sections.
- Inspect for unintended policy, scope or acceptance changes; validate UTF-8, line endings and
  whitespace. Record meaningful routing/preservation evidence in the existing acceptance owner.
- Use document checks for documentation-only changes. Product builds or runtime acceptance are
  needed only when a changed executable example or policy actually affects them.

Review growth by reading cost and responsibility, not line count alone. A short file with a dense
command table can need a separate reference; a cohesive contract need not be split solely for size.
