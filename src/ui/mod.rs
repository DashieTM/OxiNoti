mod utils;

use std::{
    borrow::BorrowMut,
    cell::Cell,
    collections::HashMap,
    path::Path,
    rc::Rc,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use adw::{traits::AdwWindowExt, Window};
use gtk::{
    gio::SimpleAction,
    glib::{self, clone, Sender},
    prelude::{ApplicationExt, ApplicationExtManual},
    subclass::prelude::ObjectSubclassIsExt,
    traits::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt},
    Application, Box, Image, Label, ProgressBar,
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
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationButton>>>>,
    timed_out: bool,
) {
    if notibox.imp().removed.get() {
        return;
    }
    notibox.imp().removed.set(true);
    noticount.update(|x| x - 1);
    let count = noticount.get();
    if count == 0 {
        window.set_visible(false);
    }
    let id = notibox.imp().notification_id.get();
    id_map.write().unwrap().remove(&id);
    // notibox.unmap();
    mainbox.remove(&*notibox);
    window.queue_resize();
    if timed_out {
        return;
    }
    thread::spawn(move || {
        use dbus::blocking::Connection;

        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            Duration::from_millis(1000),
        );
        let _: Result<(), dbus::Error> =
            proxy.method_call("org.freedesktop.Notifications", "CloseNotification", (id,));
    });
}

pub fn show_notification(
    noticount: Rc<Cell<i32>>,
    mainbox: &Box,
    window: &Window,
    notification: Notification,
    tx2: Arc<Sender<Arc<NotificationButton>>>,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationButton>>>>,
) {
    let notibox = Arc::new(NotificationButton::new());
    notibox.set_opacity(1.0);
    notibox.set_size_request(300, 120);
    let noticlone = notibox.clone();
    let noticlone2 = notibox.clone();
    let basebox = Box::new(gtk::Orientation::Vertical, 5);
    let regularbox = Box::new(gtk::Orientation::Horizontal, 5);
    notibox.set_css_classes(&["NotificationBox", notification.urgency.to_str()]);
    let bodybox = Box::new(gtk::Orientation::Vertical, 5);
    bodybox.set_css_classes(&[&"bodybox"]);
    let imagebox = Box::new(gtk::Orientation::Horizontal, 5);
    imagebox.set_css_classes(&[&"imagebox"]);
    let appbox = Box::new(gtk::Orientation::Horizontal, 2);
    appbox.set_css_classes(&[&"miscbox"]);

    let summary = Label::new(Some(&notification.summary));
    summary.set_css_classes(&[&"summary"]);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let mut notisummary = noticlone2.imp().summary.borrow_mut();
    *notisummary = summary;
    let app_name = Label::new(Some(&notification.app_name));
    app_name.set_css_classes(&[&"appname"]);
    app_name.set_ellipsize(gtk::pango::EllipsizeMode::End);
    // let timestamp = Label::new(Some(&notification.expire_timeout.to_string()));
    // timestamp.set_css_classes(&[&"timestamp"]);
    let (body, text_css) = class_from_html(notification.body);
    let text = Label::new(None);
    text.set_css_classes(&[&text_css, &"text"]);
    text.set_text(body.as_str());
    text.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let mut notitext = noticlone2.imp().body.borrow_mut();
    *notitext = text;

    appbox.append(&app_name);
    // appbox.append(&timestamp);
    bodybox.append(&appbox);
    bodybox.append(&*notisummary);
    bodybox.append(&*notitext);
    regularbox.append(&bodybox);
    regularbox.append(&imagebox);
    basebox.append(&regularbox);
    notibox.set_child(Some(&basebox));

    let image = Image::new();
    set_image(notification.image_path, notification.app_icon, &image);
    let mut notiimage = noticlone2.imp().image.borrow_mut();
    *notiimage = image;
    imagebox.append(&*notiimage);

    if let Some(progress) = notification.progress {
        if progress < 0 {
            return;
        }
        let progbar = ProgressBar::new();
        progbar.set_fraction(progress as f64 / 100.0);
        let mut shared_progbar = notibox.imp().fraction.borrow_mut();
        *shared_progbar = progbar;
        basebox.append(&*shared_progbar);
    }

    notibox.imp().notification_id.set(notification.replaces_id);
    notibox.imp().reset.set(false);
    notibox.imp().removed.set(false);
    noticount.update(|x| x + 1);

    let id_map_clone = id_map.clone();

    notibox.connect_clicked(
        clone!(@weak noticount, @weak mainbox, @weak window => move |notibox| {
            remove_notification(&mainbox, &window, noticount, notibox, id_map.clone(), false);
        }),
    );

    id_map_clone
        .write()
        .unwrap()
        .insert(notification.replaces_id, noticlone);
    notibox.set_size_request(300, 120);
    mainbox.append(&*notibox);
    window.set_content(Some(mainbox));
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(10));
        while notibox.imp().reset.get() == true {
            notibox.imp().reset.set(false);
            thread::sleep(Duration::from_secs(10));
        }
        tx2.send(notibox).unwrap();
    });
    window.show();
}

