# Library sidebar maintenance

## Root expansion controls

The 2026-10-23 repair shows a root expansion control only when its bounded catalog window contains
child folders. Previously roots always exposed the action and queried their first folder window
only after expansion. The root loader now queues those reads serially, replaces superseded queued
revisions, and stops on disposal. Existing controller checks retain root, revision and pending-read
authority. A failed first read exposes the existing folder retry action.

The Windows Debug client built from `92089d4` plus this repair opened the existing isolated
18-image landscape gallery. Its root displayed `已同步` with no expansion arrow; opening the
overflow menu still exposed update and Explorer actions. Native logs recorded indexed-startup
preparation followed by successful background verification of all 18 entries, without mutations.
The check used existing catalog/storage and read-only media access, with no source changes.

An earlier Flutter hot restart left the Rust runtime owned by the previous Dart ticket and returned
`library_synchronization_state_raced`. Closing the entire process and starting the rebuilt client
resolved that diagnostic-session conflict. Runtime ownership guards remain unchanged.

Focused regressions cover initial folder discovery before expansion, cached-window reuse, nested
leaf controls, empty-root actions, retry reachability, serial/coalesced reads, disposal and existing
folder publication authority. All 37 focused cases passed. Hosted verification follows the
repair's normal main push.

Affected production owners contain 41 root-loader, 169 folder-controller, 436 navigation and
515 source-tile lines, with no inline tests. Dedicated regression owners contain 62 root-loader,
1,104 navigation-semantics and 287 unified-navigation lines; existing controller regressions remain
unchanged. This is a sidebar behavior correction, with no schema, bridge or dependency change.

## Per-source result activity

The intermediate repair replaced the all-root analysis feedback set with a source-owned set. Catalog source
versions, synchronization work and explicit updates identify affected roots. Their feedback survives
waiting, analysis, failure and query publication; completion and root removal retire it. Analysis
publication also reports a retired, unconsumed run as pending before its listener executes, closing
the listener-order gap that otherwise cleared the affected set too early.

All 20 focused activity, publication and existing duplicate-workflow cases passed, including the
two-source row regression through delayed publication. The Windows Debug client at `c88c9a7` plus
this repair imported the existing 600-image synthetic subdirectory into a disposable copy of the
18-image landscape catalog. At 125 discovered files the landscape row remained `已同步`; after
import and analysis both rows displayed `已同步`, with 618 physical images and 24 folded items.
SHA-256 checks confirmed all 618 source files unchanged. The copied storage initially retained the
old configured path and correctly rejected import; recreating only its disposable settings resolved
that fixture setup issue. The screenshot catalog was not used for the added registration.

Affected production owners contain 78 source-activity, 151 analysis-activity and 87 publication
lines, with no inline tests. Dedicated activity and publication regressions contain 301 and 199
lines. No database, bridge, source-access or analysis-scheduling contract changed. Hosted checks
follow normal main publication and remain separate from these client and focused results.

## Folder discovery lifecycle follow-up

Hosted checks exposed an initial timeline microtask reading disposed controller state after the
folder controller first created that dependency. The existing publication guard now runs before
that read. The five focused folder-retirement cases and analysis of the native fixture passed.
The Windows accessibility fixture now supplies one real child-folder branch and two leaf sources,
checking both the retained expansion action and its absence for leaves. Native execution remains
owned by the normal Windows accessibility check.

Affected owners contain 1,292 viewport production lines, 1,053 native fixture lines, 568 probe
lines and 255 dedicated folder-retirement test lines; the production owner has no inline tests.

The scroll-continuity fixture now supplies a published initial catalog revision and waits for
automatic root discovery before expanding. All four cases reproduced the missing expansion control
without that setup and pass with it. They also verify that expansion reuses the discovered window;
the existing leaf recovery, scroll retention, refresh failure and empty-result assertions remain.
This follow-up changes only test setup and assertions, with hosted verification following publication.

## Import completion and independent analysis

The subsequent correction removes analysis feedback from source rows entirely. A completed import
with synchronized catalog state stays `已同步` while duplicate analysis waits, runs, pauses, fails or
publishes results. The duplicate workspace retains its feedback, and analysis still refreshes the
gallery on retirement. Directory updates, unavailable sources and offline verification continue
to use their own existing states. This supersedes the intermediate source-owned analysis feedback
above and removes its now-unused tracking owner.

All 22 focused source-status and navigation cases passed. The two real-controller widget scenarios
retain completed import feedback, drive scheduler waiting/pause/running and successful or failed
analysis retirement, and hold the resulting gallery refresh open while checking both source rows.
Focused analysis reported no issues. A fresh native interaction was not completed: the computer-use
helper could not restore the minimized preview window, reporting user input before activation and
requiring activation before capture. The previously opened preview process and catalog remain intact.

Changed production owners contain 115 analysis-activity, 34 result-progress and 499 source-tile
lines, with no inline tests. Dedicated source-status and navigation regressions contain 239 and
1,091 lines. This change has no source-media, schema, bridge, dependency or scheduling changes.
