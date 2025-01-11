cmake_minimum_required(VERSION 3.8)
project(rmw_zenoh_rs)

# Default to C++17
if(NOT CMAKE_CXX_STANDARD)
  set(CMAKE_CXX_STANDARD 17)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
endif()

if(CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  add_compile_options(-Wall -Wextra -Wpedantic)
endif()

# find dependencies
find_package(ament_cmake REQUIRED)

find_package(ament_index_cpp REQUIRED)
find_package(fastcdr CONFIG REQUIRED)
find_package(rcpputils REQUIRED)
find_package(rcutils REQUIRED)
find_package(rosidl_typesupport_fastrtps_c REQUIRED)
find_package(rosidl_typesupport_fastrtps_cpp REQUIRED)
find_package(rmw REQUIRED)

set(GENERATED_LIB_FILE ${CMAKE_CURRENT_BINARY_DIR}/release/librmw_zenoh_rs.so)

file(GLOB RUST_SOURCES
  "${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs"
)

add_library(rmw_zenoh_rs SHARED cpp/dummy_for_build.cpp)

set_source_files_properties(
  cpp/dummy_for_build.cpp
  PROPERTIES
    OBJECT_DEPENDS "${RUST_SOURCES};${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
)

add_custom_command(
  TARGET rmw_zenoh_rs
  POST_BUILD
  COMMAND cargo build --release --manifest-path ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml --target-dir ${CMAKE_CURRENT_BINARY_DIR}
  COMMENT "Building Rust library"
)

set_target_properties(rmw_zenoh_rs PROPERTIES
  LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/release"
  OUTPUT_NAME "rmw_zenoh_rs"
)

configure_rmw_library(rmw_zenoh_rs)

ament_export_targets(export_rmw_zenoh_rs)

register_rmw_implementation(
  "c:rosidl_typesupport_c:rosidl_typesupport_fastrtps_c:rosidl_typesupport_introspection_c"
  "cpp:rosidl_typesupport_cpp:rosidl_typesupport_fastrtps_cpp:rosidl_typesupport_introspection_cpp")

install(
  TARGETS rmw_zenoh_rs
  EXPORT export_rmw_zenoh_rs
  ARCHIVE DESTINATION lib
  LIBRARY DESTINATION lib
  RUNTIME DESTINATION bin
)

install(
DIRECTORY config
DESTINATION share/${PROJECT_NAME}
)

ament_package()
