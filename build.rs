//! Build script: embeds the app icon (assets/icon.ico) and version information in the Windows
//! executable, so Explorer, shortcuts, the taskbar and the Apps list show the right icon.
//! (The window icon set at run time in main.rs only covers the open window.)

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("ProductName", "Git Simple GUI")
            .set("FileDescription", "Git Simple GUI")
            .set(
                "LegalCopyright",
                "Copyright (C) 2026 Pierre Bocquillon - PolyForm Noncommercial 1.0.0",
            );
        if let Err(e) = res.compile() {
            // Without the Windows resource compiler the app still builds, only without its icon.
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}
