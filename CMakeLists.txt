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

add_custom_command(
  OUTPUT ${GENERATED_LIB_FILE} ${GENERATED_LIB_FILE}_dummy
  COMMAND cargo build --release --manifest-path ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml --target-dir ${CMAKE_CURRENT_BINARY_DIR}
  WORKING_DIRECTORY .
  BYPRODUCTS ${GENERATED_LIB_FILE}
  COMMENT "Building Rust library"
)

add_custom_target(rmw_zenoh_rs ALL DEPENDS ${GENERATED_LIB_FILE})

configure_rmw_library(rmw_zenoh_rs)

# ament_export_targets(export_rmw_zenoh_rs)

register_rmw_implementation(
  "c:rosidl_typesupport_c:rosidl_typesupport_fastrtps_c:rosidl_typesupport_introspection_c"
  "cpp:rosidl_typesupport_cpp:rosidl_typesupport_fastrtps_cpp:rosidl_typesupport_introspection_cpp")

install(
FILES ${GENERATED_LIB_FILE}
# EXPORT export_rmw_zenoh_rs
DESTINATION lib
)

install(
DIRECTORY config
DESTINATION share/${PROJECT_NAME}
)

ament_package()
