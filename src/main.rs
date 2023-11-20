use std::{cell::RefCell, sync::Arc};

use slint::{ComponentHandle, Weak};
use tray_icon::{menu::MenuEvent, ClickType, TrayIconEvent};

slint::include_modules!();

std::thread_local! {
    static UI_HANDLE: RefCell<Option<MyWindow>> = RefCell::new(None);
    static TRAY_HANDLE: RefCell<Option<tray_icon::TrayIcon>> = RefCell::new(None);
}

static LOGO: &[u8] = include_bytes!("../ui/img/logo.png");

#[derive(Clone, Default)]
struct State {
    inner: Arc<tokio::sync::Mutex<StateInner>>,
}

#[derive(Default)]
struct StateInner {
    ui_handle: Option<Weak<MyWindow>>,
}

impl State {
    async fn set_ui_handle(&self, handle: Weak<MyWindow>) {
        let mut this = self.inner.lock().await;
        this.ui_handle = Some(handle.clone());

        // let d = this.d.clone();
        let _res = handle.upgrade_in_event_loop(move |_ui| {
            // init from state
            // ui.set_d(d);
        });
    }
}

fn main() {
    let state = State::default();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("Runtime");
    let _guard = rt.enter();

    // launch logic task in tokio runtime

    init_systray(state.clone());
    open_ui(state);
    run_event_loop().expect("Event loop run without issue");

    // force clean-up (needed on Windows for the icon to be cleany remove from the task-bar)
    TRAY_HANDLE.with(|h| *h.borrow_mut() = None);
}

fn run_event_loop() -> Result<(), slint::PlatformError> {
    i_slint_backend_selector::with_platform(|b| {
        b.set_event_loop_quit_on_last_window_closed(false);
        Ok(())
    })?;
    slint::run_event_loop()
}

fn open_ui(state: State) {
    UI_HANDLE.with(move |handle| {
        let ui = handle.borrow().as_ref().map(|h| h.clone_strong());
        let Some(ui) = ui.or_else(move || create_ui(handle, state)) else {
            return;
        };

        let _res = ui.show();
    });
}

fn create_ui(handle: &RefCell<Option<MyWindow>>, state: State) -> Option<MyWindow> {
    let Ok(ui) = MyWindow::new() else {
        return None;
    };

    // set ui callbacks

    *handle.borrow_mut() = Some(ui.clone_strong());
    let ui_weak = ui.as_weak();
    tokio::spawn(async move {
        state.set_ui_handle(ui_weak).await;
    });
    Some(ui)
}

fn init_systray(state: State) {
    let tray = create_systray(state).expect("Create systray");
    TRAY_HANDLE.with(|h| *h.borrow_mut() = Some(tray));
}

fn load_icon() -> tray_icon::Icon {
    let icon = image::load_from_memory_with_format(LOGO, image::ImageFormat::Png)
        .expect("Logo")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    let rgba = icon.into_raw();
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Icon")
}

fn create_systray(state: State) -> Result<tray_icon::TrayIcon, Box<dyn std::error::Error>> {
    let close_item = tray_icon::menu::MenuItem::new("Close", true, None);
    let menu = tray_icon::menu::Menu::with_items(&[&close_item]).expect("Menu");

    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip("MyApp")
        .with_icon(load_icon())
        .with_menu(Box::new(menu))
        .build()?;

    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if event.click_type == ClickType::Left {
            let state = state.clone();
            let _ = slint::invoke_from_event_loop(move || {
                open_ui(state);
            });
        }
    }));

    let close_id = close_item.id().clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id() == &close_id {
            let _ = slint::quit_event_loop();
        }
    }));

    Ok(tray)
}
