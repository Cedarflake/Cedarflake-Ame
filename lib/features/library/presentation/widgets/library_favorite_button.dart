import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";

class LibraryFavoriteButton extends ConsumerWidget {
  const LibraryFavoriteButton({
    required this.target,
    this.enabled = true,
    super.key,
  });
  final LibraryOrganizationTarget target;
  final bool enabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final status = ref.watch(libraryOrganizationInspectionProvider(target));
    final saving = ref.watch(
      libraryOrganizationActionsProvider.select((value) => value.saving),
    );
    final favorite = status.value?.decisions.singleOrNull?.favorite;
    final label = _label(
      context.l10n,
      favorite: favorite,
      saving: saving,
      error: status.error,
    );
    VoidCallback? action;
    if (enabled && !saving && !status.hasError) {
      if (favorite != null) {
        action = () => unawaited(
          ref
              .read(libraryOrganizationActionsProvider.notifier)
              .favorite(target, !favorite),
        );
      }
    }
    return AmeTooltip(
      message: label,
      child: IconButton(
        key: const Key("viewer-favorite"),
        iconSize: 20,
        isSelected: favorite ?? false,
        onPressed: action,
        icon: Icon(
          Symbols.favorite_rounded,
          opticalSize: 20,
          weight: 400,
          fill: 0,
          semanticLabel: label,
        ),
        selectedIcon: Icon(
          Symbols.favorite_rounded,
          opticalSize: 20,
          weight: 400,
          fill: 1,
          semanticLabel: label,
        ),
      ),
    );
  }

  String _label(
    AppLocalizations strings, {
    required bool? favorite,
    required bool saving,
    required Object? error,
  }) {
    if (error is LibraryOrganizationFailure &&
        error.code == "organization_evidence_pending") {
      return strings.checkingPhotoContent;
    }
    if (error != null) {
      return strings.favoritesUnavailable;
    }
    if (saving) {
      return strings.savingChanges;
    }
    if (favorite == null) {
      return strings.loadingFavoriteStatus;
    }
    return favorite ? strings.removeFavorite : strings.addFavorite;
  }
}
