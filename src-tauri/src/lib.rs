mod icons;

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

use openshark_driver::Mouse;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, RunEvent, Wry};

/// Battery telemetry is slow-moving; once a minute is plenty.
const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// User settings, persisted between runs.
const CONFIG_DIR: &str = ".config/openshark-r1";
const CONFIG_FILE: &str = "config.json";

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home)
        .join(CONFIG_DIR)
        .join(CONFIG_FILE)
}

/// Load the saved settings, falling back to the driver defaults.
fn load_config() -> openshark_driver::Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &openshark_driver::Config) -> Result<(), String> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {dir:?}: {e}"))?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("cannot write {path:?}: {e}"))
}

/// Человеческая причина отказа мыши — её видят и окно, и пункт трей.
fn describe(e: &openshark_driver::Error) -> String {
    use openshark_driver::rusb::Error as Usb;
    use openshark_driver::Error as E;
    match e {
        E::NotFound => "мышь не найдена — проверьте приёмник".into(),
        E::Usb(Usb::Busy) => "мышь занята другим окном".into(),
        E::Usb(Usb::Access) => "нет доступа к USB".into(),
        E::Usb(Usb::Timeout) => "мышь не отвечает — шевельните её".into(),
        E::Usb(Usb::NoDevice) => "устройство отключено".into(),
        E::Usb(other) => format!("ошибка USB: {other}"),
        other => other.to_string(),
    }
}

struct AppState {
    /// Lazily opened USB handle; `None` while the mouse is absent.
    mouse: Mutex<Option<Mouse>>,
    /// Wakes the polling thread for an immediate refresh.
    refresh_tx: Mutex<Sender<()>>,
    /// Menu row showing the charge percentage.
    battery_item: MenuItem<Wry>,
}

/// Outcome of one battery read.
enum Reading {
    Level(u8),
    Wired,
    Failed(String),
}

/// Settings currently on disk, plus whether they came from the file.
#[tauri::command]
fn get_config() -> (openshark_driver::Config, bool) {
    let on_disk = config_path().exists();
    (load_config(), on_disk)
}

/// Validate, persist and push the settings to the mouse.
#[tauri::command]
fn apply_config(
    state: tauri::State<'_, AppState>,
    config: openshark_driver::Config,
) -> Result<(), String> {
    config.validate()?;

    with_mouse(&state, |mouse| mouse.apply_config(&config).map_err(|e| describe(&e)))?;
    save_config(&config)?;
    Ok(())
}

/// Put every button back to its factory assignment.
#[tauri::command]
fn reset_buttons(state: tauri::State<'_, AppState>) -> Result<(), String> {
    with_mouse(&state, |mouse| mouse.reset_buttons().map_err(|e| describe(&e)))
}

/// Battery percentage, or `None` while running from the cable.
#[tauri::command]
fn battery(state: tauri::State<'_, AppState>) -> Result<Option<u8>, String> {
    with_mouse(&state, |mouse| {
        if mouse.is_wired() {
            Ok(None)
        } else {
            mouse.battery_percent().map(Some).map_err(|e| describe(&e))
        }
    })
}

/// Run `f` against the shared mouse handle, reopening it when needed.
fn with_mouse<T>(
    state: &tauri::State<'_, AppState>,
    f: impl FnOnce(&mut Mouse) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state.mouse.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(Mouse::open().map_err(|e| describe(&e))?);
    }
    let result = f(guard.as_mut().expect("just opened"));
    if result.is_err() {
        // Ошибка чтения/записи — хэндл, скорее всего, протух (приёмник
        // переподключали). Закрываем, чтобы следующий вызов открыл заново.
        *guard = None;
    }
    result
}

