cmake_minimum_required(VERSION 3.14)
project(mbn_c VERSION 1.0 LANGUAGES CXX)

# Path to vcpkg toolchain
set(CMAKE_TOOLCHAIN_FILE "${CMAKE_SOURCE_DIR}/vcpkg_installed/scripts/buildsystems/vcpkg.cmake")

# Use modern C++ standards
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED True)
set(CMAKE_OSX_DEPLOYMENT_TARGET "13.3")


enable_testing()

# Find Google Test
find_package(GTest CONFIG REQUIRED)

# Path to the cbindgen-generated header
set(CBINDGEN_HEADER_DIR "${CMAKE_SOURCE_DIR}/../target")

# Rust binary
set(RUST_PROJECT_DIR "${CMAKE_SOURCE_DIR}/..")
set(RUST_LIB_DIR "${RUST_PROJECT_DIR}/target/debug")  # or target/release for release builds
set(RUST_LIB "${RUST_LIB_DIR}/libmbn_c.a")

# Add a custom command to build Rust library using Cargo
add_custom_command(
    OUTPUT "${RUST_LIB}"
    COMMAND cargo build --manifest-path "${RUST_PROJECT_DIR}/Cargo.toml"
    WORKING_DIRECTORY "${RUST_PROJECT_DIR}"
    COMMENT "Building Rust library with Cargo"
    VERBATIM
)

# Add a custom target to ensure Rust builds before the C++ tests
add_custom_target(build_rust ALL DEPENDS "${RUST_LIB}")


# Test executable
add_executable(MbnTests 
    tests/test_decode_c.cpp # Add your test files here
    tests/test_encode_c.cpp
)

# Include the cbindgen header directory
target_include_directories(MbnTests PRIVATE 
    "${CBINDGEN_HEADER_DIR}" # Include the directory containing mbn.h
    "${RUST_LIB_DIR}"
)

# Link the Rust library
target_link_libraries(MbnTests PRIVATE "${RUST_LIB}")

# Link Google Test to the tests
target_link_libraries(MbnTests PRIVATE 
    GTest::gtest 
    GTest::gtest_main 
)

# Add tests to CTest
add_test(NAME MbnTests COMMAND MbnTests)

# Link macOS frameworks (CoreFoundation and Security)
if (APPLE)
    target_link_libraries(MbnTests PRIVATE 
        "-framework CoreFoundation"
        "-framework Security"
        "-framework SystemConfiguration"
    )
endif()
