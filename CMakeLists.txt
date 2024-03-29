cmake_minimum_required(VERSION 3.19)

project(my_satacc_project)

set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_FLAGS_RELEASE "-g -O3 -DNDEBUG")
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")


include(FetchContent)

FetchContent_Declare(
    fmt
    GIT_REPOSITORY https://github.com/fmtlib/fmt
)
FetchContent_Declare(
    catch2
    GIT_REPOSITORY https://github.com/catchorg/Catch2
    GIT_TAG v3.1.0 # Optionally specify a commit hash, version tag or branch here

)

FetchContent_MakeAvailable(fmt)
FetchContent_MakeAvailable(catch2)
add_subdirectory(rusttools)
add_subdirectory(sjqutils)
add_subdirectory(mem_req_interface)
add_subdirectory(minisat_utils)
# add_subdirectory(sjqcache)
# add_subdirectory(DRAMsim3)
# add_subdirectory(new_intersim)
# add_subdirectory(gpgpu_intersim)
# add_subdirectory(ramulator)
# add_subdirectory(satacc)
add_subdirectory(minisat)
