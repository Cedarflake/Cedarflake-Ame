import "package:cedarflake_ame/features/library/presentation/widgets/library_loading_indicator.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("loading indicator remains square in a narrow tile", (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Center(
          child: SizedBox(
            width: 36,
            height: 120,
            child: LibraryLoadingIndicator(),
          ),
        ),
      ),
    );

    final size = tester.getSize(find.byType(CircularProgressIndicator));
    expect(size.width, size.height);
    expect(size.width, lessThanOrEqualTo(24));
  });
}
