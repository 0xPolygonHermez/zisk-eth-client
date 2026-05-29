# Host-target build orchestrator for zilkworm's prover/host_lib (which exposes
# z6m_run for native FFI). Driven by ../build.rs via the `cmake` crate.
#
# All C++ source lives in zilkworm's submodule (third_party/zilkworm/...).
# This file is just glue: it bootstraps Cable, brings in third_party + zilk_core
# (transitive deps of host_lib), and then adds prover/host_lib which provides
# the `z6m_host` static library target.

cmake_minimum_required(VERSION 3.28)

include(${Z6M_ROOT}/third_party/evmone/cmake/cable/bootstrap.cmake)
include(CableBuildType)
cable_set_build_type(DEFAULT Release CONFIGURATION_TYPES Release)

project(zilkworm_host_lib LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(SILKWORM_MAIN_DIR ${Z6M_ROOT})
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

include(CableBuildInfo)
cable_add_buildinfo_library(PROJECT_NAME zilkworm GIT_DESCRIBE "" GIT_BRANCH "")

add_subdirectory(${Z6M_ROOT}/third_party     ${CMAKE_BINARY_DIR}/third_party)
add_subdirectory(${Z6M_ROOT}/zilk_core       ${CMAKE_BINARY_DIR}/zilk_core)
add_subdirectory(${Z6M_ROOT}/prover/host_lib ${CMAKE_BINARY_DIR}/host_lib)
