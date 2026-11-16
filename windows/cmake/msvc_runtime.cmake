set(CMAKE_INSTALL_SYSTEM_RUNTIME_LIBS_SKIP TRUE)
set(CMAKE_INSTALL_DEBUG_LIBRARIES FALSE)
include(InstallRequiredSystemLibraries)

set(AME_MODEL_MSVC_RUNTIME_LIBS)
foreach(runtime_name msvcp140.dll msvcp140_1.dll vcruntime140.dll vcruntime140_1.dll)
  set(runtime_found FALSE)
  foreach(runtime_path IN LISTS CMAKE_INSTALL_SYSTEM_RUNTIME_LIBS)
    get_filename_component(runtime_file "${runtime_path}" NAME)
    if(runtime_file STREQUAL runtime_name AND EXISTS "${runtime_path}")
      set(runtime_found TRUE)
      list(APPEND AME_MODEL_MSVC_RUNTIME_LIBS "${runtime_path}")
    endif()
  endforeach()
  if(NOT runtime_found)
    message(FATAL_ERROR "The MSVC redistributable is missing ${runtime_name}")
  endif()
endforeach()

install(PROGRAMS ${CMAKE_INSTALL_SYSTEM_RUNTIME_LIBS}
  DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
  CONFIGURATIONS Profile;Release
  COMPONENT Runtime)

foreach(runtime_directory classification-runtime classification-accelerator)
  install(PROGRAMS ${AME_MODEL_MSVC_RUNTIME_LIBS}
    DESTINATION "${INSTALL_BUNDLE_DATA_DIR}/${runtime_directory}"
    CONFIGURATIONS Profile;Release
    COMPONENT Runtime)
endforeach()
