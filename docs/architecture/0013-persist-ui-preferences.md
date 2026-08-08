# ADR 0013: Persist stable UI preferences outside the catalog

- Status: Accepted for validation
- Date: 2026-08-08

## Context

Ame needs to preserve desktop window geometry, theme, viewer behavior, expanded sidebar width,
gallery sort, layout shape, and thumbnail size between launches. These values are user interface
preferences, not catalog facts or source-media metadata. Persisting them in the Rust catalog would
give the catalog ownership of presentation policy, while ad hoc files would duplicate mature
platform preference storage and lifecycle work.

Window placement also needs platform-aware restoration. Replaying stale coordinates without
checking current displays can strand the application outside the visible work area after a monitor
is disconnected or its resolution changes.

## Decision drivers

- restore stable user choices without restoring transient interaction state;
- keep UI preferences independent from catalogs and source directories;
- preserve a replaceable Ame-owned contract instead of exposing plugin APIs to presentation;
- avoid an off-screen or unusably small restored window;
- keep preference writes ordered and bounded during frequent desktop window events;
- tolerate missing, malformed, or future-version preference data.

## Considered options

### Store preferences in the Rust catalog

This would reuse existing persistence but incorrectly bind global presentation settings and window
placement to a particular media catalog and its migrations.

### Maintain a custom JSON settings file

This gives full file ownership but requires Ame to own platform storage locations, concurrent write
behavior, replacement, and corruption recovery for a small non-critical preference record.

### Flutter `shared_preferences`

Version 2.5.5 is maintained by the Flutter publisher, BSD-3-Clause licensed, supports Windows, and
provides the recommended asynchronous API without a process-local cache. The package explicitly
does not guarantee critical-data durability, which is acceptable for rebuildable UI preferences but
not for catalogs, user decisions, or operation history.

## Accepted decision

Admit `shared_preferences` 2.5.5 behind three Ame-owned typed contracts:

- `AmePreferenceStore` owns theme, viewer mouse-wheel behavior, the initial viewer scale mode, and
  expanded sidebar width;
- `AmeWindowPreferenceStore` owns normal window bounds and maximized state;
- `LibraryViewPreferenceStore` owns gallery sort key, sort direction, layout shape, and thumbnail
  size.

Each preference group is encoded as one versioned JSON string so readers can reject malformed or
unknown data and one user action does not expose a partially updated group. Adapter writes are
serialized. Defaults remain defined by Ame contracts rather than plugin behavior.

`screen_retriever` 0.2.2 is admitted alongside the existing `window_manager` adapter to obtain
current logical display work areas. Startup clamps saved normal bounds to the display with the
largest intersection or to the primary display when the saved monitor no longer exists. The normal
bounds remain separate from the maximized state. Move and resize completion events are debounced.
Closing freezes later placement events and uses the platform close path; preference I/O must not
delay or block window shutdown.

The application restores only stable presentation preferences. It does not restore selection mode,
selected assets, open menus, hover state, the current viewer transform, active searches, or a
transient scroll position as part of this decision.

## Validation gates

- adapter tests round-trip all three records and prove malformed data falls back safely;
- pure geometry tests cover a valid secondary monitor, a removed monitor, oversized bounds, and
  invalid dimensions;
- widget tests prove saved gallery presentation is applied and later toolbar changes are written;
- Flutter formatting, analysis, focused tests, existing widget tests, and a Windows Release build
  pass;
- runtime restart checks prove normal bounds, maximized state, theme, viewer choices, sidebar width,
  sort, layout, and thumbnail size are restored, and removing or changing the saved display cannot
  strand the window off-screen.

The decision becomes Accepted after the runtime restart checks pass. Compilation alone is not
sufficient.

## Consequences and risks

- global UI preferences follow the application rather than a particular image folder or catalog;
- preference loss resets presentation to safe defaults and never affects source media or durable
  catalog data;
- sequential writes preserve user action order but are not a transactional durability guarantee;
- display identifiers are not persisted, so restoration follows current work-area intersection and
  remains resilient to monitor replacement;
- future preference additions require a version-compatible decoder and explicit default.

## Replacement strategy

Both consumers depend on Ame-owned contracts. A later settings database or platform store can
replace `shared_preferences` without changing the gallery or window presentation. If display
retrieval regresses, Ame can ignore saved coordinates and retain size plus maximized state while the
adapter is repaired.
