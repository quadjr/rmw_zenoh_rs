extern crate bindgen;

use std::path::PathBuf;

fn main() {
    let prefix_path = "/opt/ros/humble";
    let include_path = prefix_path.to_string() + "/include";
    let bindgen_out_path = PathBuf::from("src");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("binding.hpp")
        .clang_arg(format!("-I{include_path}/rmw/"))
        .clang_arg(format!("-I{include_path}/rcutils/"))
        .clang_arg(format!("-I{include_path}/rosidl_runtime_c/"))
        .clang_arg(format!("-I{include_path}/rosidl_typesupport_interface/"))
        .allowlist_function("rs_.*")
        .allowlist_function("rcutils_.*")
        .allowlist_function("rmw_topic_endpoint_info_.*")
        .allowlist_function("rmw_get_zero_initialized_.*")
        .allowlist_function("rmw_names_and_types_init")
        .allowlist_function("rmw_check_zero_rmw_string_array")
        .allowlist_function("rmw_security_options_copy")
        .allowlist_function("rmw_security_options_fini")
        .allowlist_function("rmw_validate_node_name")
        .allowlist_function("rmw_validate_namespace")
        .allowlist_function("rmw_validate_full_topic_name")
        .allowlist_function("rmw_event_fini")
        .allowlist_function("rmw_names_and_types_check_zero")
        .allowlist_type("rmw_.*")
        .allowlist_type("rcutils_.*")
        .allowlist_type("rosidl_.*")
        .allowlist_var("RMW_.*")
        .derive_default(true)
        .generate()
        .expect("Unable to generate bindings");
    bindings
        .write_to_file(bindgen_out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Compile cpp
    cc::Build::new()
        .cpp(true)
        .warnings(true)
        .flag("-std=c++17")
        .include(format!("{include_path}/rmw/"))
        .include(format!("{include_path}/rcutils/"))
        .include(format!("{include_path}/fastcdr"))
        .include(format!("{include_path}/rosidl_runtime_c"))
        .include(format!("{include_path}/rosidl_typesupport_fastrtps_c"))
        .include(format!("{include_path}/rosidl_typesupport_fastrtps_cpp"))
        .include(format!("{include_path}/rosidl_typesupport_interface"))
        .file("cpp/type_support.cpp")
        .compile("type_support");

    // Link libraries
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-search=native={prefix_path}/lib/");
    println!("cargo:rustc-link-lib=dylib=rmw");
    println!("cargo:rustc-link-lib=dylib=rcutils");
    println!("cargo:rustc-link-lib=dylib=fastcdr");
    println!("cargo:rustc-link-lib=dylib=rosidl_runtime_c");
    println!("cargo:rustc-link-lib=dylib=rosidl_typesupport_fastrtps_c");
    println!("cargo:rustc-link-lib=dylib=rosidl_typesupport_fastrtps_cpp");
}
