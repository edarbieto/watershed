#include "wrapper.h"

#include <bs_roformer/inference.h>
#include <ggml-backend.h>

#include <algorithm>
#include <cstring>
#include <exception>
#include <memory>
#include <string>
#include <vector>

struct WsBsrModel {
    std::unique_ptr<Inference> inference;
    std::string backend_name;
    std::string device_description;
};

namespace {

// Settings for the backend created during the current ws_bsr_load call, read by
// watershed_bsr_init_backend() on the same thread.
struct BackendRequest {
    bool force_cpu = false;
    int n_threads = 0;
    std::string backend_name;
    std::string device_description;
};
thread_local BackendRequest* g_request = nullptr;

const char kCancelledMessage[] = "Inference cancelled";

void CopyError(char* err, size_t err_len, const char* message) {
    if (!err || err_len == 0) {
        return;
    }
    size_t n = std::min(std::strlen(message), err_len - 1);
    std::memcpy(err, message, n);
    err[n] = '\0';
}

}  // namespace

// Called by the patched BSRoformer::Initialize (see CMakeLists.txt).
ggml_backend_t watershed_bsr_init_backend() {
    bool force_cpu = g_request && g_request->force_cpu;
    ggml_backend_t backend = force_cpu
        ? ggml_backend_init_by_type(GGML_BACKEND_DEVICE_TYPE_CPU, nullptr)
        : ggml_backend_init_best();
    if (!backend) {
        return nullptr;
    }

    ggml_backend_dev_t device = ggml_backend_get_device(backend);
    if (g_request && g_request->n_threads > 0) {
        // Only the CPU backend exposes this; GPU backends return null.
        ggml_backend_reg_t reg = ggml_backend_dev_backend_reg(device);
        auto set_threads = reinterpret_cast<ggml_backend_set_n_threads_t>(
            ggml_backend_reg_get_proc_address(reg, "ggml_backend_set_n_threads"));
        if (set_threads) {
            set_threads(backend, g_request->n_threads);
        }
    }
    if (g_request) {
        g_request->backend_name = ggml_backend_name(backend);
        g_request->device_description = ggml_backend_dev_description(device);
    }
    return backend;
}

extern "C" {

WsBsrModel* ws_bsr_load(const char* model_path_utf8, int force_cpu, int n_threads,
                        char* err, size_t err_len) {
    BackendRequest request;
    request.force_cpu = force_cpu != 0;
    request.n_threads = n_threads;
    g_request = &request;
    try {
        auto model = std::make_unique<WsBsrModel>();
        model->inference = std::make_unique<Inference>(model_path_utf8);
        model->backend_name = std::move(request.backend_name);
        model->device_description = std::move(request.device_description);
        g_request = nullptr;
        return model.release();
    } catch (const std::exception& e) {
        CopyError(err, err_len, e.what());
    } catch (...) {
        CopyError(err, err_len, "unknown C++ exception");
    }
    g_request = nullptr;
    return nullptr;
}

void ws_bsr_free(WsBsrModel* model) {
    delete model;
}

void ws_bsr_info(const WsBsrModel* model, WsBsrInfo* out) {
    const Inference& inference = *model->inference;
    out->sample_rate = inference.GetSampleRate();
    out->num_stems = inference.GetNumStems();
    out->chunk_size = inference.GetDefaultChunkSize();
    out->num_overlap = inference.GetDefaultNumOverlap();
}

const char* ws_bsr_backend_name(const WsBsrModel* model) {
    return model->backend_name.c_str();
}

const char* ws_bsr_device_description(const WsBsrModel* model) {
    return model->device_description.c_str();
}

int ws_bsr_process(WsBsrModel* model, const float* input, size_t num_frames,
                   int chunk_size, int num_overlap, float* const* outputs,
                   WsBsrProgressFn progress, WsBsrCancelFn cancel, void* user,
                   char* err, size_t err_len) {
    bool cancelled = false;
    try {
        Inference& inference = *model->inference;
        if (chunk_size <= 0) {
            chunk_size = inference.GetDefaultChunkSize();
        }
        if (num_overlap <= 0) {
            num_overlap = inference.GetDefaultNumOverlap();
        }

        std::vector<float> audio(input, input + num_frames * 2);
        std::function<void(float)> on_progress;
        if (progress) {
            on_progress = [progress, user](float fraction) { progress(user, fraction); };
        }
        Inference::CancelCallback on_cancel;
        if (cancel) {
            on_cancel = [cancel, user, &cancelled]() {
                if (cancel(user) != 0) {
                    cancelled = true;
                }
                return cancelled;
            };
        }

        std::vector<std::vector<float>> stems =
            inference.Process(audio, chunk_size, num_overlap, on_progress, on_cancel);
        std::vector<float>().swap(audio);

        size_t expected = num_frames * 2;
        if (stems.size() != static_cast<size_t>(inference.GetNumStems())) {
            CopyError(err, err_len, "model returned an unexpected number of stems");
            return WS_BSR_ERROR;
        }
        for (size_t i = 0; i < stems.size(); ++i) {
            if (stems[i].size() != expected) {
                CopyError(err, err_len, "model returned a stem of unexpected length");
                return WS_BSR_ERROR;
            }
            std::memcpy(outputs[i], stems[i].data(), expected * sizeof(float));
            // Free each stem as soon as it is copied to keep the peak memory down.
            std::vector<float>().swap(stems[i]);
        }
        return WS_BSR_OK;
    } catch (const std::exception& e) {
        if (cancelled || std::strcmp(e.what(), kCancelledMessage) == 0) {
            return WS_BSR_CANCELLED;
        }
        CopyError(err, err_len, e.what());
    } catch (...) {
        CopyError(err, err_len, "unknown C++ exception");
    }
    return WS_BSR_ERROR;
}

}  // extern "C"
