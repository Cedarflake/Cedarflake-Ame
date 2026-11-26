import "package:intl/intl.dart";

import "../../../../../app/presentation/ame_localizations.dart";
import "../../../../updates/domain/application_update.dart";

String updatePolicyLabel(
  AppLocalizations strings,
  ApplicationUpdatePolicy policy,
) => switch (policy) {
  ApplicationUpdatePolicy.manual => strings.updateManual,
  ApplicationUpdatePolicy.check => strings.updateCheckAutomatically,
  ApplicationUpdatePolicy.download => strings.updateDownloadAutomatically,
};

String updateChannelLabel(
  AppLocalizations strings,
  ApplicationUpdateChannel channel,
) => switch (channel) {
  ApplicationUpdateChannel.stable => strings.updateStable,
  ApplicationUpdateChannel.preview => strings.updatePreview,
};

String updateFailureLabel(
  AppLocalizations strings,
  ApplicationUpdateFailure failure,
) => switch (failure.code) {
  ApplicationUpdateFailureCode.unavailable => strings.updateUnavailable,
  ApplicationUpdateFailureCode.network => strings.updateNetworkFailure,
  ApplicationUpdateFailureCode.invalidRelease => strings.updateReleaseFailure,
  ApplicationUpdateFailureCode.damagedDownload => strings.updatePackageFailure,
  ApplicationUpdateFailureCode.storage => strings.updateStorageFailure,
  ApplicationUpdateFailureCode.installation =>
    strings.updateInstallationFailure,
  ApplicationUpdateFailureCode.shutdown => strings.updateShutdownFailure,
};

String updateSize(AppLocalizations strings, int bytes) =>
    "${NumberFormat("0.0", strings.localeName).format(bytes / (1024 * 1024))} MB";

String updatePercent(AppLocalizations strings, double value) =>
    NumberFormat.decimalPercentPattern(
      locale: strings.localeName,
      decimalDigits: 1,
    ).format(value);
