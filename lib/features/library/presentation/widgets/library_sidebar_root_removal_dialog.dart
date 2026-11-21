import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../application/library_controller.dart";
import "../../application/library_sidebar_root_removal.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "library_sidebar_removal_dialog.dart";

Future<Set<String>> showLibrarySidebarRootRemovalDialog({
  required BuildContext context,
  required List<LibraryRoot> roots,
  required LibraryController controller,
  required LibraryState Function() readState,
}) => showLibrarySidebarRemovalDialog(
  context: context,
  removal: createSidebarRootRemoval(
    roots: roots,
    controller: controller,
    readState: readState,
  ),
  title: context.l10n.confirmRemoveFolders(roots.length),
  detail: context.l10n.removeFoldersImpact,
  actionLabel: context.l10n.removeFromAme,
);
