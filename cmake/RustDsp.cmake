# Build the dependency-free Rust static library with the repository's pinned
# toolchain. Cargo outputs stay under the CMake build tree, including multi-
# config generators. macOS universal builds compile and combine both slices.
find_program(ELECTROBOW_CARGO cargo HINTS "$ENV{HOME}/.cargo/bin" "$ENV{USERPROFILE}/.cargo/bin" REQUIRED)
find_program(ELECTROBOW_RUSTC rustc HINTS "$ENV{HOME}/.cargo/bin" "$ENV{USERPROFILE}/.cargo/bin" REQUIRED)
execute_process(COMMAND "${ELECTROBOW_RUSTC}" -vV
    WORKING_DIRECTORY "${PROJECT_SOURCE_DIR}"
    OUTPUT_VARIABLE _rust_version COMMAND_ERROR_IS_FATAL ANY)
string(REGEX MATCH "host: ([^\n\r]+)" _rust_host_match "${_rust_version}")
set(_rust_host "${CMAKE_MATCH_1}")
if(NOT _rust_host)
    message(FATAL_ERROR "Could not determine Rust host target from rustc -vV")
endif()

set(_rust_targets "${_rust_host}")
if(APPLE AND CMAKE_OSX_ARCHITECTURES)
    set(_rust_targets "")
    foreach(_arch IN LISTS CMAKE_OSX_ARCHITECTURES)
        if(_arch STREQUAL "arm64")
            list(APPEND _rust_targets aarch64-apple-darwin)
        elseif(_arch STREQUAL "x86_64")
            list(APPEND _rust_targets x86_64-apple-darwin)
        else()
            message(FATAL_ERROR "Unsupported Rust macOS architecture: ${_arch}")
        endif()
    endforeach()
elseif(WIN32)
    if(NOT MSVC)
        message(FATAL_ERROR "ElectroBow's Rust Windows build requires the MSVC toolchain")
    endif()
    if(CMAKE_GENERATOR_PLATFORM STREQUAL "ARM64" OR CMAKE_CXX_COMPILER_ARCHITECTURE_ID STREQUAL "ARM64")
        set(_rust_targets aarch64-pc-windows-msvc)
    elseif(CMAKE_SIZEOF_VOID_P EQUAL 4)
        set(_rust_targets i686-pc-windows-msvc)
    else()
        set(_rust_targets x86_64-pc-windows-msvc)
    endif()
elseif(CMAKE_CROSSCOMPILING)
    message(FATAL_ERROR "Rust cross-compilation outside macOS/Windows is not configured")
endif()

set(_rust_profile "$<IF:$<CONFIG:Debug>,dev,release>")
set(_rust_output_profile "$<IF:$<CONFIG:Debug>,debug,release>")
set(_rust_target_dir "${PROJECT_BINARY_DIR}/rust")
set(_rust_archive_name "libelectrobow_dsp.a")
if(WIN32)
    set(_rust_archive_name "electrobow_dsp.lib")
endif()
file(GLOB_RECURSE _rust_sources CONFIGURE_DEPENDS "${PROJECT_SOURCE_DIR}/rust/dsp/src/*.rs")
set(_rust_inputs ${_rust_sources}
    "${PROJECT_SOURCE_DIR}/Cargo.toml"
    "${PROJECT_SOURCE_DIR}/Cargo.lock"
    "${PROJECT_SOURCE_DIR}/rust/dsp/Cargo.toml"
    "${PROJECT_SOURCE_DIR}/rust-toolchain.toml")
set(_rust_archives "")
set(_rust_environment "")
if(APPLE AND CMAKE_OSX_DEPLOYMENT_TARGET)
    list(APPEND _rust_environment "MACOSX_DEPLOYMENT_TARGET=${CMAKE_OSX_DEPLOYMENT_TARGET}")
endif()
foreach(_target IN LISTS _rust_targets)
    set(_archive "${_rust_target_dir}/${_target}/${_rust_output_profile}/${_rust_archive_name}")
    add_custom_command(OUTPUT "${_archive}"
        COMMAND "${CMAKE_COMMAND}" -E env --unset=MAKEFLAGS --unset=MFLAGS ${_rust_environment}
            "${ELECTROBOW_CARGO}" build --manifest-path "${PROJECT_SOURCE_DIR}/Cargo.toml"
            --package electrobow-dsp --lib --locked --offline
            --profile "${_rust_profile}" --target "${_target}" --target-dir "${_rust_target_dir}"
        DEPENDS ${_rust_inputs}
        WORKING_DIRECTORY "${PROJECT_SOURCE_DIR}"
        COMMENT "Building Rust DSP for ${_target} (${_rust_output_profile})"
        VERBATIM)
    list(APPEND _rust_archives "${_archive}")
endforeach()
list(LENGTH _rust_archives _rust_archive_count)
if(_rust_archive_count GREATER 1)
    find_program(ELECTROBOW_LIPO lipo REQUIRED)
    set(_rust_archive "${_rust_target_dir}/universal/${_rust_output_profile}/${_rust_archive_name}")
    add_custom_command(OUTPUT "${_rust_archive}"
        COMMAND "${CMAKE_COMMAND}" -E make_directory "${_rust_target_dir}/universal/${_rust_output_profile}"
        COMMAND "${ELECTROBOW_LIPO}" -create ${_rust_archives} -output "${_rust_archive}"
        DEPENDS ${_rust_archives}
        COMMENT "Combining universal Rust DSP archive" VERBATIM)
else()
    list(GET _rust_archives 0 _rust_archive)
endif()
add_custom_target(ElectroBowRustBuild DEPENDS "${_rust_archive}")
add_library(ElectroBowRust INTERFACE)
add_dependencies(ElectroBowRust ElectroBowRustBuild)
target_include_directories(ElectroBowRust INTERFACE "${PROJECT_SOURCE_DIR}/rust/dsp/include")
target_link_libraries(ElectroBowRust INTERFACE "${_rust_archive}")
# Native libraries required by the pinned Rust std staticlib. macOS uses
# libSystem, already linked by the C++ driver. Recheck when upgrading Rust.
if(WIN32)
    target_link_libraries(ElectroBowRust INTERFACE kernel32 ntdll userenv ws2_32 dbghelp synchronization)
elseif(NOT APPLE)
    find_package(Threads REQUIRED)
    target_link_libraries(ElectroBowRust INTERFACE gcc_s util rt Threads::Threads m ${CMAKE_DL_LIBS})
    target_link_options(ElectroBowRust INTERFACE "LINKER:--exclude-libs,libelectrobow_dsp.a")
endif()

if(BUILD_TESTING)
    # Rust unit tests always run for the local host, even when the plugin is
    # universal. A separate target-dir avoids contending with library builds.
    add_test(NAME ElectroBowRustTests
        COMMAND "${ELECTROBOW_CARGO}" test --manifest-path "${PROJECT_SOURCE_DIR}/Cargo.toml"
            --package electrobow-dsp --locked --offline --profile "${_rust_profile}"
            --target "${_rust_host}" --target-dir "${PROJECT_BINARY_DIR}/rust-tests")
    set_tests_properties(ElectroBowRustTests PROPERTIES WORKING_DIRECTORY "${PROJECT_SOURCE_DIR}")
endif()
