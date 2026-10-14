import "package:flutter/material.dart";

import "../library_strings.dart";

class LibraryQueryFailureBanner extends StatelessWidget {
  const LibraryQueryFailureBanner({required this.onRetry, super.key});

  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) => MaterialBanner(
    key: const Key("library-query-failure"),
    content: const Text("图片列表加载失败"),
    actions: [
      TextButton(
        key: const Key("library-query-retry"),
        onPressed: onRetry,
        child: const Text(LibraryStrings.retryLoading),
      ),
    ],
  );
}
