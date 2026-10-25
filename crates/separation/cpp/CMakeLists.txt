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

# The submodules are not modified: CMake compiles patched copies of some of their
# sources. Anchors are single lines so that CRLF checkouts match too.

# Replaces every `anchor` in the variable named `text_var`, read from `src`.
function(patch_anchor text_var src anchor replacement)
    string(FIND "${${text_var}}" "${anchor}" anchor_pos)
    if(anchor_pos EQUAL -1)
        message(FATAL_ERROR "Patch anchor not found in ${src}: ${anchor}\n"
                            "The submodule changed; update cpp/CMakeLists.txt.")
    endif()
    string(REPLACE "${anchor}" "${replacement}" patched "${${text_var}}")
    set(${text_var} "${patched}" PARENT_SCOPE)
endfunction()

# Writes `text` to `out`, touching it only when its content changes so rebuilds
# stay incremental, and reconfigures when `src` changes.
function(write_patched text out src)
    file(WRITE "${out}.tmp" "${text}")
    configure_file("${out}.tmp" "${out}" COPYONLY)
    set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${src}")
endfunction()

# model.cpp picks its ggml backend itself and never sets the CPU thread count
# (ggml then uses 4). Route both calls to watershed_bsr_init_backend() in
# wrapper.cpp.
set(MODEL_SRC "${BSR_DIR}/src/model.cpp")
set(MODEL_PATCHED "${CMAKE_CURRENT_BINARY_DIR}/patched/model.cpp")
file(READ "${MODEL_SRC}" model_text)
foreach(anchor
        "backend_ = ggml_backend_init_by_type(GGML_BACKEND_DEVICE_TYPE_CPU, nullptr);"
        "backend_ = ggml_backend_init_best();")
    patch_anchor(model_text "${MODEL_SRC}" "${anchor}" "backend_ = watershed_bsr_init_backend();")
endforeach()
# The flash attention over frequency bands reads K and V as views whose rows are
# 4096 bytes apart for 256 bytes of data. Metal pads the KV tail in a scratch
# buffer with that same layout: ~3.4 GB of the compute buffer. Contiguous copies
# (the same values; ~1 ms per layer on the GPU) shrink it to ~0.3 GB. Other
# backends only pay the copies.
set(anchor "ggml_tensor* f_attn_out_fa = ggml_flash_attn_ext(ctx, fQ_fa, fK_fa, fV_fa, nullptr, scale, 0.0f, 0.0f);")
patch_anchor(model_text "${MODEL_SRC}" "${anchor}"
             "fK_fa = ggml_cont(ctx, fK_fa); fV_fa = ggml_cont(ctx, fV_fa); ${anchor}")
write_patched("#include <ggml-backend.h>\nggml_backend_t watershed_bsr_init_backend();\n${model_text}"
              "${MODEL_PATCHED}" "${MODEL_SRC}")

# inference.cpp patches, all to lower the memory of a separation without changing
# its output or speed.
set(INFERENCE_SRC "${BSR_DIR}/src/inference.cpp")
set(INFERENCE_PATCHED "${CMAKE_CURRENT_BINARY_DIR}/patched/inference.cpp")
file(READ "${INFERENCE_SRC}" inference_text)

# Chunks in flight hold ~45 MB each before inference and ~185 MB after (the masks
# of 6 stems). The GPU takes far longer per chunk than the CPU stages around it, so
# one chunk waiting in each queue keeps it as busy as three.
foreach(queue input_queue output_queue)
    set(anchor "ThreadSafeQueue<std::shared_ptr<ChunkState>> ${queue}(3);")
    patch_anchor(inference_text "${INFERENCE_SRC}" "${anchor}"
                 "ThreadSafeQueue<std::shared_ptr<ChunkState>> ${queue}(1);")
endforeach()

# Free each chunk's buffers as soon as they are used: the model input once it is
# on the device (the copy is synchronous), the masks and spectrogram once the
# audio is rebuilt from them.
set(anchor "ggml_backend_tensor_set(graph->input_tensor, state->stft_flattened.data(), 0, input_bytes);")
patch_anchor(inference_text "${INFERENCE_SRC}" "${anchor}"
             "${anchor} std::vector<float>().swap(state->stft_flattened);")
