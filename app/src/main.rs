//! Runs the app on the desktop backend (`cargo run`).
//! iOS packages use `nsh run` / `nsh package`; this binary is a macOS host only.

#[cfg(target_os = "macos")]
fn main() -> impl std::process::Termination {
    use nsh_platform_macos as platform;
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<grkit_app::App>()),
        platform::AppOptions {
            title: "GRKit".to_owned(),
            size: (390, 844),
            ..Default::default()
        },
    )
}

#[cfg(not(target_os = "macos"))]
fn main() {
    // Device / simulator packages are built by `nsh package`, not this binary.
}