pub fn run() {
    tauri::Builder::default()
        // Второй запуск поднимает уже открытое окно вместо дубля, который
        // успевает перехватить usb-интерфейс у первого.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            // Closing the settings window only hides it; the tray keeps running.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            let battery_item =
                MenuItem::with_id(app, "battery_info", "Заряд: …", false, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Настройки…", true, None::<&str>)?;
            let refresh =
                MenuItem::with_id(app, "refresh", "Обновить сейчас", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&battery_item, &settings, &refresh, &quit])?;

            let (refresh_tx, refresh_rx) = std::sync::mpsc::channel();

            app.manage(AppState {
                mouse: Mutex::new(None),
                refresh_tx: Mutex::new(refresh_tx),
                battery_item: battery_item.clone(),
            });

            // Left icon: the mouse glyph.
            TrayIconBuilder::with_id("mouse")
                .icon(icons::mouse_icon())
                .tooltip("OpenShark R1")
                .menu(&menu)
                .on_menu_event(handle_menu_event)
                .build(app)?;

            // Right icon: live battery level.
            TrayIconBuilder::with_id("battery")
                .icon(icons::battery_icon(None))
                .tooltip("Заряд: …")
                .menu(&menu)
                .on_menu_event(handle_menu_event)
                .build(app)?;

            let handle = app.handle().clone();
            std::thread::spawn(move || poll_loop(handle, refresh_rx));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            apply_config,
            reset_buttons,
            battery
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } | RunEvent::Exit = event {
                // Release the interface and re-attach the kernel driver.
                let state = app.state::<AppState>();
                *state.mouse.lock().expect("mouse state") = None;
            }
        });
}

/// Показать скрытое или свёрнутое окно настроек (label "main").
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "quit" => app.exit(0),
        // Reveal the hidden settings window.
        "settings" => show_main_window(app),
        "refresh" => {
            let state = app.state::<AppState>();
            let _ = state.refresh_tx.lock().expect("refresh channel").send(());
        }
        _ => {}
    }
}

fn poll_loop(app: AppHandle, refresh_rx: Receiver<()>) {
    loop {
        let reading = read_battery(&app);
        apply_reading(&app, &reading);

        match refresh_rx.recv_timeout(POLL_INTERVAL) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn read_battery(app: &AppHandle) -> Reading {
    let state = app.state::<AppState>();
    with_mouse(&state, |mouse| {
        if mouse.is_wired() {
            Ok(Reading::Wired)
        } else {
            mouse
                .battery_percent()
                .map(Reading::Level)
                .map_err(|e| describe(&e))
        }
    })
    .unwrap_or_else(Reading::Failed)
}

fn apply_reading(app: &AppHandle, reading: &Reading) {
    let (level, label) = match reading {
        Reading::Level(pct) => (Some(*pct), format!("Заряд: {pct}%")),
        Reading::Wired => (None, "Питание от кабеля".to_string()),
        Reading::Failed(e) => (None, format!("Нет данных: {e}")),
    };

    let icon = icons::battery_icon(level);
    let item = app.state::<AppState>().battery_item.clone();
    let main = app.clone();

    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = main.tray_by_id("battery") {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_tooltip(Some(label.clone()));
        }
        let _ = item.set_text(&label);
    });

    let _ = app.emit("battery-changed", level);
}

#[cfg(test)]
mod tests {
    use super::describe;
    use openshark_driver::rusb::Error as Usb;
    use openshark_driver::Error as E;

    #[test]
    fn describe_maps_errors_to_human_reasons() {
        assert_eq!(describe(&E::NotFound), "мышь не найдена — проверьте приёмник");
        assert_eq!(describe(&E::Usb(Usb::Busy)), "мышь занята другим окном");
        assert_eq!(describe(&E::Usb(Usb::Access)), "нет доступа к USB");
        assert_eq!(describe(&E::Usb(Usb::Timeout)), "мышь не отвечает — шевельните её");
        assert_eq!(describe(&E::Usb(Usb::NoDevice)), "устройство отключено");
        assert!(describe(&E::Usb(Usb::Io)).starts_with("ошибка USB:"));
    }
}
