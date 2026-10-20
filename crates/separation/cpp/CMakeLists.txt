# Builds BSRoformer.cpp and ggml as static libraries plus a small C wrapper
# (wrapper.cpp) that the Rust crate links against. Driven by build.rs.

cmake_minimum_required(VERSION 3.21)
project(watershed_bsr LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(VENDOR_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../vendor")
set(BSR_DIR "${VENDOR_DIR}/BSRoformer.cpp")

# Put every static library in one folder so build.rs knows where to look.
foreach(config_suffix "" _RELEASE _DEBUG _RELWITHDEBINFO _MINSIZEREL)
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY${config_suffix} "${CMAKE_BINARY_DIR}/lib")
endforeach()

# ggml builds its Vulkan shader generator as an ExternalProject, by default under
# ggml/src/ggml-vulkan/vulkan-shaders-gen-prefix/src/vulkan-shaders-gen-build.
# Under cargo's deep OUT_DIR that passes Windows' 260-character path limit, which
# MSVC tools still enforce. EP_BASE (inherited by subdirectories) shortens it.
include(ExternalProject)
set_property(DIRECTORY PROPERTY EP_BASE "${CMAKE_BINARY_DIR}/ep")

add_subdirectory("${VENDOR_DIR}/ggml" ggml EXCLUDE_FROM_ALL)

# BSRoformer.cpp picks its ggml backend itself and never sets the CPU thread count
# (ggml then uses 4). Route both calls to watershed_bsr_init_backend() in
# wrapper.cpp by compiling a patched copy of model.cpp. The anchors are single
# lines so that CRLF checkouts match too.
set(MODEL_SRC "${BSR_DIR}/src/model.cpp")
set(MODEL_PATCHED "${CMAKE_CURRENT_BINARY_DIR}/patched/model.cpp")
file(READ "${MODEL_SRC}" model_text)
set(patched_text "${model_text}")
foreach(anchor
        "backend_ = ggml_backend_init_by_type(GGML_BACKEND_DEVICE_TYPE_CPU, nullptr);"
        "backend_ = ggml_backend_init_best();")
    string(FIND "${patched_text}" "${anchor}" anchor_pos)
    if(anchor_pos EQUAL -1)
        message(FATAL_ERROR "Patch anchor not found in ${MODEL_SRC}: ${anchor}\n"
                            "BSRoformer.cpp changed; update cpp/CMakeLists.txt.")
    endif()
    string(REPLACE "${anchor}" "backend_ = watershed_bsr_init_backend();" patched_text "${patched_text}")
endforeach()
set(patched_text "#include <ggml-backend.h>\nggml_backend_t watershed_bsr_init_backend();\n${patched_text}")
file(WRITE "${MODEL_PATCHED}.tmp" "${patched_text}")
# Only touches the copy when its content changes, so rebuilds stay incremental.
configure_file("${MODEL_PATCHED}.tmp" "${MODEL_PATCHED}" COPYONLY)
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${MODEL_SRC}")

add_library(watershed_bsr STATIC
    wrapper.cpp
    "${MODEL_PATCHED}"
    "${BSR_DIR}/src/utils.cpp"
    "${BSR_DIR}/src/inference.cpp"
)
target_include_directories(watershed_bsr PRIVATE
    "${BSR_DIR}/include"
    "${BSR_DIR}/src"
    "${CMAKE_CURRENT_SOURCE_DIR}"
)
target_link_libraries(watershed_bsr PRIVATE ggml)

if(MSVC)
    target_compile_options(watershed_bsr PRIVATE /W3 /utf-8)
endif()

# BSRoformer.cpp parallelizes its STFT with OpenMP when available. Apple Clang has
# none, which only makes that part single-threaded.
find_package(OpenMP COMPONENTS CXX)
if(OpenMP_CXX_FOUND)
    target_link_libraries(watershed_bsr PRIVATE OpenMP::OpenMP_CXX)
endif()
