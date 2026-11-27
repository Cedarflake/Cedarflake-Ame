cmake_minimum_required(VERSION 3.19)
project(ame_installer_admission LANGUAGES CXX)

string(TOLOWER "${CMAKE_VS_PLATFORM_NAME}" platform)
if(NOT WIN32 OR NOT MSVC OR NOT platform MATCHES "^(x64|arm64)$")
  message(FATAL_ERROR "The installer requires Windows x64 or ARM64 MSVC")
endif()
if(platform STREQUAL "arm64")
  set(other_platform x64)
else()
  set(other_platform arm64)
endif()
set(families_file "${CMAKE_CURRENT_SOURCE_DIR}/product_families.json")
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${families_file}")
file(READ "${families_file}" families)
string(JSON upgrade_code GET "${families}" "${platform}")
string(JSON other_upgrade_code GET "${families}" "${other_platform}")

add_library(ame_installer_policy STATIC protected_paths.cpp service_configuration.cpp
            installed_broker_identity.cpp registered_product.cpp service_start.cpp
            application_destination.cpp service_privileges.cpp service_rollback.cpp
            user_data_identity.cpp user_data_cleanup.cpp)
target_compile_features(ame_installer_policy PUBLIC cxx_std_17)
target_compile_definitions(ame_installer_policy PUBLIC UNICODE _UNICODE NOMINMAX)
target_compile_definitions(ame_installer_policy PUBLIC
  AME_UPGRADE_CODE=L"{${upgrade_code}}" AME_OTHER_UPGRADE_CODE=L"{${other_upgrade_code}}")
target_link_libraries(ame_installer_policy PUBLIC advapi32 shell32 ole32 bcrypt msi wtsapi32)

add_library(ame_installer_admission SHARED admission.cpp user_data_cleanup_actions.cpp)
target_link_libraries(ame_installer_admission PRIVATE ame_installer_policy msi)

add_executable(ame_installer_policy_test tests/policy_test.cpp tests/identity_test.cpp
               tests/lifecycle_test.cpp tests/privileges_test.cpp tests/rollback_test.cpp
               tests/user_data_cleanup_test.cpp)
target_link_libraries(ame_installer_policy_test PRIVATE ame_installer_policy)
foreach(target ame_installer_policy ame_installer_admission ame_installer_policy_test)
  target_compile_options(${target} PRIVATE /W4 /WX /EHsc /utf-8)
  set_property(TARGET ${target} PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
endforeach()

enable_testing()
add_test(NAME installer_policy COMMAND ame_installer_policy_test)
set_tests_properties(installer_policy PROPERTIES TIMEOUT 15)
