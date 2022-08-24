cmake_minimum_required(VERSION 3.19)

project(rust_tools_wrapper)
set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_FLAGS_RELEASE "-g -O3 -DNDEBUG")
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")

include(FetchContent)

FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.2.1 # Optionally specify a commit hash, version tag or branch here
)

FetchContent_MakeAvailable(Corrosion)
corrosion_import_crate(MANIFEST_PATH Cargo.toml)

add_library(rust_tools_wrapper ${CMAKE_CURRENT_SOURCE_DIR}/rust_tools_wrapper.cc)
target_link_libraries(rust_tools_wrapper PUBLIC rusttools)
target_include_directories(rust_tools_wrapper PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
