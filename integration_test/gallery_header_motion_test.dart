import "package:integration_test/integration_test.dart";

import "../test/features/library/presentation/library_gallery_header_reflow_test.dart"
    as header_reflow;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  header_reflow.main();
}
