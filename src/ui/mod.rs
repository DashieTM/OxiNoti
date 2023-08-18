mod utils;

use std::{cell::Cell, rc::Rc, sync::Arc, thread, time::Duration};

use adw::{traits::AdwWindowExt, Window};
use gtk::{
    gio::SimpleAction,
    glib::{self, clone, Sender},
    prelude::{ApplicationExt, ApplicationExtManual},
    subclass::prelude::ObjectSubclassIsExt,
    traits::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt},
    Application, Box, Image, Label, Picture,
};
use gtk4_layer_shell::Edge;

use crate::daemon::{Notification, NotificationServer};

use self::utils::NotificationButton;

const APP_ID: &str = "org.dashie.oxinoti";

pub fn remove_notification(
    mainbox: &Box,
    window: &Window,
    noticount: Rc<Cell<i32>>,
    notibox: &NotificationButton,
) {
    if notibox.imp().removed.get() {
        println!("wat");
        return;
    }
    notibox.imp().removed.set(true);
    noticount.update(|x| x - 1);
    if noticount.get() == 0 {
        window.hide();
    }
    let id = notibox.imp().notification_id.get();
    // notibox.unmap();
    mainbox.remove(&*notibox);
    thread::spawn(move || {
        use dbus::blocking::Connection;

        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy(
            "org.freedesktop.Notifications2",
            "/org/freedesktop/Notifications2",
            Duration::from_millis(1000),
        );
        let _: Result<(), dbus::Error> =
            proxy.method_call("org.freedesktop.Notifications2", "CloseNotification", (id,));
    });
}

pub fn show_notification(
    noticount: Rc<Cell<i32>>,
    mainbox: &Box,
    window: &Window,
    notification: Notification,
    tx2: Arc<Sender<Arc<NotificationButton>>>,
) {
    let notibox = Arc::new(NotificationButton::new());
    let basebox = Box::new(gtk::Orientation::Horizontal, 5);
    notibox.set_css_classes(&["NotificationBox", notification.urgency.to_str()]);
    let bodybox = Box::new(gtk::Orientation::Vertical, 5);
    let imagebox = Box::new(gtk::Orientation::Vertical, 5);
    let appbox = Box::new(gtk::Orientation::Horizontal, 2);

    let summary = Label::new(Some(&notification.summary));
    let app_name = Label::new(Some(&notification.app_name));
    let timestamp = Label::new(Some(&notification.expire_timeout.to_string()));
    let (body, text_css) = class_from_html(notification.body);
    let text = Label::new(Some(body.as_str()));
    text.set_css_classes(&[&text_css]);

    let image = Image::from_icon_name(notification.app_icon.as_str());
    imagebox.append(&image);
    let picture = Picture::new();
    picture.set_filename(notification.image_path.clone());
    imagebox.append(&picture);

    appbox.append(&app_name);
    appbox.append(&timestamp);
    bodybox.append(&appbox);
    bodybox.append(&summary);
    bodybox.append(&text);
    basebox.append(&bodybox);
    basebox.append(&imagebox);
    notibox.set_child(Some(&basebox));

    notibox.imp().notification_id.set(notification.replaces_id);
    notibox.imp().removed.set(false);
    noticount.update(|x| x + 1);

    notibox.connect_clicked(
        clone!(@weak noticount, @weak mainbox, @weak window => move |notibox| {
            println!("clicked");
            remove_notification(&mainbox, &window, noticount, notibox);
        }),
    );

    mainbox.append(&*notibox);
    window.set_content(Some(mainbox));
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(10));
        tx2.send(notibox).unwrap();
    });
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
        let (tx2_initial, rx2) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let tx2 = Arc::new(tx2_initial);
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
        let windowrc2 = windowrc.clone();

        // used in order to not close the window if we still have notifications
        let noticount = Rc::new(Cell::new(0));
        let noticount2 = noticount.clone();

        let action_present = SimpleAction::new("present", None);

        action_present.connect_activate(clone!(@weak window => move |_, _| {
            window.present();
        }));

        let mainbox = Box::new(gtk::Orientation::Vertical, 5);
        let mainbox2 = mainbox.clone();

        rx.attach(None, move |notification| {
            show_notification(
                noticount.clone(),
                &mainbox,
                &window,
                notification,
                tx2.clone(),
            );
            glib::Continue(true)
        });
        rx2.attach(None, move |notibox| {
            remove_notification(&mainbox2, &windowrc2, noticount2.clone(), &*notibox);
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

fn class_from_html(mut body: String) -> (String, String) {
    let mut open = false;
    let mut ret: &str = "";
    for char in body.chars() {
        if char == '<' && !open {
            open = true;
        } else if open {
            ret = match char {
                'b' => "bold",
                'i' => "italic",
                'u' => "underline",
                'h' => "hyprlink",
                _ => {
                    ret = "";
                    break;
                }
            };
            break;
        }
    }
    body.remove_matches("<b>");
    body.remove_matches("</b>");
    body.remove_matches("<i>");
    body.remove_matches("</i>");
    body.remove_matches("<a href=\">");
    body.remove_matches("</a>");
    body.remove_matches("<u>");
    body.remove_matches("</u>");
    // let new_body = body.remove_matches("<img src=\">");
    // let new_body = body.remove_matches("<alt=\">");
    (body, String::from(ret))
}
