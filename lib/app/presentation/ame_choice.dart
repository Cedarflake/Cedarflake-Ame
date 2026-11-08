import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "ame_menu.dart";
import "ame_typography.dart";

@immutable
class AmeChoiceEntry<T> {
  const AmeChoiceEntry({
    required this.value,
    required this.label,
    this.enabled = true,
  });

  final T value;
  final String label;
  final bool enabled;
}

class AmeChoice<T> extends StatefulWidget {
  const AmeChoice({
    required this.value,
    required this.entries,
    required this.onSelected,
    this.enabled = true,
    this.width = 176,
    this.selectedLabel,
    this.label,
    super.key,
  }) : assert(width > 0);

  final T value;
  final List<AmeChoiceEntry<T>> entries;
  final ValueChanged<T?> onSelected;
  final bool enabled;
  final double width;
  final String? selectedLabel;
  final String? label;

  @override
  State<AmeChoice<T>> createState() => _AmeChoiceState<T>();
}

class _AmeChoiceState<T> extends State<AmeChoice<T>> {
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
            key: ValueKey("ame-choice-option-${entry.value}"),
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
        child: _decorate(
          SizedBox(
            height: _height,
            child: OutlinedButton(
              key: ValueKey(widget.value),
              onPressed: widget.enabled ? openMenu : null,
              style: OutlinedButton.styleFrom(
                alignment: AlignmentDirectional.centerStart,
                padding: const EdgeInsetsDirectional.only(start: 16, end: 12),
                textStyle: Theme.of(context).textTheme.bodyLarge,
                side: widget.label == null ? null : BorderSide.none,
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
        ),
      ),
    );
  }

  Widget _decorate(Widget child) {
    if (widget.label == null) return child;
    return InputDecorator(
      decoration: InputDecoration(
        labelText: widget.label,
        enabled: widget.enabled,
        contentPadding: EdgeInsets.zero,
        border: const OutlineInputBorder(),
      ),
      child: child,
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
