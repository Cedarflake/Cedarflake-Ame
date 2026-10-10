import "package:flutter/material.dart";

import "../../domain/library_query_activity.dart";

class LibraryQueryFailureBanner extends StatelessWidget {
  const LibraryQueryFailureBanner({
    required this.failure,
    required this.onRetry,
    super.key,
  });

  final LibraryQueryFailed failure;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) => MaterialBanner(
    key: const Key("library-query-failure"),
    content: Text("图片加载失败，仍显示之前的结果。${failure.message}"),
    actions: [
      TextButton(
        key: const Key("library-query-retry"),
        onPressed: onRetry,
        child: const Text("重试加载"),
      ),
    ],
  );
}
