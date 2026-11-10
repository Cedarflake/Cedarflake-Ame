# Dialog templates

Status: binding presentation requirements under the [gallery UI contract](../gallery-ui.md).

## Shared structure

Use Material `AlertDialog` through `AmeDialog` for task confirmations, short forms and details.
The shared owner keeps the content width, scrolling, title and action placement consistent;
Material owns shape, colors, focus, keyboard dismissal and button feedback. Keep desktop window
constraints and enlarged text usable. Do not make unrelated controls visually match by overriding
their corner radii. Use labels, alignment and grouping to establish the relationship instead.

| Template | Content order | Actions |
| --- | --- | --- |
| Confirmation | Affected object with its existing icon/name, consequence, optional choice | Cancel, explicit action |
| Short form | Required fields in reading order; auxiliary choices stay beside their object | Cancel, save/apply |
| Result or location | Human-readable result, labeled time/counts, compact location with a connected open action | Close; recovery only when supported |
| Error details | What happened, affected object when useful, separate labeled diagnostic code | Close; connected retry when valid |

`AmeDialogField`, `AmeDialogItem`, `AmeDialogLocation` and `AmeDialogDiagnostic` compose these
templates. Keep operation policy and lifetime in existing application owners. Do not add a universal
dialog controller or copy framework input/navigation behavior. Existing bottom trays and the
framework date picker retain their own interaction owners; they are not converted into alerts.

## Content and controls

- Show counts when they explain batch scope or a consequence. Single-file rename has no redundant
  count; it remains a single-file action, not a batch-rename editor.
- Give time and location values labels. Show a folder or filename in the normal layout, retain the
  complete location in a tooltip or explicit details, and provide an implemented open-location action.
  Never turn a pathname into decorative button text or an unexplained paragraph.
- Destination selection is a labeled object row with a choose/change action; organization rules
  occupy their own aligned field. Preserve common field styling and menu motion.
- The permanent-deletion choice has no row-wide hover fill. Retain checkbox state, click feedback,
  keyboard focus and accessible labeling; every new confirmation starts unchecked.
- New albums place an icon-selection button beside the name input. Expand icon choices on demand.
  Keep the button square and outside the input border, aligned with its content rather than error
  text. Album deletion shows the actual album icon and name above the consequence.
- Diagnostic codes retain their source, such as `OS: 32`, and complete hexadecimal values. Separate
  them from the explanation without reducing a typed code to an unexplained number.
- Restoration presents the backup identity, creation date and relevant nonzero contents as labeled
  rows. Keep conflicts and the effects on current decisions explicit without repeating paragraphs.
- Separate the date and weekday deliberately in the date picker's landscape header. Preserve
  framework calendar navigation, text entry, range validation and accessible full-date labels.

Feature-specific wording and authorization stay with the owning workflow. This template grants no
file mutation, restoration, cleanup or global theme change. The [feedback contract](feedback.md)
continues to own notifications and bottom feedback.
