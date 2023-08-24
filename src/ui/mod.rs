mod utils;

use std::{
    borrow::BorrowMut,
    cell::Cell,
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::Duration,
};

use gdk_pixbuf::Pixbuf;
use gtk::{
    gdk,
    gio::SimpleAction,
    glib::{self, clone, Sender},
    pango,
    prelude::{ApplicationExt, ApplicationExtManual},
    subclass::prelude::ObjectSubclassIsExt,
    traits::{
        BoxExt, ButtonExt, ContainerExt, CssProviderExt, GtkWindowExt, ImageExt, LabelExt,
        ProgressBarExt, StyleContextExt, WidgetExt,
    },
    Align, Application, Box, Image, Label, PackType, ProgressBar, StyleContext, Window, WindowType,
};
use gtk_layer_shell::Edge;

use crate::daemon::{Notification, NotificationServer};

use self::utils::NotificationButton;

const APP_ID: &str = "org.dashie.oxinoti";

pub fn remove_notification(
    mainbox: &Box,
    window: &Window,
    noticount: Arc<Cell<i32>>,
    id: u32,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationButton>>>>,
    timed_out: bool,
    mutex: Arc<Mutex<bool>>,
) {
    let _guard = mutex.lock().unwrap();
    let notiopt = id_map.write().unwrap().remove(&id);
    if notiopt.is_none() {
        return;
    }
    let notibox = notiopt.unwrap();

    notibox.unmap();

    mainbox.remove(&*notibox);
    window.queue_resize();
    drop(notibox);

    noticount.update(|x| x - 1);
    let count = noticount.get();
    if count == 0 {
        window.hide();
    }

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
    noticount: Arc<Cell<i32>>,
    mainbox: &Box,
    window: &Window,
    notification: Notification,
    tx2: Arc<Sender<Arc<NotificationButton>>>,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationButton>>>>,
    mutex: Arc<Mutex<bool>>,
) {
    let mutexclone = mutex.clone();
    let _guard = mutex.lock().unwrap();
    let notibox = Arc::new(NotificationButton::new());
    notibox.style_context().add_class("NotificationBox");
    notibox.imp().notification_id.set(notification.replaces_id);
    notibox
        .imp()
        .reset
        .store(true, std::sync::atomic::Ordering::SeqCst);
    notibox.set_opacity(1.0);
    notibox.set_size_request(120, 120);
    let noticlone = notibox.clone();
    let noticlone2 = notibox.clone();
    let notiimp = noticlone2.imp();

    let basebox = Box::new(gtk::Orientation::Vertical, 5);
    let regularbox = Box::new(gtk::Orientation::Horizontal, 5);
    regularbox.set_homogeneous(true);
    notibox.style_context().add_class("NotificationBox");
    notibox
        .style_context()
        .add_class(notification.urgency.to_str());
    let bodybox = Box::new(gtk::Orientation::Vertical, 5);
    bodybox.style_context().add_class("bodybox");
    let imagebox = Box::new(gtk::Orientation::Horizontal, 5);
    imagebox.style_context().add_class("imagebox");
    // app name
    let app_name = Label::new(Some(&notification.app_name));
    app_name.style_context().add_class("appname");
    app_name.set_ellipsize(pango::EllipsizeMode::End);
    bodybox.add(&app_name);

    // summary
    if notification.summary != "" {
        notiimp.has_summary.set(true);
        let summary = Label::new(Some(&notification.summary));
        summary.style_context().add_class("summary");
        summary.set_wrap_mode(pango::WrapMode::Word);
        summary.set_line_wrap(true);
        summary.set_valign(Align::Center);
        let mut notisummary = notiimp.summary.borrow_mut();
        *notisummary = summary;
        bodybox.add(&*notisummary);
        bodybox.set_child_packing(&*notisummary, true, true, 5, PackType::Start);
    }

    // body
    if notification.body != "" {
        notiimp.has_body.set(true);
        let (body, text_css) = class_from_html(notification.body);
        let text = Label::new(None);
        text.style_context().add_class("text");
        text.style_context().add_class(&text_css);
        text.set_text(body.as_str());
        text.set_wrap_mode(pango::WrapMode::Word);
        text.set_line_wrap(true);
        text.set_valign(Align::Center);
        let mut notitext = notiimp.body.borrow_mut();
        *notitext = text;
        bodybox.add(&*notitext);
        bodybox.set_child_packing(&*notitext, true, true, 5, PackType::End);
    }

    regularbox.add(&bodybox);
    regularbox.add(&imagebox);
    regularbox.set_child_packing(&bodybox, true, true, 5, PackType::Start);
    regularbox.set_child_packing(&imagebox, true, true, 5, PackType::End);
    basebox.add(&regularbox);
    notibox.set_child(Some(&basebox));

    // image
    let image = Image::new();
    if set_image(notification.image_path, notification.app_icon, &image) {
        notiimp.has_image.set(true);
        let mut notiimage = notiimp.image.borrow_mut();
        *notiimage = image;
        imagebox.add(&*notiimage);
    }

    // progress bar
    if notification.progress.is_some() {
        notiimp.has_progbar.set(true);
        let progbar = ProgressBar::new();
        let mut shared_progbar = notiimp.fraction.borrow_mut();
        *shared_progbar = progbar;
        if let Some(progress) = notification.progress {
            if progress < 0 {
                return;
            }
            shared_progbar.set_fraction(progress as f64 / 100.0);
            basebox.add(&*shared_progbar);
        }
    }

    noticount.update(|x| x + 1);

    let id_map_clone = id_map.clone();
    let id = notibox.imp().notification_id.get();
    notibox.connect_clicked(
        clone!(@weak noticount, @weak mainbox, @weak window => move |_| {
            remove_notification(&mainbox, &window, noticount, id, id_map.clone(), false, mutexclone.clone());
        }),
    );

    id_map_clone
        .write()
        .unwrap()
        .insert(notification.replaces_id, noticlone);
    mainbox.add(&*notibox);
    thread::spawn(clone!(@weak notibox => move || {
        thread::sleep(Duration::from_secs(3));
        while notibox.imp().reset.load(std::sync::atomic::Ordering::SeqCst) == true {
            notibox.imp().reset.store(false, std::sync::atomic::Ordering::SeqCst);
            thread::sleep(Duration::from_secs(3));
        }
        tx2.send(notibox).unwrap();
    }));
    window.show_all();
}

