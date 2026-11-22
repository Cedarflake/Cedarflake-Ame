import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "../domain/library_review.dart";

class LibraryProgressControl {
  const LibraryProgressControl(
    this.action,
    this.label,
    this.icon,
    this.enabled,
  );

  final LibraryProgressAction action;
  final String label;
  final IconData icon;
  final bool enabled;
}

class LibraryProgressControls {
  LibraryProgressControls.forDisposition(
    AppLocalizations strings,
    LibraryOrganizationDisposition disposition, {
    bool hasConflict = false,
    bool multiple = false,
  }) : defer = disposition == LibraryOrganizationDisposition.deferred
           ? LibraryProgressControl(
               LibraryProgressAction.restore,
               strings.removeFromLater,
               Symbols.remove_circle_rounded,
               !hasConflict,
             )
           : LibraryProgressControl(
               LibraryProgressAction.defer,
               strings.organizeLater,
               Symbols.schedule_rounded,
               true,
             ),
       ignore = disposition == LibraryOrganizationDisposition.ignored
           ? LibraryProgressControl(
               LibraryProgressAction.restore,
               strings.unignore,
               Symbols.remove_circle_rounded,
               !hasConflict,
             )
           : LibraryProgressControl(
               LibraryProgressAction.ignore,
               multiple ? strings.ignoreSelected : strings.ignorePhoto,
               Symbols.hide_image_rounded,
               !hasConflict,
             );

  factory LibraryProgressControls.forQueue(
    AppLocalizations strings,
    LibraryOrganizationQueue queue, {
    bool hasConflict = false,
  }) => LibraryProgressControls.forDisposition(
    strings,
    switch (queue) {
      LibraryOrganizationQueue.pending =>
        LibraryOrganizationDisposition.pending,
      LibraryOrganizationQueue.deferred =>
        LibraryOrganizationDisposition.deferred,
      LibraryOrganizationQueue.ignored =>
        LibraryOrganizationDisposition.ignored,
    },
    hasConflict: hasConflict,
    multiple: true,
  );

  final LibraryProgressControl defer;
  final LibraryProgressControl ignore;
}
