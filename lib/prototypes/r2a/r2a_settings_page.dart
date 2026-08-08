import "package:flutter/material.dart";

import "r2a_strings.dart";

class R2aSettingsPage extends StatefulWidget {
  const R2aSettingsPage({required this.onBack, super.key});

  final VoidCallback onBack;

  @override
  State<R2aSettingsPage> createState() => _R2aSettingsPageState();
}

class _R2aSettingsPageState extends State<R2aSettingsPage> {
  String _theme = R2aStrings.followSystem;
  String _wheelBehavior = R2aStrings.zoom;
  String _openBehavior = R2aStrings.fitWindow;
  String _thumbnailLimit = "4 GB";

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return ColoredBox(
      color: colorScheme.surface,
      child: CustomScrollView(
        key: const Key("r2a-settings-page"),
        slivers: [
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(32, 24, 32, 64),
            sliver: SliverToBoxAdapter(
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 980),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Row(
                        children: [
                          IconButton(
                            key: const Key("r2a-settings-back"),
                            tooltip: R2aStrings.backToLibrary,
                            onPressed: widget.onBack,
                            icon: const Icon(Icons.arrow_back),
                          ),
                          const SizedBox(width: 8),
                          const Icon(Icons.settings_outlined, size: 30),
                          const SizedBox(width: 12),
                          Text(
                            R2aStrings.settings,
                            style: Theme.of(context).textTheme.headlineMedium,
                          ),
                        ],
                      ),
                      const SizedBox(height: 36),
                      _SettingsSection(
                        title: "个性化",
                        children: [
                          _SettingsChoiceRow(
                            icon: Icons.palette_outlined,
                            title: R2aStrings.applicationTheme,
                            description: R2aStrings.applicationThemeDescription,
                            value: _theme,
                            values: const [
                              R2aStrings.followSystem,
                              R2aStrings.light,
                              R2aStrings.dark,
                            ],
                            onChanged: (value) =>
                                setState(() => _theme = value),
                          ),
                        ],
                      ),
                      const SizedBox(height: 28),
                      _SettingsSection(
                        title: R2aStrings.browsing,
                        children: [
                          _SettingsChoiceRow(
                            icon: Icons.mouse_outlined,
                            title: R2aStrings.mouseWheel,
                            description: R2aStrings.mouseWheelDescription,
                            value: _wheelBehavior,
                            values: const [
                              R2aStrings.zoom,
                              R2aStrings.previousOrNext,
                            ],
                            onChanged: (value) =>
                                setState(() => _wheelBehavior = value),
                          ),
                          _SettingsChoiceRow(
                            icon: Icons.image_outlined,
                            title: R2aStrings.openImage,
                            description: R2aStrings.openImageDescription,
                            value: _openBehavior,
                            values: const [
                              R2aStrings.fitWindow,
                              R2aStrings.actualSize,
                            ],
                            onChanged: (value) =>
                                setState(() => _openBehavior = value),
                          ),
                        ],
                      ),
                      const SizedBox(height: 28),
                      _SettingsSection(
                        title: R2aStrings.storage,
                        children: [
                          const _SettingsActionRow(
                            icon: Icons.storage_outlined,
                            title: R2aStrings.libraryDataLocation,
                            description: R2aStrings.libraryDataDescription,
                            detail: "C:\\Users\\Example\\AppData\\Local\\Ame",
                            actionLabel: R2aStrings.change,
                          ),
                          const _SettingsActionRow(
                            icon: Icons.photo_library_outlined,
                            title: R2aStrings.thumbnailLocation,
                            description: R2aStrings.thumbnailDescription,
                            detail:
                                "C:\\Users\\Example\\AppData\\Local\\Ame\\预览",
                            actionLabel: R2aStrings.change,
                          ),
                          _SettingsChoiceRow(
                            icon: Icons.data_usage_outlined,
                            title: R2aStrings.thumbnailLimit,
                            description: R2aStrings.thumbnailLimitDescription,
                            value: _thumbnailLimit,
                            values: const ["1 GB", "4 GB", "10 GB", "20 GB"],
                            onChanged: (value) =>
                                setState(() => _thumbnailLimit = value),
                          ),
                          const _SettingsActionRow(
                            icon: Icons.cleaning_services_outlined,
                            title: R2aStrings.clearThumbnails,
                            description: R2aStrings.clearThumbnailsDescription,
                            detail: "当前占用 1.2 GB",
                            actionLabel: R2aStrings.clear,
                          ),
                        ],
                      ),
                      const SizedBox(height: 28),
                      const _SettingsSection(
                        title: R2aStrings.about,
                        children: [
                          _SettingsInfoRow(
                            icon: Icons.info_outline,
                            title: R2aStrings.version,
                            detail: "0.1.0",
                          ),
                          _SettingsInfoRow(
                            icon: Icons.code_outlined,
                            title: R2aStrings.openSourceNotices,
                            detail: "查看 Ame 使用的开源软件与许可",
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _SettingsSection extends StatelessWidget {
  const _SettingsSection({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: 4, bottom: 10),
          child: Text(title, style: Theme.of(context).textTheme.titleMedium),
        ),
        Card(
          margin: EdgeInsets.zero,
          elevation: 0,
          color: Theme.of(context).colorScheme.surfaceContainerLow,
          clipBehavior: Clip.antiAlias,
          child: Column(
            children: [
              for (var index = 0; index < children.length; index++) ...[
                children[index],
                if (index != children.length - 1)
                  Divider(
                    height: 1,
                    indent: 64,
                    color: Theme.of(context).colorScheme.outlineVariant,
                  ),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

class _SettingsChoiceRow extends StatelessWidget {
  const _SettingsChoiceRow({
    required this.icon,
    required this.title,
    required this.description,
    required this.value,
    required this.values,
    required this.onChanged,
  });

  final IconData icon;
  final String title;
  final String description;
  final String value;
  final List<String> values;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    return _SettingsRow(
      icon: icon,
      title: title,
      description: description,
      trailing: DropdownButton<String>(
        value: value,
        underline: const SizedBox.shrink(),
        borderRadius: BorderRadius.circular(12),
        items: [
          for (final option in values)
            DropdownMenuItem(value: option, child: Text(option)),
        ],
        onChanged: (next) {
          if (next != null) {
            onChanged(next);
          }
        },
      ),
    );
  }
}

class _SettingsActionRow extends StatelessWidget {
  const _SettingsActionRow({
    required this.icon,
    required this.title,
    required this.description,
    required this.detail,
    required this.actionLabel,
  });

  final IconData icon;
  final String title;
  final String description;
  final String detail;
  final String actionLabel;

  @override
  Widget build(BuildContext context) {
    return _SettingsRow(
      icon: icon,
      title: title,
      description: description,
      detail: detail,
      trailing: OutlinedButton(onPressed: () {}, child: Text(actionLabel)),
    );
  }
}

class _SettingsInfoRow extends StatelessWidget {
  const _SettingsInfoRow({
    required this.icon,
    required this.title,
    required this.detail,
  });

  final IconData icon;
  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return _SettingsRow(
      icon: icon,
      title: title,
      description: detail,
      trailing: const Icon(Icons.chevron_right),
    );
  }
}

class _SettingsRow extends StatelessWidget {
  const _SettingsRow({
    required this.icon,
    required this.title,
    required this.description,
    required this.trailing,
    this.detail,
  });

  final IconData icon;
  final String title;
  final String description;
  final String? detail;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      constraints: const BoxConstraints(minHeight: 82),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
        child: Row(
          children: [
            Icon(icon),
            const SizedBox(width: 20),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: Theme.of(context).textTheme.titleSmall),
                  const SizedBox(height: 3),
                  Text(
                    description,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  if (detail != null) ...[
                    const SizedBox(height: 3),
                    Text(
                      detail!,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.labelSmall,
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(width: 20),
            trailing,
          ],
        ),
      ),
    );
  }
}