pub fn modify_notification(
    notification: Notification,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationButton>>>>,
    mutex: Arc<Mutex<bool>>,
) {
    let _guard = mutex.lock().unwrap();
    let id = notification.replaces_id;
    let map = id_map.write().unwrap();
    let mut notibox = map.get(&id);
    let notibox_borrow_opt = notibox.borrow_mut();
    if notibox_borrow_opt.is_none() {
        return;
    }
    let notibox_borrow = notibox_borrow_opt.unwrap();
    let notiimp = notibox_borrow.imp();
    notiimp
        .reset
        .store(true, std::sync::atomic::Ordering::SeqCst);

    // progress bar
    let exists = notiimp.has_progbar.get();
    if let Some(progress) = notification.progress {
        if progress < 0 && exists {
            notibox_borrow.remove(&notiimp.fraction.take());
            notiimp.has_progbar.set(false);
        } else if progress > 0 {
            let mut progbar = notiimp.fraction.borrow_mut();
            if !exists {
                let newprog = ProgressBar::new();
                *progbar = newprog;
                notiimp.has_progbar.set(true);
            }
            progbar.set_fraction(progress as f64 / 100.0);
        }
    }

    let exists = notiimp.has_summary.get();
    if notification.summary == "" && exists {
        notibox_borrow.remove(&notiimp.summary.take());
        notiimp.has_summary.set(false);
    } else if notification.summary != "" {
        let (text, css_classes) = class_from_html(notification.summary);
        let mut text_borrow = notiimp.summary.borrow_mut();
        if !exists {
            *text_borrow = Label::new(None);
            notibox_borrow.add(&*text_borrow);
            notiimp.has_summary.set(true);
        }
        text_borrow.set_text(text.as_str());
        text_borrow.style_context().add_class("summary");
        text_borrow.style_context().add_class(&css_classes);
    }

    // body
    let exists = notiimp.has_body.get();
    if notification.body == "" && exists {
        notibox_borrow.remove(&notiimp.body.take());
        notiimp.has_body.set(false);
    } else if notification.body != "" {
        let (text, css_classes) = class_from_html(notification.body);
        let mut text_borrow = notiimp.body.borrow_mut();
        if !exists {
            *text_borrow = Label::new(None);
            notibox_borrow.add(&*text_borrow);
            notiimp.has_body.set(true);
        }
        text_borrow.set_text(text.as_str());
        text_borrow.style_context().add_class("text");
        text_borrow.style_context().add_class(&css_classes);
    }

    // image
    let exists = notiimp.has_image.get();
    let mut image_path = "".to_string();
    if notification.image_path.is_some() {
        image_path = notification.image_path.unwrap();
    }
    if image_path == "" && notification.app_icon == "" && exists {
        notibox_borrow.remove(&notiimp.image.take());
        notiimp.has_image.set(false);
    } else {
        let mut image_borrow = notiimp.image.borrow_mut();
        if !exists {
            let img = Image::new();
            *image_borrow = img;
            notibox_borrow.add(&*image_borrow);
            notiimp.has_image.set(true);
        }
        set_image(Some(image_path), notification.app_icon, &image_borrow);
    }
}

