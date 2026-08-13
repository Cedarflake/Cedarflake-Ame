# Ame notify 8.2.0 Windows backports

This directory contains the CC0-1.0 `notify` 8.2.0 crate published on crates.io with its Windows
backend patched for Ame's continuous-synchronization safety contract. The original crates.io
checksum is `4d3d07927151ff8575b7087f245456e549fea62edf0ec4e565a5ee50c8402bc3`.

The backports follow the upstream fixes that were available only on the unreleased 9.x line when
R2c-B was validated:

- `75d72fd1`: emit a watched-root removal event instead of silently unwatching;
- `21abf764`: surface initial and rearm failures through the event handler;
- `d01dc40d`: emit `Flag::Rescan` for zero-byte and `ERROR_NOTIFY_ENUM_DIR` completions.

Ame additionally surfaces other Windows completion errors through the existing `notify::Error`
callback before unwatching. No public `notify` type crosses the Ame adapter boundary.

Only `src/windows.rs` differs from the published 8.2.0 source. Replace this directory with an exact
stable upstream release after the same regression fixtures pass against that release.
