cmake_minimum_required(VERSION 3.5)
project(my_satacc_project)

find_package(Catch2 REQUIRED)
find_package(fmt REQUIRED)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_FLAGS_RELEASE "-g -O3 -DNDEBUG")
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")

add_subdirectory(sjqutils)
add_subdirectory(mem_req_interface)
add_subdirectory(minisat_utils)
add_subdirectory(sjqcache)
add_subdirectory(DRAMsim3)
add_subdirectory(new_intersim)
add_subdirectory(gpgpu_intersim)
add_subdirectory(ramulator)
add_subdirectory(satacc)
add_subdirectory(minisat)