pub fn initialize_ui(css_string: String) {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(move |_| {
        if !gtk::is_initialized() {
            gtk::init().unwrap();
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
        let lock = Arc::new(Mutex::new(false));
        let lock2 = lock.clone();
        let mainbox = Box::new(gtk::Orientation::Vertical, 5);
        mainbox.style_context().add_class("MainBox");
        let window = Window::builder()
            .name("MainWindow")
            .application(app)
            .child(&mainbox)
            .type_(WindowType::Toplevel)
            .build();
        window.set_vexpand_set(true);
        window.set_hexpand_set(false);
        window.set_default_size(120, 120);

        gtk_layer_shell::init_for_window(&window);
        // gtk_layer_shell::set_keyboard_mode(&window, gtk4_layer_shell::KeyboardMode::None);
        gtk_layer_shell::auto_exclusive_zone_enable(&window);
        gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
        gtk_layer_shell::set_anchor(&window, Edge::Right, true);
        gtk_layer_shell::set_anchor(&window, Edge::Top, true);

        let windowrc = window.clone();
        let windowrc2 = windowrc.clone();

        // used in order to not close the window if we still have notifications
        let noticount = Arc::new(Cell::new(0));
        let noticount2 = noticount.clone();

        let id_map = Arc::new(RwLock::new(HashMap::<u32, Arc<NotificationButton>>::new()));
        let id_map_clone = id_map.clone();

        let action_present = SimpleAction::new("present", None);

        action_present.connect_activate(clone!(@weak window => move |_, _| {
            window.present();
        }));

        let mainbox2 = mainbox.clone();
        mainbox.set_hexpand_set(false);
        mainbox.set_vexpand_set(true);
        mainbox.set_size_request(120, 120);

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
                    lock2.clone(),
                );
            } else {
                modify_notification(notification, id_map.clone(), lock2.clone());
            }
            glib::Continue(true)
        });
        rx2.attach(None, move |notibox| {
            let id = notibox.imp().notification_id.get();
            drop(notibox);
            remove_notification(
                &mainbox2,
                &windowrc2,
                noticount2.clone(),
                id,
                id_map_clone.clone(),
                true,
                lock.clone(),
            );
            glib::Continue(true)
        });
    });

    fn load_css(css_string: &String) {
        let context_provider = gtk::CssProvider::new();
        if css_string != "" {
            if context_provider.load_from_path(css_string).is_err() {
                println!("Loading css failed! Please provide a path to a css file.");
            }
        }

        StyleContext::add_provider_for_screen(
            &gdk::Screen::default().unwrap(),
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
    (body, String::from(ret))
}

fn set_image(picture: Option<String>, icon: String, image: &Image) -> bool {
    let mut pixbuf: Option<Pixbuf> = None;
    let resize_pixbuf = |pixbuf: Option<Pixbuf>| {
        pixbuf
            .unwrap()
            .scale_simple(100, 100, gdk_pixbuf::InterpType::Bilinear)
    };
    let use_icon = |mut _pixbuf: Option<Pixbuf>| {
        if Path::new(&icon).is_file() {
            _pixbuf = Some(Pixbuf::from_file(&icon).unwrap());
            _pixbuf = resize_pixbuf(_pixbuf);
            image.set_pixbuf(Some(&_pixbuf.unwrap()));
            image.style_context().add_class("picture");
        } else {
            image.set_icon_name(Some(icon.as_str()));
            image.style_context().add_class("image");
        }
    };

    if let Some(path_opt) = picture {
        if Path::new(&path_opt).is_file() {
            pixbuf = Some(Pixbuf::from_file(path_opt).unwrap());
            pixbuf = resize_pixbuf(pixbuf);
            image.set_pixbuf(Some(&pixbuf.unwrap()));
            image.style_context().add_class("picture");
            return true;
        } else if icon != "" {
            (use_icon)(pixbuf);
            return true;
        }
    } else if icon != "" {
        (use_icon)(pixbuf);
        return true;
    }
    false
}
