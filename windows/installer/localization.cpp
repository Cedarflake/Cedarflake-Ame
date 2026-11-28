#include "localization.h"

#include <msiquery.h>

#include <algorithm>
#include <cwctype>
#include <stdexcept>

namespace ame::installer {
namespace {

void Check(UINT result) {
  if (result != ERROR_SUCCESS) throw std::runtime_error("Cannot load installer language resources");
}

std::wstring Field(MSIHANDLE record, UINT field) {
  DWORD length = 0;
  const auto result = MsiRecordGetStringW(record, field, L"", &length);
  if ((result != ERROR_MORE_DATA && result != ERROR_SUCCESS) || length > 32768) {
    throw std::runtime_error("Installer language resource exceeds its text bound");
  }
  std::vector<wchar_t> text(static_cast<size_t>(length) + 1);
  DWORD capacity = static_cast<DWORD>(text.size());
  Check(MsiRecordGetStringW(record, field, text.data(), &capacity));
  return std::wstring(text.data(), capacity);
}

std::wstring Property(MSIHANDLE session, const wchar_t* name) {
  wchar_t value[32]{};
  DWORD length = 32;
  const auto result = MsiGetPropertyW(session, name, value, &length);
  if (result != ERROR_SUCCESS) return L"";
  return std::wstring(value, length);
}

InstallerLanguage SessionLanguage(MSIHANDLE session) {
  const auto selected = Property(session, L"AME_INSTALLER_LANGUAGE");
  if (selected == L"1033") return InstallerLanguage::English;
  if (selected == L"1041") return InstallerLanguage::Japanese;
  if (selected == L"2052") return InstallerLanguage::Chinese;

  ULONG count = 0, length = 0;
  if (!GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &count, nullptr, &length) ||
      length == 0 || length > 32768) {
    wchar_t name[LOCALE_NAME_MAX_LENGTH]{};
    LCIDToLocaleName(GetUserDefaultUILanguage(), name, LOCALE_NAME_MAX_LENGTH, 0);
    return ResolveInstallerLanguage({name});
  }
  std::vector<wchar_t> names(length);
  if (!GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &count, names.data(), &length)) {
    throw std::runtime_error("Cannot read Windows display-language preferences");
  }
  std::vector<std::wstring> preferences;
  size_t offset = 0;
  while (offset < names.size() && names[offset] != L'\0') {
    const auto end = std::find(names.begin() + offset, names.end(), L'\0');
    if (end == names.end()) throw std::runtime_error("Invalid Windows language list");
    preferences.emplace_back(names.begin() + offset, end);
    offset = static_cast<size_t>(end - names.begin()) + 1;
  }
  return ResolveInstallerLanguage(preferences);
}

void PopulateTable(MSIHANDLE database, const wchar_t* table, InstallerLanguage language) {
  PMSIHANDLE target;
  const auto target_sql = L"SELECT * FROM `" + std::wstring(table) + L"`";
  Check(MsiDatabaseOpenViewW(database, target_sql.c_str(), &target));
  Check(MsiViewExecute(target, 0));
  PMSIHANDLE types;
  Check(MsiViewGetColumnInfo(target, MSICOLINFO_TYPES, &types));
  const auto fields = MsiRecordGetFieldCount(types);

  const auto sql = L"SELECT `Row`, `Field`, `Value` FROM `AmeInstallerText` WHERE `Language` = " +
      std::to_wstring(static_cast<int>(language)) + L" AND `Table` = '" + table +
      L"' ORDER BY `Row`, `Field`";
  PMSIHANDLE source;
  Check(MsiDatabaseOpenViewW(database, sql.c_str(), &source));
  Check(MsiViewExecute(source, 0));
  PMSIHANDLE row;
  int current = 0;
  UINT next_field = 1;
  size_t cells = 0;
  for (;;) {
    PMSIHANDLE cell;
    const auto result = MsiViewFetch(source, &cell);
    if (result == ERROR_NO_MORE_ITEMS) break;
    Check(result);
    if (++cells > 12000) throw std::runtime_error("Installer language table exceeds its bound");
    const int row_id = MsiRecordGetInteger(cell, 1);
    const int field_id = MsiRecordGetInteger(cell, 2);
    if (row_id != current) {
      if (row_id != current + 1) throw std::runtime_error("Incomplete installer language rows");
      if (row) {
        if (next_field != fields + 1) throw std::runtime_error("Incomplete installer language fields");
        Check(MsiViewModify(target, MSIMODIFY_INSERT_TEMPORARY, row));
      }
      row = MsiCreateRecord(fields);
      if (!row) throw std::runtime_error("Cannot allocate installer language row");
      current = row_id;
      next_field = 1;
    }
    if (field_id != static_cast<int>(next_field) || next_field > fields) {
      throw std::runtime_error("Invalid installer language field order");
    }
    if (!MsiRecordIsNull(cell, 3)) {
      const auto value = Field(cell, 3);
      const auto type = Field(types, next_field);
      if (!type.empty() && (type[0] == L'i' || type[0] == L'I')) {
        size_t consumed = 0;
        const int number = std::stoi(value, &consumed);
        if (consumed != value.size()) throw std::runtime_error("Invalid installer language number");
        Check(MsiRecordSetInteger(row, next_field, number));
      } else {
        Check(MsiRecordSetStringW(row, next_field, value.c_str()));
      }
    }
    ++next_field;
  }
  if (row) {
    if (next_field != fields + 1) throw std::runtime_error("Incomplete installer language fields");
    Check(MsiViewModify(target, MSIMODIFY_INSERT_TEMPORARY, row));
  }
}

