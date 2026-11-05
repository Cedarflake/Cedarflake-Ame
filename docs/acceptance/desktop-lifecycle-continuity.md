# Desktop lifecycle continuity

This record owns post-stage desktop attachment and shutdown evidence. The [roadmap](../roadmap.md)
owns delivery status; [ADR 0036](../architecture/0036-durable-classification-dispatch.md) and
[ADR 0024](../architecture/0024-windows-change-driven-library-continuity.md) own runtime authority.

## Development hot restart — 2026-11-05

The Windows Debug client at `05038a3b` reproduced both failures after Flutter hot restart.
Classification predictions advanced from 13,799 to 13,894 in the still-running native task, while
the replacement desktop reported interrupted analysis. Synchronization rejected the new startup
ticket with `library_synchronization_state_raced`, and the source displayed `更新受阻`.

The change based on that revision reconnects classification through a generation-bound desktop
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

Normal close after these restarts exited the process, but delayed presentation callbacks attempted
to read the already-disposed Provider container. This also occurred before the attachment change;
the remaining repair is presentation unmount ordering before container disposal. Hosted checks for
the submitted branch remain required.

The native desktop facade, connection owner and bridge contain 116, 230 and 157 production lines;
their desktop fixtures contain 147 and 77 dedicated test lines. Native synchronization admission
contains 321 production and 773 dedicated test lines. Dart synchronization and retry owners contain
440 and 36 production lines, with 1,356 dedicated controller-test lines. None has inline tests;
generated bridge code is excluded from handwritten counts.
