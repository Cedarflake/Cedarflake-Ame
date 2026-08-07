import "package:flutter/material.dart";

import "../features/library/presentation/r2a/r2a_gallery_prototype.dart";
import "ame_theme.dart";

class R2aPrototypeApp extends StatelessWidget {
  const R2aPrototypeApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: "Cedarflake Ame",
      theme: buildAmeTheme(),
      home: const R2aGalleryPrototype(),
    );
  }
}