void SetMessages(MSIHANDLE session, MSIHANDLE database, InstallerLanguage language) {
  PMSIHANDLE view;
  const auto sql = L"SELECT `Property`, `Value` FROM `AmeInstallerMessage` WHERE `Language` = " +
      std::to_wstring(static_cast<int>(language));
  Check(MsiDatabaseOpenViewW(database, sql.c_str(), &view));
  Check(MsiViewExecute(view, 0));
  size_t count = 0;
  for (;;) {
    PMSIHANDLE row;
    const auto result = MsiViewFetch(view, &row);
    if (result == ERROR_NO_MORE_ITEMS) break;
    Check(result);
    const auto name = Field(row, 1);
    if (++count > 32 || name.rfind(L"AmeMessage", 0) != 0) {
      throw std::runtime_error("Unexpected installer message property");
    }
    PMSIHANDLE format = MsiCreateRecord(0);
    Check(MsiRecordSetStringW(format, 0, Field(row, 2).c_str()));
    DWORD length = 0;
    const auto sizing = MsiFormatRecordW(session, format, L"", &length);
    if ((sizing != ERROR_SUCCESS && sizing != ERROR_MORE_DATA) || length > 32768) {
      throw std::runtime_error("Installer message exceeds its bound");
    }
    std::vector<wchar_t> text(static_cast<size_t>(length) + 1);
    DWORD capacity = static_cast<DWORD>(text.size());
    Check(MsiFormatRecordW(session, format, text.data(), &capacity));
    Check(MsiSetPropertyW(session, name.c_str(), text.data()));
  }
}

}  // namespace

InstallerLanguage ResolveInstallerLanguage(const std::vector<std::wstring>& preferences) {
  for (auto name : preferences) {
    std::transform(name.begin(), name.end(), name.begin(),
        [](wchar_t character) { return static_cast<wchar_t>(std::towlower(character)); });
    if (name == L"en" || name.rfind(L"en-", 0) == 0) return InstallerLanguage::English;
    if (name == L"ja" || name.rfind(L"ja-", 0) == 0) return InstallerLanguage::Japanese;
    if (name == L"zh" || name == L"zh-cn" || name == L"zh-sg" || name == L"zh-hans" ||
        name.rfind(L"zh-hans-", 0) == 0) return InstallerLanguage::Chinese;
  }
  return InstallerLanguage::English;
}

void LocalizeInstaller(MSIHANDLE session) {
  const auto language = SessionLanguage(session);
  Check(MsiSetPropertyW(session, L"AME_INSTALLER_LANGUAGE",
      std::to_wstring(static_cast<int>(language)).c_str()));
  PMSIHANDLE database = MsiGetActiveDatabase(session);
  if (!database) throw std::runtime_error("Installer database is unavailable");
  SetMessages(session, database, language);
  PMSIHANDLE marker;
  Check(MsiDatabaseOpenViewW(database, L"SELECT `Language` FROM `AmeInstallerLocale`", &marker));
  Check(MsiViewExecute(marker, 0));
  PMSIHANDLE existing;
  const auto result = MsiViewFetch(marker, &existing);
  if (result == ERROR_SUCCESS) {
    if (MsiRecordGetInteger(existing, 1) != static_cast<int>(language)) {
      throw std::runtime_error("Installer language changed within one session");
    }
    return;
  }
  if (result != ERROR_NO_MORE_ITEMS) Check(result);
  for (const auto table : {L"Dialog", L"Control", L"TextStyle", L"UIText", L"Error", L"ActionText",
                           L"RadioButton", L"ComboBox", L"ListBox", L"ListView"}) {
    if (MsiDatabaseIsTablePersistentW(database, table) == MSICONDITION_NONE) continue;
    PopulateTable(database, table, language);
  }
  PMSIHANDLE selected = MsiCreateRecord(1);
  Check(MsiRecordSetInteger(selected, 1, static_cast<int>(language)));
  Check(MsiViewModify(marker, MSIMODIFY_INSERT_TEMPORARY, selected));
}

}  // namespace ame::installer

extern "C" __declspec(dllexport) UINT __stdcall AmeLocalizeInstaller(MSIHANDLE session) {
  try {
    ame::installer::LocalizeInstaller(session);
    return ERROR_SUCCESS;
  } catch (...) {
    return ERROR_INSTALL_FAILURE;
  }
}
