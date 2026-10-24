String libraryOrganizationErrorMessage(String code) => switch (code) {
  "organization_album_name_invalid" => "请输入 1 至 128 个字符的相册名称。",
  "organization_album_name_exists" => "已有同名相册，请换一个名字。",
  "organization_album_unavailable" => "这个相册已不存在，请重新选择。",
  "organization_selection_changed" => "图片或相册信息已更新，请检查后重试。",
  "organization_selection_expired" => "图片或整理信息已更新，请取消后重新选择图片。",
  "organization_selection_empty" => "没有可操作的图片，请重新选择。",
  "organization_selection_busy" => "上一项操作尚未结束，请稍后重试。",
  "organization_conflict_unresolved" => "相册归属有冲突，请选择一个相册。",
  "organization_evidence_pending" => "图片信息正在更新，请稍后重试。",
  _ => "尚未确认保存结果，请重试。",
};
