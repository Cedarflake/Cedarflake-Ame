import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_classification_review.dart";
import "../../application/library_classification_settings.dart";
import "../../domain/library_organization.dart";

class LibraryViewerClassification extends ConsumerWidget {
  const LibraryViewerClassification({required this.target, super.key});
  final LibraryOrganizationTarget target;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(libraryClassificationSettingsProvider).value;
    if (settings == null || settings.visibleCategories.isEmpty) {
      return const SizedBox.shrink();
    }
    final result = ref.watch(libraryClassificationItemProvider(target));
    final item = result.value;
    final category = settings.visibleCategories
        .where((category) => category.id == item?.categoryId)
        .firstOrNull;
    if (category == null) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsetsDirectional.only(end: 8),
      child: AmeTooltip(
        message: "分类：${category.label}",
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox.square(
              dimension: 40,
              child: Icon(Symbols.label_rounded, size: 20),
            ),
            Text(category.label),
          ],
        ),
      ),
    );
  }
}