pub fn modify_notification(
    notification: Notification,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationButton>>>>,
) {
    let id = notification.replaces_id;
    let map = id_map.write().unwrap();
    let mut notibox = map.get(&id);
    let notibox_borrow = notibox.borrow_mut().unwrap().imp();
    notibox_borrow.reset.set(true);
    if let Some(progress) = notification.progress {
        if progress < 0 {
            return;
        }
        notibox_borrow
            .fraction
            .borrow_mut()
            .set_fraction(progress as f64 / 100.0);
    }
    let (text, css_classes) = class_from_html(notification.summary);
    let text_borrow = notibox_borrow.summary.borrow_mut();
    text_borrow.set_text(text.as_str());
    text_borrow.set_css_classes(&[&css_classes, &"summary"]);
    let (text, css_classes) = class_from_html(notification.body);
    let text_borrow = notibox_borrow.body.borrow_mut();
    text_borrow.set_text(text.as_str());
    text_borrow.set_css_classes(&[&css_classes, &"text"]);
    let image_borrow = notibox_borrow.image.borrow_mut();
    set_image(
        notification.image_path,
        notification.app_icon,
        &image_borrow,
    );
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
        window.set_hexpand_set(false);
        window.set_default_size(300, 120);

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

        let id_map = Arc::new(RwLock::new(HashMap::<u32, Arc<NotificationButton>>::new()));
        let id_map_clone = id_map.clone();

        let action_present = SimpleAction::new("present", None);

        action_present.connect_activate(clone!(@weak window => move |_, _| {
            window.present();
        }));

        let mainbox = Box::new(gtk::Orientation::Vertical, 5);
        mainbox.set_css_classes(&[&"MainBox"]);
        let mainbox2 = mainbox.clone();
        mainbox.set_hexpand_set(false);
        mainbox.set_vexpand_set(true);
        mainbox.set_size_request(300, 120);

        rx.attach(None, move |notification| {
            if id_map
                .read()
                .unwrap()
                .get(&notification.replaces_id)
                .is_none()
            {
                show_notification(
                    noticount.clone(),
                    &mainbox,
                    &window,
                    notification,
                    tx2.clone(),
                    id_map.clone(),
                );
            } else {
                modify_notification(notification, id_map.clone());
            }
            glib::Continue(true)
        });
        rx2.attach(None, move |notibox| {
            remove_notification(
                &mainbox2,
                &windowrc2,
                noticount2.clone(),
                &*notibox,
                id_map_clone.clone(),
                true,
            );
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

fn set_image(picture: Option<String>, icon: String, image: &Image) {
    let use_icon = || {
        if Path::new(&icon).is_file() {
            image.set_file(Some(&icon));
            image.set_css_classes(&[&"picture"]);
            image.set_size_request(100, 100);
        } else {
            image.set_icon_name(Some(icon.as_str()));
            image.set_css_classes(&[&"image"]);
        }
    };

    if let Some(path_opt) = picture {
        if Path::new(&path_opt).is_file() {
            image.set_file(Some(path_opt.as_str()));
            image.set_size_request(100, 100);
            image.set_css_classes(&[&"picture"]);
        } else {
            (use_icon)();
        }
    } else {
        (use_icon)();
    }
}
