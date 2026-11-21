import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_classification_configuration.dart";

// The catalog preserves the original label on upgrade and the ID on rename.
String libraryCategoryLabel(
  AppLocalizations strings,
  LibraryClassificationCategory category,
) => switch ((category.id, category.label)) {
  ("illustration", "插画") => strings.categoryIllustration,
  ("comic", "漫画") => strings.categoryComic,
  ("meme", "表情包") => strings.categoryMeme,
  ("screenshot", "截图") => strings.categoryScreenshot,
  ("photo", "实拍") => strings.categoryPhoto,
  ("other", "其他") => strings.categoryOther,
  _ => category.label,
};
