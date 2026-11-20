if(FLUTTER_TARGET_PLATFORM STREQUAL "windows-arm64")
  set(AME_RUNTIME_ARCH arm64)
elseif(FLUTTER_TARGET_PLATFORM STREQUAL "windows-x64")
  set(AME_RUNTIME_ARCH x64)
else()
  message(FATAL_ERROR "No classification runtime for ${FLUTTER_TARGET_PLATFORM}")
endif()

find_program(AME_POWERSHELL NAMES pwsh powershell)
if(NOT AME_POWERSHELL)
  message(FATAL_ERROR "PowerShell is required to prepare the classification runtime")
endif()
set(AME_CLASSIFICATION_RUNTIME_DIR "${CMAKE_BINARY_DIR}/classification-runtime")
set(AME_CLASSIFICATION_ACCELERATOR_DIR "${CMAKE_BINARY_DIR}/classification-accelerator")
add_custom_target(ame_classification_runtime
  COMMAND "${AME_POWERSHELL}" -NoProfile -NonInteractive -ExecutionPolicy Bypass
    -File "${CMAKE_CURRENT_LIST_DIR}/../../tool/release_prepare_classification_runtime.ps1"
    -OutputDirectory "${AME_CLASSIFICATION_RUNTIME_DIR}" -Architecture "${AME_RUNTIME_ARCH}"
  COMMAND "${AME_POWERSHELL}" -NoProfile -NonInteractive -ExecutionPolicy Bypass
    -File "${CMAKE_CURRENT_LIST_DIR}/../../tool/release_prepare_classification_runtime.ps1"
    -OutputDirectory "${AME_CLASSIFICATION_ACCELERATOR_DIR}" -Architecture "${AME_RUNTIME_ARCH}" -Accelerator
  VERBATIM
)
add_dependencies(${BINARY_NAME} ame_classification_runtime)
install(DIRECTORY "${AME_CLASSIFICATION_RUNTIME_DIR}/"
  DESTINATION "${INSTALL_BUNDLE_DATA_DIR}/classification-runtime"
  COMPONENT Runtime
)
install(DIRECTORY "${AME_CLASSIFICATION_ACCELERATOR_DIR}/"
  DESTINATION "${INSTALL_BUNDLE_DATA_DIR}/classification-accelerator"
  COMPONENT Runtime
)
