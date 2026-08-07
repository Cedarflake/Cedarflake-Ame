import "package:flutter/material.dart";

import "../features/library/presentation/unified_library_screen.dart";
import "ame_theme.dart";

class AmeApp extends StatelessWidget {
  const AmeApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: "Cedarflake Ame",
      theme: buildAmeTheme(),
      home: const UnifiedLibraryScreen(),
    );
  }
}
