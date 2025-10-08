# ==============================================================================
# Function: rmf_site_generate
#
# Generates a world file and a navigation graph directory from an RMF building.yaml.
#
# Arguments:
#   INPUT_MAP           <path_to_yaml_file>    (REQUIRED) Input path to a single RMF building input file (.building.yaml/.json/.ron).
#   OUTPUT_WORLD_DIR    <path_to_world_dir>    (REQUIRED) Output directory for the output world file.
#   OUTPUT_NAV_DIR      <path_to_nav_dir>      (REQUIRED) Output directory for the nav_graph files.
#                                                         Generated nav_graphs will be placed inside.
#   DEPENDS             <list_of_dependencies> (OPTIONAL) List of files or targets that this generation depends on.
#
# Example:
#   rmf_site_generate(
#     INPUT_MAP hotel.building.yaml
#     OUTPUT_WORLD_DIR ${CMAKE_CURRENT_BINARY_DIR}/maps/hotel/
#     OUTPUT_NAV_DIR ${CMAKE_CURRENT_BINARY_DIR}/maps/hotel/nav_graphs
#   )
# ==============================================================================
function(rmf_site_generate)
  set(options)
  set(one_value_args INPUT_MAP OUTPUT_WORLD_DIR OUTPUT_NAV_DIR)
  set(multi_value_args DEPENDS)

  cmake_parse_arguments(
    rmf_site_gen
    "${options}"
    "${one_value_args}"
    "${multi_value_args}"
    ${ARGN}
  )

  # --- Argument Validation ---
  if(NOT rmf_site_gen_INPUT_MAP)
    message(FATAL_ERROR "rmf_site_generate: INPUT_MAP argument is required.")
  endif()
  if(NOT rmf_site_gen_OUTPUT_WORLD_DIR)
    message(FATAL_ERROR "rmf_site_generate: OUTPUT_WORLD_DIR argument is required.")
  endif()
  if(NOT rmf_site_gen_OUTPUT_NAV_DIR)
    message(FATAL_ERROR "rmf_site_generate: OUTPUT_NAV_DIR argument is required.")
  endif()

  # Ensure the output directories exist before running the command
  file(MAKE_DIRECTORY "${rmf_site_gen_OUTPUT_WORLD_DIR}")
  file(MAKE_DIRECTORY "${rmf_site_gen_OUTPUT_NAV_DIR}")

  set(output_world_phony ${rmf_site_gen_OUTPUT_WORLD_DIR}/phony)
  set(output_nav_graphs_phony ${rmf_site_gen_OUTPUT_NAV_DIR}/phony)

  # Add a custom command to run rmf_site_editor.
  add_custom_command(
    OUTPUT ${output_world_phony} ${output_nav_graphs_phony}
    COMMAND rmf_site_editor
            ${rmf_site_gen_INPUT_MAP}
            --export-sdf ${rmf_site_gen_OUTPUT_WORLD_DIR}
            --export-nav ${rmf_site_gen_OUTPUT_NAV_DIR}
    DEPENDS ${rmf_site_gen_INPUT_MAP} ${rmf_site_gen_DEPENDS}
    VERBATIM
  )

  # Define a unique target name for this generation task
  get_filename_component(input_basename "${rmf_site_gen_INPUT_MAP}" NAME_WE)
  set(target_name "generate_${input_basename}_site")

  add_custom_target(generate_${target_name}_nav_graphs ALL
    DEPENDS ${output_nav_graphs_phony}
  )

endfunction()


# ==============================================================================
# Function: rmf_site_generate_map_package
#
# Generates a complete RMF map package from a site input directory.
# The package includes the world file and a navigation graph directory.
#
# Arguments:
#   INPUT_MAP_DIR       <path_to_map_dir>       (REQUIRED) Input path to a directory containing map files (.building.yaml/.site.json/.site.ron).
#   OUTPUT_PACKAGE_DIR  <path_to_pkg_dir>       (REQUIRED) Output directory for the generated package.
#   DEPENDS             <list_of_dependencies>  (OPTIONAL) List of files or targets that this generation depends on.
#
# Example:
#   rmf_site_generate_map_package(
#     INPUT_MAP_DIR maps
#     OUTPUT_PACKAGE_DIR ${CMAKE_CURRENT_BINARY_DIR}/maps
#   )
# ==============================================================================
function(rmf_site_generate_map_package)
  set(options)
  set(one_value_args INPUT_MAP_DIR OUTPUT_PACKAGE_DIR)
  set(multi_value_args DEPENDS)

  cmake_parse_arguments(
    rmf_site_pkg_gen
    "${options}"
    "${one_value_args}"
    "${multi_value_args}"
    ${ARGN}
  )

  # --- Argument Validation ---
  if(NOT rmf_site_pkg_gen_INPUT_MAP_DIR)
    message(FATAL_ERROR "rmf_site_generate_map_package: INPUT_MAP_DIR argument is required.")
  endif()
  if(NOT rmf_site_pkg_gen_OUTPUT_PACKAGE_DIR)
    message(FATAL_ERROR "rmf_site_generate_map_package: OUTPUT_PACKAGE_DIR argument is required.")
  endif()

  # Consolidate all the relevant map files
  file(GLOB_RECURSE
    site_paths
    "${rmf_site_pkg_gen_INPUT_MAP_DIR}/*.building.yaml"
    "${rmf_site_pkg_gen_INPUT_MAP_DIR}/*.site.json"
    "${rmf_site_pkg_gen_INPUT_MAP_DIR}/*.site.ron"
  )

  foreach(path ${site_paths})
    # Get the output world name
    string(REGEX REPLACE "\\.[^.]*\.[^.]*$" "" no_extension_path ${path})
    string(REGEX MATCH "[^\/]+$" world_name  ${no_extension_path})

    set(map_path ${path})
    set(output_world_name ${world_name})
    set(output_dir ${rmf_site_pkg_gen_OUTPUT_PACKAGE_DIR}/${output_world_name})
    set(output_world_dir ${output_dir}/)
    set(output_nav_dir ${output_dir}/nav_graphs/)

    # Run the command to generate world file and nav graphs
    rmf_site_generate(
      INPUT_MAP ${map_path}
      OUTPUT_WORLD_DIR ${output_world_dir}
      OUTPUT_NAV_DIR ${output_nav_dir}
    )

  endforeach()

endfunction()
