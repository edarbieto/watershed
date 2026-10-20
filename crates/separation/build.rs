//! Builds BSRoformer.cpp + ggml (git submodules in `vendor/`) and the C wrapper in
//! `cpp/` with CMake, then links them statically.
//!
//! GPU backend: Metal on macOS, Vulkan on Windows and Linux (needs the Vulkan SDK,
//! or the distro's Vulkan headers, `glslc` and SPIRV-Headers on Linux).

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest_dir.join("vendor");
    for submodule in ["BSRoformer.cpp", "ggml"] {
        if !vendor.join(submodule).join("CMakeLists.txt").exists() {
            panic!(
                "vendor/{submodule} is missing: run `git submodule update --init` \
                 in the repository root"
            );
        }
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let vulkan = matches!(target_os.as_str(), "windows" | "linux");
    let metal = target_os == "macos";

    if vulkan && target_os == "windows" && env::var_os("VULKAN_SDK").is_none() {
        panic!(
            "VULKAN_SDK is not set: install the Vulkan SDK from https://vulkan.lunarg.com \
             and open a new terminal"
        );
    }

    let mut config = cmake::Config::new(manifest_dir.join("cpp"));
    config
        // The model is unusably slow unoptimized, so ggml is always a release build
        // (this also keeps MSVC on the release C runtime, like Rust).
        .profile("Release")
        .build_target("watershed_bsr")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreadedDLL")
        // Portable binaries: never tune for the build machine's CPU.
        .define("GGML_NATIVE", "OFF")
        .define("GGML_BUILD_TESTS", "OFF")
        .define("GGML_BUILD_EXAMPLES", "OFF")
        .define("GGML_VULKAN", if vulkan { "ON" } else { "OFF" })
        .define("GGML_METAL", if metal { "ON" } else { "OFF" })
        .define("GGML_METAL_EMBED_LIBRARY", if metal { "ON" } else { "OFF" })
        // Apple Clang has no OpenMP; ggml then uses its own thread pool.
        .define("GGML_OPENMP", if metal { "OFF" } else { "ON" });
    // Otherwise the cmake crate fills CMAKE_<LANG>_FLAGS_RELEASE with its own
    // flags, which carry no optimization level: these are CMake's defaults.
    let release_flags = if env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc" {
        "/O2 /Ob2 /DNDEBUG"
    } else {
        "-O3 -DNDEBUG"
    };
    for lang in ["C", "CXX"] {
        config.define(format!("CMAKE_{lang}_FLAGS_RELEASE"), release_flags);
    }
    if target_os == "windows" {
        // MSBuild otherwise leaves worker processes running that keep files in
        // target/ locked after the build.
        config.env("MSBUILDDISABLENODEREUSE", "1");
    }
    if target_arch == "x86_64" {
        // With GGML_NATIVE off ggml falls back to SSE2. AVX2-era CPUs (2013+) are
        // a safe baseline and keep the CPU fallback usable.
        for feature in ["SSE42", "AVX", "AVX2", "BMI2", "FMA", "F16C"] {
            config.define(format!("GGML_{feature}"), "ON");
        }
    }
    let out = config.build();

    let lib_dir = out.join("build").join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // Dependents before their dependencies, for linkers that care about order.
    let mut libs = vec!["watershed_bsr", "ggml"];
    if vulkan {
        libs.push("ggml-vulkan");
    }
    if metal {
        libs.extend(["ggml-metal", "ggml-blas"]);
    }
    libs.extend(["ggml-cpu", "ggml-base"]);
    for lib in libs {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    match target_os.as_str() {
        "windows" => {
            let sdk = PathBuf::from(env::var_os("VULKAN_SDK").unwrap());
            println!(
                "cargo:rustc-link-search=native={}",
                sdk.join("Lib").display()
            );
            println!("cargo:rustc-link-lib=vulkan-1");
            // MSVC's C++ runtime and OpenMP (vcomp) come in through /DEFAULTLIB
            // directives in the object files.
        }
        "linux" => {
            for lib in ["vulkan", "gomp", "stdc++"] {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
        "macos" => {
            println!("cargo:rustc-link-lib=c++");
            for framework in ["Foundation", "Metal", "MetalKit", "Accelerate"] {
                println!("cargo:rustc-link-lib=framework={framework}");
            }
        }
        other => panic!("unsupported target OS for BSRoformer.cpp: {other}"),
    }

    rerun_if_changed(&manifest_dir.join("cpp"));
    for dir in ["src", "include", "CMakeLists.txt"] {
        rerun_if_changed(&vendor.join("BSRoformer.cpp").join(dir));
    }
    for dir in ["src", "include", "cmake", "CMakeLists.txt"] {
        rerun_if_changed(&vendor.join("ggml").join(dir));
    }
    println!("cargo:rerun-if-env-changed=VULKAN_SDK");
}

fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}
