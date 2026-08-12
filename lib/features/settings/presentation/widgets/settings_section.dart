import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

class SettingsSection extends StatelessWidget {
  const SettingsSection({
    required this.title,
    required this.children,
    super.key,
  });

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

class SettingsRow extends StatelessWidget {
  const SettingsRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.trailing,
    this.onTap,
    this.enabled = true,
    super.key,
  });

  final IconData icon;
  final String title;
  final Widget subtitle;
  final Widget? trailing;
  final VoidCallback? onTap;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      contentPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
      minLeadingWidth: 24,
      horizontalTitleGap: 20,
      leading: Icon(icon),
      title: Text(title),
      subtitle: Padding(
        padding: const EdgeInsets.only(top: 4),
        child: DefaultTextStyle.merge(
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
          child: subtitle,
        ),
      ),
      trailing: trailing,
      onTap: onTap,
      enabled: enabled,
    );
  }
}

class SettingsChoice<T> extends StatelessWidget {
  const SettingsChoice({
    required this.value,
    required this.entries,
    required this.onSelected,
    this.enabled = true,
    super.key,
  });

  final T value;
  final List<DropdownMenuEntry<T>> entries;
  final ValueChanged<T?> onSelected;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return DropdownMenu<T>(
      key: ValueKey(value),
      width: 176,
      initialSelection: value,
      enabled: enabled,
      enableSearch: false,
      requestFocusOnTap: false,
      selectOnly: true,
      trailingIcon: const Icon(Symbols.arrow_drop_down_rounded),
      selectedTrailingIcon: const Icon(Symbols.arrow_drop_up_rounded),
      onSelected: onSelected,
      dropdownMenuEntries: entries,
    );
  }
}
