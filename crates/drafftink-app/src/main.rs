//! Main application entry point (native).

#[cfg(feature = "native")]
fn main() {
    env_logger::init();
    log_panics_to_file();
    log::info!("Starting DrafftInk");

    let (server, room) = parse_args();
    pollster::block_on(drafftink_app::App::run(server, room));
}

/// Also write panics to `~/.cache/drafftink/crash.log`: when started from an
/// app launcher, stderr usually goes nowhere.
#[cfg(feature = "native")]
fn log_panics_to_file() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = dirs::cache_dir().map(|d| d.join("drafftink")) {
            let backtrace = std::backtrace::Backtrace::force_capture();
            let report = format!("[{:?}] {info}\n{backtrace}\n", std::time::SystemTime::now());
            let _ = std::fs::create_dir_all(&dir).and_then(|()| {
                use std::io::Write;
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("crash.log"))?
                    .write_all(report.as_bytes())
            });
        }
        default_hook(info);
    }));
}

/// Parse `--server <url>` and `--room <id>` from the command line so the
/// desktop app can auto-connect and auto-join a collaboration room on startup.
#[cfg(feature = "native")]
fn parse_args() -> (Option<String>, Option<String>) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut server = None;
    let mut room = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--server" if i + 1 < args.len() => {
                server = Some(args[i + 1].clone());
                i += 2;
            }
            "--room" if i + 1 < args.len() => {
                room = Some(args[i + 1].clone());
                i += 2;
            }
            _ => i += 1,
        }
    }
    (server, room)
}

#[cfg(not(feature = "native"))]
fn main() {
    panic!("Native feature not enabled. Use `cargo run --features native`");
}
