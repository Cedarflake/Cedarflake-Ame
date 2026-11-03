import "package:flutter/material.dart";
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
    if (settings == null || settings.categories.isEmpty) {
      return const SizedBox.shrink();
    }
    final result = ref.watch(libraryClassificationItemProvider(target));
    if (result.hasError) {
      return AmeTooltip(
        message: "重新读取分类",
        child: IconButton(
          key: const Key("library-reload-classification"),
          onPressed: () =>
              ref.invalidate(libraryClassificationItemProvider(target)),
          icon: const Icon(Symbols.refresh_rounded, semanticLabel: "重新读取分类"),
        ),
      );
    }
    final item = result.asData?.value;
    final category = settings.categories
        .where((category) => category.id == item?.categoryId)
        .firstOrNull;
    if (category == null) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      child: AmeTooltip(
        message: "分类：${category.label}",
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Symbols.label_rounded, size: 20),
            const SizedBox(width: 8),
            Text(category.label),
          ],
        ),
      ),
    );
  }
}
