cmake_minimum_required(VERSION 3.14)
project(ame_window_lifecycle_tests LANGUAGES CXX)

if(NOT WIN32 OR NOT MSVC OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
  message(FATAL_ERROR "The window lifecycle fixture requires Windows x64 MSVC")
endif()

get_filename_component(WINDOWS_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../.." ABSOLUTE)
set(FLUTTER_EPHEMERAL_ROOT "${WINDOWS_ROOT}/flutter/ephemeral" CACHE PATH
  "Existing repository-pinned Flutter Windows headers, wrapper, and DLL")
set(WRAPPER_ROOT "${FLUTTER_EPHEMERAL_ROOT}/cpp_client_wrapper")
set(ENGINE_DLL "${FLUTTER_EPHEMERAL_ROOT}/flutter_windows.dll")
if(NOT EXISTS "${ENGINE_DLL}" OR NOT EXISTS "${ENGINE_DLL}.lib")
  message(FATAL_ERROR "Prepare the pinned Flutter artifacts before this isolated build")
endif()

add_executable(ame_window_lifecycle_test
  "flutter_window_lifecycle_test.cpp"
  "../flutter_window.cpp"
  "../win32_window.cpp"
  "${WRAPPER_ROOT}/core_implementations.cc"
  "${WRAPPER_ROOT}/standard_codec.cc"
  "${WRAPPER_ROOT}/flutter_engine.cc"
  "${WRAPPER_ROOT}/flutter_view_controller.cc"
)
target_compile_features(ame_window_lifecycle_test PRIVATE cxx_std_17)
target_compile_definitions(ame_window_lifecycle_test PRIVATE UNICODE _UNICODE NOMINMAX)
target_compile_options(ame_window_lifecycle_test PRIVATE /W4 /WX /wd4100 /EHsc)
target_include_directories(ame_window_lifecycle_test PRIVATE
  ".." "${WINDOWS_ROOT}" "${FLUTTER_EPHEMERAL_ROOT}" "${WRAPPER_ROOT}/include")
target_link_libraries(ame_window_lifecycle_test PRIVATE
  "${ENGINE_DLL}.lib" user32 advapi32 dwmapi)
add_custom_command(TARGET ame_window_lifecycle_test POST_BUILD
  COMMAND "${CMAKE_COMMAND}" -E copy_if_different "${ENGINE_DLL}"
    "$<TARGET_FILE_DIR:ame_window_lifecycle_test>/flutter_windows.dll"
  VERBATIM)

enable_testing()
foreach(phase control startup teardown)
  add_test(NAME "window_lifecycle_${phase}" COMMAND ame_window_lifecycle_test "${phase}")
  set_tests_properties("window_lifecycle_${phase}" PROPERTIES TIMEOUT 15)
endforeach()
