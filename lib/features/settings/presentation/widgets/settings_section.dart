import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_typography.dart";

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
          child: Text(
            title,
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
              fontWeight: ameFontWeightSemibold,
            ),
          ),
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
      title: Text(
        title,
        style: Theme.of(
          context,
        ).textTheme.bodyLarge?.copyWith(fontWeight: ameFontWeightSemibold),
      ),
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

@immutable
class SettingsChoiceEntry<T> {
  const SettingsChoiceEntry({
    required this.value,
    required this.label,
    this.enabled = true,
  });

  final T value;
  final String label;
  final bool enabled;
}

class SettingsChoice<T> extends StatefulWidget {
  const SettingsChoice({
    required this.value,
    required this.entries,
    required this.onSelected,
    this.enabled = true,
    this.width = 176,
    this.selectedLabel,
    super.key,
  }) : assert(width > 0);

  final T value;
  final List<SettingsChoiceEntry<T>> entries;
  final ValueChanged<T?> onSelected;
  final bool enabled;
  final double width;
  final String? selectedLabel;

  @override
  State<SettingsChoice<T>> createState() => _SettingsChoiceState<T>();
}

class _SettingsChoiceState<T> extends State<SettingsChoice<T>> {
  static const _height = 56.0;

  bool _isMenuOpen = false;

  @override
  Widget build(BuildContext context) {
    final selectedLabel = _selectedLabel();
    return AmePopupMenuButton<T>(
      labels: widget.entries.map((entry) => entry.label),
      menuWidth: widget.width,
      initialValue: widget.value,
      items: [
        for (final entry in widget.entries)
          CheckedPopupMenuItem<T>(
            key: ValueKey("settings-choice-option-${entry.value}"),
            value: entry.value,
            checked: entry.value == widget.value,
            enabled: entry.enabled,
            child: Text(
              entry.label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.labelLarge?.copyWith(
                fontWeight: entry.value == widget.value
                    ? ameFontWeightSemibold
                    : ameFontWeightMedium,
              ),
            ),
          ),
      ],
      onOpenChanged: (isOpen) {
        if (mounted && _isMenuOpen != isOpen) {
          setState(() {
            _isMenuOpen = isOpen;
          });
        }
      },
      onSelected: (value) => widget.onSelected(value),
      builder: (context, openMenu) => SizedBox(
        width: widget.width,
        height: _height,
        child: OutlinedButton(
          key: ValueKey(widget.value),
          onPressed: widget.enabled ? openMenu : null,
          style: OutlinedButton.styleFrom(
            alignment: AlignmentDirectional.centerStart,
            padding: const EdgeInsetsDirectional.only(start: 16, end: 12),
            textStyle: Theme.of(context).textTheme.bodyLarge,
            shape: const RoundedRectangleBorder(
              borderRadius: BorderRadius.all(Radius.circular(4)),
            ),
          ),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  selectedLabel,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Icon(
                _isMenuOpen
                    ? Symbols.arrow_drop_up_rounded
                    : Symbols.arrow_drop_down_rounded,
              ),
            ],
          ),
        ),
      ),
    );
  }

  String _selectedLabel() {
    for (final entry in widget.entries) {
      if (entry.value == widget.value) {
        return entry.label;
      }
    }
    return widget.selectedLabel ?? widget.value.toString();
  }
}
