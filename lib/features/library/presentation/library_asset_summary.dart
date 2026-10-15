String libraryFileType(String path) {
  final fileName = path.replaceAll("\\", "/").split("/").last;
  final separator = fileName.lastIndexOf(".");
  if (separator < 0 || separator == fileName.length - 1) {
    return "未知";
  }
  return fileName.substring(separator + 1).toUpperCase();
}

String libraryFileSize(BigInt value) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  var amount = value.toDouble();
  var unitIndex = 0;
  while (amount >= 1024 && unitIndex < units.length - 1) {
    amount /= 1024;
    unitIndex++;
  }
  if (unitIndex == 0) {
    return "$value ${units[unitIndex]}";
  }
  return "${amount.toStringAsFixed(1)} ${units[unitIndex]}";
}
