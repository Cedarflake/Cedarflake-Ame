import "../domain/library_metadata_filter.dart";

abstract final class LibraryFilterStrings {
  static const more = "更多筛选";
  static const title = "筛选图片";
  static const clear = "清除筛选";
  static const cancel = "取消";
  static const apply = "应用";
  static const date = "日期";
  static const dateField = "日期类型";
  static const dateStart = "开始日期";
  static const dateEnd = "结束日期";
  static const dateHint = "年-月-日";
  static const file = "文件属性";
  static const fileFormat = "文件格式";
  static const otherFormat = "其他格式";
  static const any = "不限";
  static const custom = "自定义";
  static const fileSize = "文件大小";
  static const width = "宽度";
  static const height = "高度";
  static const minimum = "最小值";
  static const maximum = "最大值";
  static const extension = "扩展名";
  static const extensionHint = "例如 jpg、png";
  static const minimumBytes = "最小文件大小";
  static const maximumBytes = "最大文件大小";
  static const unit = "单位";
  static const dimensions = "图片尺寸";
  static const minimumWidth = "最小宽度";
  static const maximumWidth = "最大宽度";
  static const minimumHeight = "最小高度";
  static const maximumHeight = "最大高度";
  static const camera = "相机信息";
  static const cameraHint = "品牌或型号";
  static const invalidExtension = "请输入不超过 16 位的字母或数字";
  static const invalidNumber = "请输入有效的非负数";
  static const invalidInteger = "请输入有效的非负整数";
  static const fractionalBytes = "换算后的大小需为整数字节";
  static const numberTooLarge = "数值过大";
  static const reversedRange = "最大值不能小于最小值";
  static const invalidDate = "请输入有效日期，如 2026-01-31";
  static const reversedDates = "结束日期不能早于开始日期";
  static const invalidCamera = "请输入不超过 128 个字符的品牌或型号";

  static String dateFieldName(LibraryDateField field) => switch (field) {
    LibraryDateField.capture => "拍摄日期",
    LibraryDateField.created => "创建日期",
    LibraryDateField.modified => "修改日期",
  };
}
