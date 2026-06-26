set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} ${CMAKE_CURRENT_LIST_DIR}/cmake)

find_package(Rust)

set(LIBRARY_NAME "${CMAKE_STATIC_LIBRARY_PREFIX}ddnet_corpus_runner${CMAKE_STATIC_LIBRARY_SUFFIX}")
set(CARGO_BUILD ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${PROJECT_BINARY_DIR} ${RUST_CARGO} build --locked --manifest-path "${CMAKE_CURRENT_LIST_DIR}/../Cargo.toml")

if(CMAKE_BUILD_TYPE STREQUAL Debug)
    set(CARGO_BUILD_DIR "debug")
else()
    set(CARGO_BUILD_DIR "release")
    list(APPEND CARGO_BUILD --release)
endif()

add_library(corpus_runner_c STATIC IMPORTED GLOBAL)
set_target_properties(corpus_runner_c PROPERTIES
    IMPORTED_LOCATION "${PROJECT_BINARY_DIR}/${CARGO_BUILD_DIR}/${LIBRARY_NAME}"
)

add_custom_target(corpus_runner_target DEPENDS "${PROJECT_BINARY_DIR}/${CARGO_BUILD_DIR}/${LIBRARY_NAME}")
add_custom_command(
    OUTPUT "${PROJECT_BINARY_DIR}/${CARGO_BUILD_DIR}/${LIBRARY_NAME}"
    COMMAND ${CARGO_BUILD}
    WORKING_DIRECTORY "${PROJECT_BINARY_DIR}"
    USES_TERMINAL
)

add_custom_target(corpus_runner_header_target DEPENDS "${PROJECT_BINARY_DIR}/include/ddcorpus/runner.h")
add_custom_command(
    OUTPUT "${PROJECT_BINARY_DIR}/include/ddcorpus/runner.h"
    COMMAND cbindgen --crate ddnet_corpus_runner --lang c --output ${PROJECT_BINARY_DIR}/include/ddcorpus/runner.h
    WORKING_DIRECTORY "${PROJECT_BINARY_DIR}/corpus_runner"
    USES_TERMINAL
)

add_library(corpus_runner INTERFACE)
add_dependencies(corpus_runner
    corpus_runner_target
    corpus_runner_header_target
)
target_link_libraries(corpus_runner INTERFACE
    corpus_runner_c
)
target_include_directories(corpus_runner INTERFACE
    "${PROJECT_BINARY_DIR}/include"
    "${CMAKE_CURRENT_LIST_DIR}/include"
)
