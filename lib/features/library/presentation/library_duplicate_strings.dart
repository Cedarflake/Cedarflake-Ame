import "../domain/library_exact_duplicates.dart";

// Duplicates workspace: exact analysis, result groups and physical-location inspection.
abstract final class LibraryDuplicateStrings {
  static const title = "重复图片";
  static const workspaceTitle = "重复图片查找";
  static const revealLocation = "在文件资源管理器中打开";
  static const revealFailed = "无法在文件资源管理器中打开";
  static const start = "查找重复图片";
  static const refreshAnalysis = "更新结果";
  static const retry = "重试";
  static const stop = "停止查找";
  static const stopping = "正在停止";
  static const running = "正在查找重复图片";
  static const completed = "查找完成";
  static const cancelled = "已停止查找";
  static const failed = "查找未完成";
  static const automaticWaiting = "稍后继续查找";
  static const paused = "已暂停查重";
  static const pause = "暂停查重";
  static const resume = "继续查重";
  static const pauseSaveFailed = "无法保存查重设置";
  static const close = "关闭";
  static const emptyBeforeAnalysis = "查找后，可以查看相同图片保存在哪些文件夹。";
  static const emptyAfterAnalysis = "已检查的图片中没有发现重复项";
  static const waitingForResults = "找到的重复图片会显示在这里。";
  static const emptyLocations = "这组图片的位置已发生变化";
  static const loadFailed = "暂时无法读取结果";
  static const viewLocations = "查看重复位置";
  static const back = "返回重复图片";
  static const previous = "上一页";
  static const next = "下一页";
  static const refresh = "刷新结果";

  // Duplicates workspace: group review, reversible choices and the ignored list.
  static const pendingGroups = "重复组";
  static const ignoredGroups = "已忽略";
  static const noIgnoredGroups = "没有已忽略的重复组";
  static const prefer = "设为首选";
  static const clearPreferred = "取消首选";
  static const ignore = "忽略此组";
  static const restoreGroup = "恢复此组";
  static const openImage = "查看图片";
  static const saveFailed = "未能保存选择，请重试";
  static String groups(int count) => "$count 个重复图像组";

  // Gallery header filter and the local location list opened from a photo badge.
  static const filter = "筛选";
  static const showDescendants = "显示子目录";
  static const hideDescendants = "隐藏子目录";
  static const allImages = "全部图片";
  static const mergeDuplicates = "合并重复图片";
  static const currentImage = "当前图片";

  static String checked(int count) => "本次已检查 $count 个文件";
  static String skipped(int count) => "$count 个文件未能完成检查，可稍后重试。";
  static String locations(int count) => "$count 个位置";
  static String physicalFiles(int count) => "$count 份文件";

  // Each location row is refreshed using metadata only; opening the location list never downloads files.
  static String? availability(LibraryDuplicateAvailability value) =>
      switch (value) {
        LibraryDuplicateAvailability.local => null,
        LibraryDuplicateAvailability.cloudOnly => "仅在云端",
        LibraryDuplicateAvailability.offline => "文件夹离线",
        LibraryDuplicateAvailability.unavailable => "暂时无法访问",
      };
}
