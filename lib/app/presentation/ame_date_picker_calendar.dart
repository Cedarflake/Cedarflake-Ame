import "package:material_ui/material_ui.dart";

Future<DateTime?> showAmeDatePicker({
  required BuildContext context,
  required String helpText,
  required DateTime initialDate,
  required DateTime firstDate,
  required DateTime lastDate,
}) => showDatePicker(
  context: context,
  helpText: helpText,
  initialDate: initialDate,
  firstDate: firstDate,
  lastDate: lastDate,
  calendarDelegate: AmeDatePickerCalendar(
    separateWeekday: MediaQuery.orientationOf(context) == Orientation.landscape,
  ),
  builder: (context, child) {
    final theme = Theme.of(context);
    return Theme(
      data: theme.copyWith(
        // The pinned picker uses headlineSmall directly in landscape
        // input mode, bypassing its own headerHeadlineStyle setting.
        textTheme: theme.textTheme.copyWith(
          headlineSmall: theme.textTheme.titleMedium?.copyWith(
            fontWeight: theme.textTheme.headlineSmall?.fontWeight,
          ),
        ),
        datePickerTheme: theme.datePickerTheme.copyWith(
          headerHeadlineStyle: theme.textTheme.headlineSmall,
        ),
      ),
      child: child!,
    );
  },
);

class AmeDatePickerCalendar extends GregorianCalendarDelegate {
  const AmeDatePickerCalendar({required this.separateWeekday});

  final bool separateWeekday;

  @override
  String formatMediumDate(DateTime date, MaterialLocalizations localizations) {
    final text = super.formatMediumDate(date, localizations);
    return separateWeekday
        ? text.replaceFirst(RegExp(r"(?=周|星期)"), "\n")
        : text;
  }
}
