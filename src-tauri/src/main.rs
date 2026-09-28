// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // NVIDIA + Wayland: WebKitGTK attaches a shared-memory buffer while the
    // driver already armed explicit sync, and the compositor kills the client
    // with "Error 71 (Protocol error)". Disabling explicit sync keeps hardware
    // acceleration and avoids the crash (see tauri-apps/tauri#10702).
    // Must happen before WebKit creates its first GL context; it is also
    // inherited by the WebKitWebProcess children.
    #[cfg(all(target_os = "linux", not(target_env = "musl")))]
    if std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none()
        && std::path::Path::new("/proc/driver/nvidia/version").exists()
    {
        // SAFETY: called once at process start, before any threads exist.
        unsafe { std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1") };
    }

    openshark_r1_lib::run()
}
