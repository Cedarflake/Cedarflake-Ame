import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

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
    LibraryOrganizationDisposition disposition, {
    bool hasConflict = false,
    bool multiple = false,
  }) : defer = disposition == LibraryOrganizationDisposition.deferred
           ? LibraryProgressControl(
               LibraryProgressAction.restore,
               "移除",
               Symbols.remove_circle_rounded,
               !hasConflict,
             )
           : const LibraryProgressControl(
               LibraryProgressAction.defer,
               "稍后整理",
               Symbols.schedule_rounded,
               true,
             ),
       ignore = disposition == LibraryOrganizationDisposition.ignored
           ? LibraryProgressControl(
               LibraryProgressAction.restore,
               "移除",
               Symbols.remove_circle_rounded,
               !hasConflict,
             )
           : LibraryProgressControl(
               LibraryProgressAction.ignore,
               multiple ? "忽略所选图片" : "忽略此图",
               Symbols.hide_image_rounded,
               !hasConflict,
             );

  factory LibraryProgressControls.forQueue(
    LibraryOrganizationQueue queue, {
    bool hasConflict = false,
  }) => LibraryProgressControls.forDisposition(
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
