fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    // Windows: the icon Explorer and the taskbar show for the .exe, and its version
    // info (taken from Cargo.toml). The window icon is set at run time in `main.rs`.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico");
        // Task Manager shows the description; both default to the package name.
        res.set("ProductName", "Watershed");
        res.set("FileDescription", "Watershed");
        res.compile().expect("compiling the Windows resources");
    }
}
