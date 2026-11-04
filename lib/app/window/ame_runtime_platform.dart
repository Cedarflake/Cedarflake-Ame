import "ame_runtime_platform_stub.dart"
    if (dart.library.io) "ame_runtime_platform_io.dart"
    as host;

bool get isAmeWindowsHost => host.isAmeWindowsHost;
