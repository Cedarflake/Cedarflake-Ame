cmake_minimum_required(VERSION 3.15)
project(ame_installer_admission LANGUAGES CXX)

if(NOT WIN32 OR NOT MSVC OR NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
  message(FATAL_ERROR "The installer requires Windows x64 MSVC")
endif()

add_library(ame_installer_policy STATIC protected_paths.cpp service_configuration.cpp
            installed_broker_identity.cpp registered_product.cpp service_start.cpp
            application_destination.cpp service_privileges.cpp)
target_compile_features(ame_installer_policy PUBLIC cxx_std_17)
target_compile_definitions(ame_installer_policy PUBLIC UNICODE _UNICODE NOMINMAX)
target_link_libraries(ame_installer_policy PUBLIC advapi32 shell32 ole32 bcrypt msi)

add_library(ame_installer_admission SHARED admission.cpp)
target_link_libraries(ame_installer_admission PRIVATE ame_installer_policy msi)

add_executable(ame_installer_policy_test tests/policy_test.cpp tests/identity_test.cpp
               tests/lifecycle_test.cpp tests/privileges_test.cpp)
target_link_libraries(ame_installer_policy_test PRIVATE ame_installer_policy)
foreach(target ame_installer_policy ame_installer_admission ame_installer_policy_test)
  target_compile_options(${target} PRIVATE /W4 /WX /EHsc /utf-8)
  set_property(TARGET ${target} PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
endforeach()

enable_testing()
add_test(NAME installer_policy COMMAND ame_installer_policy_test)
set_tests_properties(installer_policy PROPERTIES TIMEOUT 15)
