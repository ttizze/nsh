//! Native checks need AppKit on the process main thread and a desktop session.
#[cfg(target_os = "macos")]
mod ime;
fn main() {
    if !std::env::args().any(|arg| arg == "--ignored") {
        println!("Native AppKit checks skipped; run with --ignored in a desktop session.");
    } else {
        #[cfg(target_os = "macos")]
        {
            use nsh_platform_macos::{
                objc2::MainThreadMarker,
                objc2_app_kit::{NSApplication, NSApplicationActivationPolicy},
            };
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            app.finishLaunching();
            ime::run();
        }
    }
}
