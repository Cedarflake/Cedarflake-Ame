import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_riverpod/misc.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../features/library/application/library_album_pages.dart";
import "../../features/library/presentation/library_workbench_destinations.dart";
import "../../features/library/presentation/widgets/library_album_picker.dart";
import "../../features/settings/presentation/default_album_picker.dart";
import "../../features/settings/presentation/pages/ame_settings_page.dart";
import "../../features/settings/presentation/widgets/organization_record_feedback.dart";

List<Override> workbenchPresentationBindings() => [
  librarySettingsDestinationProvider.overrideWithValue(
    (context, {required hasLibraryRoots, required libraryRootIds}) =>
        AmeSettingsPage(
          hasLibraryRoots: hasLibraryRoots,
          libraryRootIds: libraryRootIds,
        ),
  ),
  libraryRecordFeedbackProvider.overrideWithValue(
    buildOrganizationRecordFeedback,
  ),
  defaultAlbumPickerProvider.overrideWithValue(_pickDefaultAlbum),
];

Future<DefaultAlbumSelection?> _pickDefaultAlbum(
  BuildContext context,
  String? initialAlbumId,
) async {
  final container = ProviderScope.containerOf(context, listen: false);
  final current = initialAlbumId == null
      ? null
      : container.read(libraryAlbumProvider(initialAlbumId)).value;
  final result = await showLibraryAlbumPicker(
    context: context,
    actionLabel: "选择",
    actionIcon: Symbols.check_rounded,
    clearLabel: "未设置",
    clearIcon: Symbols.close_rounded,
    initialAlbum: current,
  );
  return result == null ? null : (albumId: result.album?.id);
}

final workbenchPresentationRequirements = <ProviderListenable<Object?>>[
  librarySettingsDestinationProvider,
  libraryRecordFeedbackProvider,
  defaultAlbumPickerProvider,
];
