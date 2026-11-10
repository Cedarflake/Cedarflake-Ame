import "package:material_ui/material_ui.dart";

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
