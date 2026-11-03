# The Flutter tooling requires that developers have a version of Visual Studio
# installed that includes CMake 3.14 or later. You should not increase this
# version, as doing so will cause the plugin to fail to compile for some
# customers of the plugin.
cmake_minimum_required(VERSION 3.14)

# Project-level configuration.
set(PROJECT_NAME "rust_lib_cedarflake_ame")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
apply_cargokit(${PROJECT_NAME} ../../../../../../rust rust_lib_cedarflake_ame "")

if(FLUTTER_TARGET_PLATFORM STREQUAL "windows-arm64")
  set(AME_RUST_TARGET "aarch64-pc-windows-msvc")
else()
  set(AME_RUST_TARGET "x86_64-pc-windows-msvc")
endif()
set(AME_CLASSIFICATION_WORKER
  "${CMAKE_CURRENT_BINARY_DIR}/cargokit_build/${AME_RUST_TARGET}/$<IF:$<CONFIG:Debug>,debug,release>/cedarflake_ame_classification_worker.exe"
)

# List of absolute paths to libraries that should be bundled with the plugin.
# This list could contain prebuilt libraries, or libraries created by an
# external build triggered from this build file.
set(rust_lib_cedarflake_ame_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  "${AME_CLASSIFICATION_WORKER}"
  PARENT_SCOPE
)
