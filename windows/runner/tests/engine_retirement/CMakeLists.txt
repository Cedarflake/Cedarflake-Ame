cmake_minimum_required(VERSION 3.14)
project(ame_window_engine_lifecycle_tests LANGUAGES CXX)

if(NOT WIN32 OR NOT MSVC OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
  message(FATAL_ERROR "The engine lifecycle fixture requires Windows x64 MSVC")
endif()
foreach(required AME_FLUTTER_ENGINE_ROOT AME_FLUTTER_ASSETS_DIR AME_FLUTTER_ICU_PATH)
  if(NOT DEFINED ${required} OR NOT IS_ABSOLUTE "${${required}}")
    message(FATAL_ERROR "Supply an absolute -D${required}=... path")
  endif()
endforeach()

get_filename_component(WINDOWS_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../.." ABSOLUTE)
set(WRAPPER_ROOT "${AME_FLUTTER_ENGINE_ROOT}/cpp_client_wrapper")
set(ENGINE_DLL "${AME_FLUTTER_ENGINE_ROOT}/flutter_windows.dll")
foreach(required "${ENGINE_DLL}" "${ENGINE_DLL}.lib" "${AME_FLUTTER_ICU_PATH}"
    "${AME_FLUTTER_ASSETS_DIR}/kernel_blob.bin"
    "${WRAPPER_ROOT}/flutter_view_controller.cc")
  if(NOT EXISTS "${required}" OR IS_DIRECTORY "${required}")
    message(FATAL_ERROR "A prepared Debug SDK or fixture input is missing: ${required}")
  endif()
endforeach()

add_executable(ame_window_engine_lifecycle_test
  "flutter_window_engine_lifecycle_test.cpp"
  "../../flutter_window.cpp"
  "../../win32_window.cpp"
  "${WRAPPER_ROOT}/core_implementations.cc"
  "${WRAPPER_ROOT}/standard_codec.cc"
  "${WRAPPER_ROOT}/flutter_engine.cc"
  "${WRAPPER_ROOT}/flutter_view_controller.cc"
)
target_compile_features(ame_window_engine_lifecycle_test PRIVATE cxx_std_17)
target_compile_definitions(ame_window_engine_lifecycle_test PRIVATE
  UNICODE _UNICODE NOMINMAX _WIN32_WINNT=0x0A00)
target_compile_options(ame_window_engine_lifecycle_test PRIVATE /W4 /WX /wd4100 /EHsc)
target_include_directories(ame_window_engine_lifecycle_test PRIVATE
  "../.." "${WINDOWS_ROOT}" "${AME_FLUTTER_ENGINE_ROOT}" "${WRAPPER_ROOT}/include")
target_link_libraries(ame_window_engine_lifecycle_test PRIVATE
  "${ENGINE_DLL}.lib" ole32 user32 advapi32 dwmapi)
add_custom_command(TARGET ame_window_engine_lifecycle_test POST_BUILD
  COMMAND "${CMAKE_COMMAND}" -E copy_if_different "${ENGINE_DLL}"
    "$<TARGET_FILE_DIR:ame_window_engine_lifecycle_test>/flutter_windows.dll"
  VERBATIM)

enable_testing()
foreach(phase explicit_destroy scope_exit)
  string(REPLACE "_" "-" argument "${phase}")
  add_test(NAME "engine_lifecycle_${phase}" COMMAND ame_window_engine_lifecycle_test
    "${argument}" "${AME_FLUTTER_ASSETS_DIR}" "${AME_FLUTTER_ICU_PATH}")
  set_tests_properties("engine_lifecycle_${phase}" PROPERTIES TIMEOUT 30)
endforeach()
