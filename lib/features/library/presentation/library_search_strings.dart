import "../domain/library_semantic_search.dart";

abstract final class LibrarySearchStrings {
  static const content = "画面内容";
  static const fileName = "文件名和路径";
  static const currentScope = "当前范围";
  static const entireLibrary = "整个图库";
  static const related = "查找相关图片";
  static const update = "更新搜索结果";
  static const retry = "重试搜索";
  static const contentHint = "描述要找的图片";
  static const fileHint = "搜索文件名或路径";
  static const noResultsHint = "可以换个描述、调整筛选，或搜索整个图库。";
  static const noFilterResultsHint = "可以调整或清除筛选。";

  static String failure(Object error) => switch (error) {
    LibrarySemanticFailure(code: "semantic_model_unavailable") =>
      "请先在设置中安装图片分析模型。",
    LibrarySemanticFailure(code: "semantic_query_invalid") =>
      "请输入不超过 512 个字符的描述。",
    LibrarySemanticFailure(code: "semantic_index_blocked") =>
      "部分图片尚未完成分析，已保留可用的搜索结果。请重试分析。",
    LibrarySemanticFailure(code: "semantic_catalog_changed") => "图库刚有更新，请重试搜索。",
    LibrarySemanticFailure(code: "semantic_source_changed") =>
      "这张图片已变化或无法读取，请重新选择。",
    LibrarySemanticFailure(code: "semantic_session_unavailable") =>
      "图片分析当前不可用，请重试搜索。",
    LibrarySemanticFailure(code: "semantic_storage_invalid") =>
      "搜索数据已损坏，请重试以重新建立。",
    LibrarySemanticFailure(code: "semantic_storage_failed") =>
      "暂时无法读写搜索数据，请重试。",
    _ => "暂时无法搜索，已保留原来的图片列表。请重试。",
  };

  static String coverage(LibrarySemanticResults results) =>
      "按相关度排列 · 已分析 ${results.indexedItems}/${results.eligibleItems} 张"
      "${results.storageFull ? ' · 分析存储已满' : ''}";

  static String? notice(LibrarySemanticResults results) {
    if (results.storageFull || results.indexedItems < results.eligibleItems) {
      return "部分结果";
    }
    return null;
  }
}
