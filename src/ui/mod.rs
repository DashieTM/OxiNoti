mod utils;

use std::{cell::Cell, rc::Rc, thread};

use adw::{traits::AdwWindowExt, Window};
use gtk::{
    gio::SimpleAction,
    glib::{self, clone},
    prelude::{ApplicationExt, ApplicationExtManual},
    subclass::prelude::ObjectSubclassIsExt,
    traits::{BoxExt, GestureExt, GestureSingleExt, GtkWindowExt, WidgetExt},
    Application, Box, Label,
};
use gtk4_layer_shell::Edge;

use crate::daemon::{Notification, NotificationServer};

use self::utils::NotificationButton;

const APP_ID: &str = "org.dashie.oxinoti";
pub fn show_notification(
    noticount: Rc<Cell<i32>>,
    mainbox: &Box,
    window: &Window,
    notification: Notification,
) {
    let notibox = NotificationButton::new(gtk::Orientation::Horizontal, 5);
    notibox.set_css_classes(&["NotificationBox"]);
    let bodybox = Box::new(gtk::Orientation::Vertical, 5);
    let imagebox = Box::new(gtk::Orientation::Vertical, 5);

    let summary = Label::new(Some(&notification.summary));
    let app_name = Label::new(Some(&notification.app_name));
    let text = Label::new(Some(&notification.body));
    let timestamp = Label::new(Some(&notification.expire_timeout.to_string()));

    bodybox.append(&summary);
    bodybox.append(&app_name);
    bodybox.append(&text);
    bodybox.append(&timestamp);
    notibox.append(&bodybox);
    notibox.append(&imagebox);

    notibox.imp().notification_id.set(notification.replaces_id);
    noticount.update(|x| x + 1);

    let gesture = gtk::GestureClick::new();
    gesture.connect_pressed(
        clone!(@strong noticount, @weak notibox, @weak mainbox, @weak window => move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            mainbox.remove(&notibox);
            noticount.update(|x| x - 1);
            if noticount.get() == 0 {
            window.hide();
            }
            let id = notibox.imp().notification_id.get();
            thread::spawn(move || {
                use dbus::blocking::Connection;
                use std::time::Duration;

                let conn = Connection::new_session().unwrap();
                let proxy = conn.with_proxy("org.freedesktop.Notifications2", "/org/freedesktop/Notifications2", Duration::from_millis(1000));
                let _: Result<(), dbus::Error> =
                    proxy.method_call("org.freedesktop.Notifications2", "CloseNotification", (id,));
            });
        }),
    );
    notibox.add_controller(gesture);

    mainbox.append(&notibox);
    window.set_content(Some(mainbox));
    window.show();
}

pub fn initialize_ui(css_string: String) {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(move |_| {
        if !adw::is_initialized() {
            adw::init().unwrap();
        }
        load_css(&css_string);
    });

    app.connect_activate(move |app| {
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        thread::spawn(move || {
            let mut server = NotificationServer::create(tx);
            server.run();
        });
        let window = Window::builder()
            .name("MainWindow")
            .application(app)
            .build();
        window.set_vexpand_set(true);

        gtk4_layer_shell::init_for_window(&window);
        gtk4_layer_shell::set_keyboard_mode(&window, gtk4_layer_shell::KeyboardMode::None);
        gtk4_layer_shell::auto_exclusive_zone_enable(&window);
        gtk4_layer_shell::set_layer(&window, gtk4_layer_shell::Layer::Overlay);
        gtk4_layer_shell::set_anchor(&window, Edge::Right, true);
        gtk4_layer_shell::set_anchor(&window, Edge::Top, true);

        let windowrc = window.clone();
        let windowrc1 = windowrc.clone();

        // used in order to not close the window if we still have notifications
        let noticount = Rc::new(Cell::new(0));

        let action_present = SimpleAction::new("present", None);

        action_present.connect_activate(clone!(@weak window => move |_, _| {
            window.present();
        }));

        let focus_event_controller = gtk::EventControllerFocus::new();
        focus_event_controller.connect_leave(move |_| {
            windowrc.hide();
        });

        let gesture = gtk::GestureClick::new();
        gesture.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
        gesture.connect_pressed(move |_gesture, _, _, _| {
            windowrc1.hide();
        });
        let mainbox = Box::new(gtk::Orientation::Vertical, 5);
        window.add_controller(focus_event_controller);
        window.add_controller(gesture);

        rx.attach(None, move |notification| {
            show_notification(noticount.clone(), &mainbox, &window, notification);
            glib::Continue(true)
        });
    });

    fn load_css(css_string: &String) {
        let context_provider = gtk::CssProvider::new();
        if css_string != "" {
            context_provider.load_from_path(css_string);
        }

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &context_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    app.run_with_args(&[""]);
}