set(anchor "PostProcessAndISTFT(state->mask_output, state->stft_outputs, state->n_frames, state->final_audio, scratch);")
patch_anchor(inference_text "${INFERENCE_SRC}" "${anchor}"
             "${anchor} std::vector<float>().swap(state->mask_output); std::vector<std::vector<float>>().swap(state->stft_outputs);")

# The overlap-added stems are normalized and cropped into a second copy of all of
# them, the peak memory of a separation (6 stems twice). Do it in place and return
# early; the original code after the return is unreachable.
# The anchor's indentation stays in front of the replacement.
set(in_place [=[
// Watershed: in place. Each output sample is read at or after where it lands.
    for (auto& stem : result) {
        for (int k = 0; k < n_input_samples; ++k) {
            int padded_idx = (pad_l + k) * channels;
            float w0 = counter[padded_idx + 0];
            float w1 = counter[padded_idx + 1];
            if (w0 < 1e-4f) w0 = 1.0f;
            if (w1 < 1e-4f) w1 = 1.0f;
            stem[k * channels + 0] = stem[padded_idx + 0] / w0;
            stem[k * channels + 1] = stem[padded_idx + 1] / w1;
        }
        stem.resize(n_input_samples * channels);
    }
    return result;
]=])
set(anchor "std::vector<std::vector<float>> final_output_stems(num_stems);")
patch_anchor(inference_text "${INFERENCE_SRC}" "${anchor}" "${in_place}    ${anchor}")
write_patched("${inference_text}" "${INFERENCE_PATCHED}" "${INFERENCE_SRC}")

# ggml-metal reserves scratch space for every flash attention as if any kernel
# could run, so that a graph whose shapes change (llama.cpp's KV cache) never has
# to be reallocated. Ours keep their shapes, and none uses the vector kernel's
# temp buffer (~2.4 GB per chunk) or the dequantized KV copy. Reserve them only
# when used, like the conditions ggml left commented out there.
if(TARGET ggml-metal)
    set(METAL_OPS_SRC "${VENDOR_DIR}/ggml/src/ggml-metal/ggml-metal-ops.cpp")
    set(METAL_OPS_PATCHED "${CMAKE_CURRENT_BINARY_DIR}/patched/ggml-metal-ops.cpp")
    file(READ "${METAL_OPS_SRC}" metal_ops_text)
    set(anchor "const int64_t ne01_max = std::min(ne01, 32);")
    patch_anchor(metal_ops_text "${METAL_OPS_SRC}" "${anchor}"
                 "if (!ggml_metal_op_flash_attn_ext_use_vec(op)) { return 0; } ${anchor}")
    set(anchor "const size_t k_size = ggml_metal_op_flash_attn_ext_kv_f16_k_size(op);")
    patch_anchor(metal_ops_text "${METAL_OPS_SRC}" "${anchor}"
                 "if (!ggml_metal_op_flash_attn_ext_use_kv_f16(op)) { return 0; } ${anchor}")
    write_patched("${metal_ops_text}" "${METAL_OPS_PATCHED}" "${METAL_OPS_SRC}")
    get_target_property(metal_sources ggml-metal SOURCES)
    list(TRANSFORM metal_sources REPLACE "^ggml-metal-ops\\.cpp$" "${METAL_OPS_PATCHED}")
    if(NOT "${METAL_OPS_PATCHED}" IN_LIST metal_sources)
        message(FATAL_ERROR "ggml-metal-ops.cpp not found in the ggml-metal sources; "
                            "ggml changed, update cpp/CMakeLists.txt.")
    endif()
    set_property(TARGET ggml-metal PROPERTY SOURCES ${metal_sources})
    # The copy's own quoted includes live next to the original.
    target_include_directories(ggml-metal PRIVATE "${VENDOR_DIR}/ggml/src/ggml-metal")
endif()

add_library(watershed_bsr STATIC
    wrapper.cpp
    "${MODEL_PATCHED}"
    "${BSR_DIR}/src/utils.cpp"
    "${INFERENCE_PATCHED}"
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
