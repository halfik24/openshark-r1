//! Apply the default config to the mouse (active stage = 800 DPI).

use openshark_driver::{Config, Mouse};

fn main() {
    let cfg = Config {
        active_dpi: 1, // stage 1 = 800 DPI
        ..Default::default()
    };

    let mouse = Mouse::open().expect("open");
    println!("opened (wired = {}), applying: rate={} dpis={:?} active={} ...",
        mouse.is_wired(), cfg.polling_rate, cfg.dpis, cfg.active_dpi);

    match mouse.apply_config(&cfg) {
        Ok(()) => println!("OK: config applied"),
        Err(e) => println!("FAILED: {e}"),
    }
}
