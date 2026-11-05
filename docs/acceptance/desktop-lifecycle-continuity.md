# Desktop lifecycle continuity

This record owns post-stage desktop attachment and shutdown evidence. The [roadmap](../roadmap.md)
owns delivery status; [ADR 0036](../architecture/0036-durable-classification-dispatch.md) and
[ADR 0024](../architecture/0024-windows-change-driven-library-continuity.md) own runtime authority.

## Development hot restart — 2026-11-05

The Windows Debug client at `05038a3b` reproduced both failures after Flutter hot restart.
Classification predictions advanced from 13,799 to 13,894 in the still-running native task, while
the replacement desktop reported interrupted analysis. Synchronization rejected the new startup
ticket with `library_synchronization_state_raced`, and the source displayed `更新受阻`.

Commit `11e7a498` reconnects classification through a generation-bound desktop
attachment and transfers a ready synchronization runtime to a newer startup ticket. Old attachment
controls cannot retire the current classification task. Replaced observers stop waiting, and a
busy synchronization operation uses the existing bounded startup retry schedule before handoff.
The bridge awaits classification execution outside the ordinary query worker pool.

All three native desktop-classification cases, 18 native synchronization-admission cases and 34
Flutter synchronization cases pass locally. They cover repeated attachment, obsolete close and
observer retirement, unchanged durable session identity, current-owner shutdown, native
configuration guards, newer-ticket adoption, stale-ticket rejection and startup retry visibility.
Bridge code was regenerated; repository formatting and whitespace checks pass.

The rebuilt client completed three consecutive hot restarts in 821, 792 and 816 milliseconds.
Synchronization startup returned synchronized state after 1,028, 1,092 and 1,138 milliseconds.
The source displayed `已同步` after each restart, without an interrupted-classification notification.
The durable classification session remained unchanged while prediction count increased from 16,219
to 16,405. Worker residency remained governed by foreground admission; process identity was not
used as durable session identity. Ordinary gallery and populated smart-album navigation remained
usable. This is continuity evidence, not classification accuracy or full-library completion.

Verification reused the authorized local-primary development catalog and installed models.
Diagnostics opened the catalog read-only. No source mutation, placeholder hydration, model download,
schema change or analysis-parameter change was introduced. Source-reading and publication guards
remain with their existing owners; no new full-library hash pass was performed.

The native desktop facade, connection owner and bridge contain 116, 230 and 157 production lines;
their desktop fixtures contain 147 and 77 dedicated test lines. Native synchronization admission
contains 321 production and 773 dedicated test lines. Dart synchronization and retry owners contain
440 and 36 production lines, with 1,356 dedicated controller-test lines. None has inline tests;
generated bridge code is excluded from handwritten counts.

## Joined presentation shutdown — 2026-11-05

Normal close initially exited the process but left delayed exact-analysis and synchronization
callbacks reading an already-disposed Provider container. This also occurred before `11e7a498`.
The correction based on that commit retains background-resource retirement first, unmounts the
application through a framework frame, and only then disposes the root container. The installed
Flutter `runApp` warm-up frame also runs after the native window is hidden; bootstrap that never
mounted needs no frame. This uses framework mounting rather than manual element disposal.

The existing assembly-lifetime fixture now mounts actual consumers and checks background,
presentation and Provider retirement order, including provider access during retirement and
repeated shutdown. All six assembly cases pass. The first diagnostic fixture did not advance the
scheduled root attachment and then placed an assertion inside a guarded frame callback; complete
frame pumping and post-frame assertions corrected those fixture errors without weakening the order
checks. Repository formatting and whitespace checks pass.

The rebuilt Debug client completed another hot restart in 849 milliseconds and reattached
synchronization in 912 milliseconds. The source displayed `已同步` before normal close while the
classification worker was active. The client and worker both exited, and the captured terminal
output contained no Provider-disposal exception. Earlier gallery and smart-album checks remain
applicable. Hosted verification for the submitted branch remains pending.

The application mount, Provider assembly and entrypoint contain 18, 76 and 192 production lines;
the expanded existing assembly fixture contains 295 dedicated test lines. There are no inline tests.
