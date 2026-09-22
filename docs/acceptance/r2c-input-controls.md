# R2c picker and task controls

Status: **C06 focused correction verified; native input lifetime incomplete**.

This record covers the [actual picker and task-control method](../plans/r2c-closeout.md#actual-picker-and-task-control-method)
on 2026-09-22. Its source starts at `ef4eee27ca47c1c7e6d3b97e986f9d73da6ded7a`, with the C06
presentation correction and regression below. It does not accept UX-02A/B/C, UX-08C or all of R2c.

## C06: cancellation while pause is pending

The task surface previously displayed controls only for `scanning`. After Pause changed the
projection to `pausing`, Cancel disappeared although `library_scan_control.dart` still admitted
Cancel as the higher-priority intent. This directly blocked the selected pre-registration journey;
it is an S1 control-entry defect, not a new native cancellation policy.

The connected widget regression first fails on the original presentation because
`library-cancel-button` is absent. The correction keeps the existing Cancel button during `pausing`
and displays Pause only during `scanning`. The regression drives both buttons through the real
controller, observes cancelling feedback, then delivers late Started and verifies only Cancel is
replayed. Terminal cancellation preserves the previous roots. Native registration, stream-drain,
publication and cancellation ownership are unchanged.

The affected production file has 204 lines and no inline tests; its dedicated feedback suite has
122 lines. This change adjusts the existing task control rather than introducing a new control,
state owner, migration, dependency or polling loop.

Serial verification passes 15 scan-control, 2 feedback, 8 retained-interaction and 9 diagnostic
admission/input cases (34 total). After the diagnostic keyboard guard was tightened, its three
affected cases pass again. Repository formatting checks 228 files with no changes; complete Dart
analysis passes with informational and warning diagnostics fatal. The final isolated Debug build
passes in 22.9 seconds. Full local lint/Daily remains blocked by C02; earlier hosted results are
not final-source evidence for this presentation change.

Independent preparation review identifies three evidence weaknesses: Enter needed a focused task
button, cancellation needed current terminal-state pixels, and the issue needed its exact damaged
path and error code. The corrections retain those independent assertions. A scoped recheck finds
that an invalid retry could be hidden by a later valid one; rejection is now permanently recorded
and forwarded to the diagnostic failure owner. Negative tests reject a later correct Enter after
an invalid attempt. The final scoped recheck closes that finding. Active review totals about nine
minutes, including the C06 control admission; no native pass is inferred from review.

## Intended native evidence

The prepared generated source contains three valid images, including PNG bytes under a `.jpg`
name, and one damaged PNG in a Chinese-named directory. Exact paths, hashes, file IDs, sizes and
timestamps are frozen before launch. The second picker target is the existing 10000-image corpus
with twelve dimensions and historical dates, totaling 10921494393 bytes.

The diagnostic uses the production picker, bridge, catalog and synchronization lifecycle with
marked Debug-only storage and in-memory preferences. After the first real commit it would inject
one catalog display-read failure, then require actual focused Enter to retry only the display.
A second-command decorator would hold dispatch for at most 120 seconds so actual Pause then Cancel
could reach the pre-registration boundary; it forwards real events without substituting native
results. These disclosed controls prove ownership rather than production latency. Neither phase
was reached in the lifetime below.

## Retained failed native lifetime

Run `5c0b0f7ea3c14bfba8fd6a50d066d2e3`, PID 13912 under owned parent 37844, reveals the empty
application and opens the actual Windows folder picker. Computer Use subsequently returns
`coordinate input geometry is unavailable`, `call get_window_state before using this window`,
and `unknown screenshotId screenshot-1`. Re-observation and fresh selection do not resolve the
binding failure. Reported focus remains the search box; successful delivery to the intended path
field is not established. No directory is accepted and no source scan is dispatched.

The attempt stops without an unchanged replay. After verifying executable, PID and parent identity,
the owned client is deliberately retired. Exit -1 after 250839 ms is a failed execution, not normal
shutdown evidence. Parent receipt confirms process exit, Job closure and no cleanup failure. The
monitor retires with 938 samples, peak working set 440160256 bytes, sampled private bytes 313692160,
kernel peak commitment 359796736 and minimum system availability 5378469888 bytes. No resource
bound is exceeded. There is no successful input result or ready-to-close receipt.

Full pre/post source checks pass in 70.267/50.485 seconds for all four small files and all 10000
frozen files: exact membership, bytes, IDs, sizes and baseline timestamps remain unchanged. No real
library is selected. The failure identifies a missing automation path, not an Ame scan/preview
defect, and does not establish shared causality with C02's separate native probe or file replacement.

Debug SHA-256 identities are:

- EXE: `2A24C91530C05D88B29ACC4E6FCF73199C237DEB04159B232A9FD7E43BD98DCE`.
- Dart kernel: `2BDE941F13E38E2A0336AD47EF71D6ABB17A5E0764ADF8552EB36E6931B2D98E`.
- Rust DLL: `77B193EA7CA1B5CEEC033038E3806490890E5DBB3E818AAD72B062D0A46F50C2`.

Ignored `.build/r2c-input-controls/` retains the immutable admission, diagnostic source, failing
pre-fix regression, passing focused outputs and build logs. Its GUID-owned fixture retains the
input failure, process/memory receipts, raw logs, frozen source manifest and integrity results.
Conservatively charge the full 120-minute input reservation and 45-minute C06 supplement; these
ceilings are not measured active-time totals. Earlier C01/C02 and failed native allowances remain
consumed. Another client lifetime requires a recorded changed input method and a new bounded
checkpoint. It cannot replace this failure, loosen the input oracle or launch against the user catalog.

The [C05 bulk preview pass](r2c-browsing-diagnosis.md#accumulated-candidate-and-transient-preview-feedback) remains separate:
C06 changes task-button visibility, not preview materialization or retirement. Actual picker/task
input, complete focus assertions, Release paths and the final accumulated gates remain open.
