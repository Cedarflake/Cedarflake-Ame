import "package:cedarflake_ame/features/library/presentation/widgets/timeline_linear_projection.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("maps actual offsets linearly in both directions", () {
    final projection = TimelineLinearProjection(maximumOffset: 10000);

    for (final offset in [0.0, 1.0, 2500.0, 5000.0, 9999.0, 10000.0]) {
      final value = projection.offsetToValue(offset);
      expect(projection.valueToOffset(value), closeTo(offset, 0.000001));
    }
    expect(projection.offsetToValue(1), closeTo(0.0001, 0.000001));
    expect(projection.offsetToValue(9999), closeTo(0.9999, 0.000001));
  });

  test("clamps only outside the actual scroll range", () {
    final projection = TimelineLinearProjection(maximumOffset: 100);

    expect(projection.offsetToValue(-10), 0);
    expect(projection.offsetToValue(120), 1);
    expect(projection.valueToOffset(-0.2), 0);
    expect(projection.valueToOffset(1.2), 100);
  });

  test("does not redistribute sparse ranges near either endpoint", () {
    final projection = TimelineLinearProjection(maximumOffset: 20000);

    expect(projection.offsetToValue(1), closeTo(0.00005, 0.000001));
    expect(projection.offsetToValue(10000), closeTo(0.5, 0.000001));
    expect(projection.offsetToValue(19999), closeTo(0.99995, 0.000001));
    expect(projection.valueToOffset(0.00005), closeTo(1, 0.000001));
    expect(projection.valueToOffset(0.99995), closeTo(19999, 0.000001));
  });
}
