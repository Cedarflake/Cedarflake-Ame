import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("explicit selection toggles stable location identities", () {
    var selection = GallerySelection.empty("query-1");

    selection = selection.toggle("location-a");
    selection = selection.toggle("location-b");

    expect(selection.contains("location-a"), isTrue);
    expect(selection.contains("location-b"), isTrue);
    expect(selection.selectedCount(100), 2);

    selection = selection.toggle("location-a");
    expect(selection.contains("location-a"), isFalse);
    expect(selection.selectedCount(100), 1);
  });

  test("complete-query selection stays bounded through exclusions", () {
    var selection = GallerySelection.empty("query-1").selectAll();

    expect(selection.selectedCount(79013), 79013);
    expect(selection.includedLocationIds, isEmpty);

    selection = selection.toggle("location-a");
    expect(selection.contains("location-a"), isFalse);
    expect(selection.selectedCount(79013), 79012);
    expect(selection.excludedLocationIds, {"location-a"});
  });

  test("clearing selection preserves the owning query identity", () {
    final selection = GallerySelection.empty(
      "query-2",
    ).selectAll().toggle("location-a").clear();

    expect(selection.queryId, "query-2");
    expect(selection.isEmpty, isTrue);
  });
}
