/* C interface to BSRoformer.cpp for the Rust crate (src/bsroformer.rs mirrors it).
 * No C++ exception crosses these functions: errors come back as status codes and
 * a message copied into a caller-provided buffer. */

#ifndef WATERSHED_BSR_WRAPPER_H
#define WATERSHED_BSR_WRAPPER_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct WsBsrModel WsBsrModel;

enum {
    WS_BSR_OK = 0,
    WS_BSR_ERROR = 1,
    WS_BSR_CANCELLED = 2,
};

typedef struct WsBsrInfo {
    int sample_rate;
    int num_stems;
    /* Model defaults stored in the GGUF. */
    int chunk_size;
    int num_overlap;
} WsBsrInfo;

/* Called from a worker thread with the fraction done (0..1). */
typedef void (*WsBsrProgressFn)(void *user, float fraction);
/* Polled from the calling thread; nonzero cancels. */
typedef int (*WsBsrCancelFn)(void *user);

/* Loads a GGUF model. force_cpu skips GPU backends; n_threads (> 0) sets the CPU
 * backend's thread count. Returns NULL on error. */
WsBsrModel *ws_bsr_load(const char *model_path_utf8, int force_cpu, int n_threads,
                        char *err, size_t err_len);
void ws_bsr_free(WsBsrModel *model);

void ws_bsr_info(const WsBsrModel *model, WsBsrInfo *out);
/* ggml backend name (e.g. "Vulkan0", "MTL0", "CPU") and device description.
 * Valid while the model lives. */
const char *ws_bsr_backend_name(const WsBsrModel *model);
const char *ws_bsr_device_description(const WsBsrModel *model);

/* Separates interleaved stereo input (num_frames * 2 samples at the model's
 * sample rate). outputs holds num_stems pointers, each to num_frames * 2 floats.
 * chunk_size / num_overlap <= 0 use the model defaults. */
int ws_bsr_process(WsBsrModel *model, const float *input, size_t num_frames,
                   int chunk_size, int num_overlap, float *const *outputs,
                   WsBsrProgressFn progress, WsBsrCancelFn cancel, void *user,
                   char *err, size_t err_len);

#ifdef __cplusplus
}
#endif

#endif
