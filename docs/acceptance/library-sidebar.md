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
