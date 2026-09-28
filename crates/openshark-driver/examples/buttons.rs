//! Restore the factory button mapping (fixes buttons remapped to nothing).

use openshark_driver::Mouse;

fn main() {
    let mouse = Mouse::open().expect("open");
    println!("opened (wired = {})", mouse.is_wired());

    match mouse.reset_buttons() {
        Ok(()) => println!("OK: factory button map written (ACK received)"),
        Err(e) => {
            eprintln!("FAILED: {e}");
            eprintln!("hint: the mouse must be awake — move it first");
            std::process::exit(1);
        }
    }
}
