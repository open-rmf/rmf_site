cmake_minimum_required(VERSION 3.8)
project(rmf_site_maps)

include(GNUInstallDirs)

# Default to C++17
if(NOT CMAKE_CXX_STANDARD)
  set(CMAKE_CXX_STANDARD 17)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
endif()
if(CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  add_compile_options(-Wall -Wextra -Wpedantic)
endif()

# Include rmf_site_cmake module
find_package(rmf_site_cmake REQUIRED)

# Copy source maps to the install directory
install(DIRECTORY
  maps
  DESTINATION ${CMAKE_INSTALL_DATADIR}/${PROJECT_NAME}
)

rmf_site_generate_map_package(
  INPUT_MAP_DIR maps
  OUTPUT_PACKAGE_DIR ${CMAKE_CURRENT_BINARY_DIR}/maps
)

# Copy generated world files and nav graphs to install folder
install(
  DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/maps
  DESTINATION ${CMAKE_INSTALL_DATADIR}/${PROJECT_NAME}
)
