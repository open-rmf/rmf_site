cmake_minimum_required(VERSION 3.19)
project(rmf_site_cmake CXX)

# Default to C++20
if(NOT CMAKE_CXX_STANDARD)
  set(CMAKE_CXX_STANDARD 20)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
endif()

# Generate and install the package configuration files.
include(CMakePackageConfigHelpers)
include(GNUInstallDirs)

configure_package_config_file(
  cmake/${PROJECT_NAME}Config.cmake.in
  "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake"
  INSTALL_DESTINATION share/${PROJECT_NAME}/cmake
)
install(
  FILES
    "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake"
    "cmake/rmf_site_generate.cmake"
  DESTINATION
    share/${PROJECT_NAME}/cmake
)
