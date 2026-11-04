## Download

| System | File | Needs |
|---|---|---|
| Windows 10/11 (x86_64) | `Watershed-*-windows-x86_64.zip` | A GPU with Vulkan (any recent NVIDIA, AMD or Intel driver) |
| macOS 11+ (Apple Silicon) | `Watershed-*-macos-arm64.dmg` | |
| Linux (x86_64, glibc 2.35+) | `Watershed-*-linux-x86_64.AppImage` | Vulkan drivers, ALSA (or PipeWire's ALSA support) |

A GPU is required to separate songs. The model (700 MB) downloads on the first
separation.

## First start

These builds are not signed, so the system warns before the first start:

- **Windows:** unzip and run `watershed.exe`. If SmartScreen says "Windows protected
  your PC", choose **More info → Run anyway**.
- **macOS:** open the `.dmg` and drag Watershed to Applications. The first time, macOS
  says it cannot check the app: open **System Settings → Privacy & Security** and choose
  **Open Anyway** next to Watershed.
- **Linux:** make the file executable (`chmod +x Watershed-*.AppImage`) and run it.
  AppImages need FUSE 2 (`libfuse2`, called `libfuse2t64` on Ubuntu 24.04).
