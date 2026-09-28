fn main() {
    let mouse = openshark_driver::Mouse::open().expect("open mouse");
    if mouse.is_wired() {
        println!("wired connection - no battery telemetry");
        return;
    }
    println!("battery: {}%", mouse.battery_percent().expect("read battery"));
}
