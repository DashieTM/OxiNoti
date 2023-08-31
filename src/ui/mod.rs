/*
Copyright © 2023 Fabio Lenherr

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <http://www.gnu.org/licenses/>.
*/

pub mod utils;

use std::{
    borrow::BorrowMut,
    cell::Cell,
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::Duration,
};

use gtk::gdk_pixbuf::Pixbuf;
use gtk::{
    gdk,
    gio::SimpleAction,
    glib::{self, clone, Sender},
    pango,
    prelude::{ApplicationExt, ApplicationExtManual},
    subclass::prelude::ObjectSubclassIsExt,
    traits::{
        BoxExt, ButtonExt, ContainerExt, CssProviderExt, EntryExt, GtkWindowExt, ImageExt,
        LabelExt, ProgressBarExt, StyleContextExt, WidgetExt,
    },
    Align, Application, Box, Button, Image, Inhibit, Label, PackType, ProgressBar, StyleContext,
    Window, WindowType,
};
use gtk_layer_shell::Edge;

use crate::{
    daemon::{ImageData, Notification, NotificationServer},
    ui::utils::config::parse_config,
};

use self::utils::{config::Config, NotificationBox};

const APP_ID: &str = "org.dashie.oxinoti";

pub fn remove_notification(
    mainbox: &Box,
    window: &Window,
    noticount: Arc<Cell<i32>>,
    id: u32,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationBox>>>>,
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
    tx2: Arc<Sender<Arc<NotificationBox>>>,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationBox>>>>,
    mutex: Arc<Mutex<bool>>,
    config: Arc<Config>,
) {
    let mutexclone = mutex.clone();
    let mutexclone2 = mutex.clone();
    let _guard = mutex.lock().unwrap();

    let notibox = Arc::new(NotificationBox::new(gtk::Orientation::Vertical, 0));
    let notibutton = Button::new();
    notibox.set_opacity(1.0);
    notibox.style_context().add_class("NotificationBox");
    notibox.imp().notification_id.set(notification.replaces_id);
    notibox
        .imp()
        .reset
        .store(true, std::sync::atomic::Ordering::SeqCst);
    notibox.set_size_request(120, 5);
    let urgency_string = notification.urgency.to_str();
    notibox.style_context().add_class(urgency_string);
    notibox
        .imp()
        .previous_urgency
        .set(urgency_string.to_string());

    let noticlone = notibox.clone();
    let noticlone2 = notibox.clone();
    let notiimp = noticlone2.imp();

    let basebox = Box::new(gtk::Orientation::Vertical, 5);
    let regularbox = Box::new(gtk::Orientation::Horizontal, 5);

    let bodybox = Box::new(gtk::Orientation::Vertical, 5);
    bodybox.style_context().add_class("bodybox");

    // app name
    let app_name = Label::new(Some(&notification.app_name));
    app_name.style_context().add_class("appname");
    app_name.set_valign(Align::Center);
    app_name.set_halign(Align::Center);
    bodybox.add(&app_name);
    let mut has_body_image = false;
    let mut image_path = "".to_string();

    // summary
    if notification.summary != "" {
        notiimp.has_summary.set(true);
        let summary = Label::new(Some(&notification.summary));
        summary.style_context().add_class("summary");
        summary.set_wrap_mode(pango::WrapMode::Word);
        summary.set_width_chars(15);
        summary.set_line_wrap(true);
        summary.set_valign(Align::Center);
        summary.set_halign(Align::Center);
        let mut notisummary = notiimp.summary.borrow_mut();
        *notisummary = summary;
        bodybox.add(&*notisummary);
        bodybox.set_child_packing(&*notisummary, true, true, 5, PackType::Start);
    }

    // body
    if notification.body != "" {
        notiimp.has_body.set(true);
        let (body, text_css, has_image) = class_from_html(notification.body);
        let text = Label::new(None);
        if has_image {
            has_body_image = has_image;
            image_path = text_css;
        } else {
            text.style_context().add_class(&text_css);
        }
        text.style_context().add_class("text");
        text.set_markup(body.as_str());
        text.set_wrap_mode(pango::WrapMode::Word);
        text.set_width_chars(15);
        text.set_line_wrap(true);
        text.set_valign(Align::Center);
        text.set_halign(Align::Center);
        let mut notitext = notiimp.body.borrow_mut();
        *notitext = text;
        bodybox.add(&*notitext);
        bodybox.set_child_packing(&*notitext, true, true, 5, PackType::End);
    }

    regularbox.add(&bodybox);
    regularbox.set_child_packing(&bodybox, true, true, 5, PackType::Start);
    bodybox.set_halign(gtk::Align::Fill);
    basebox.add(&regularbox);
    notibox.add(&notibutton);
    notibutton.set_child(Some(&basebox));

    // image
    let image = Image::new();
    if has_body_image
        && set_image(
            notification.image_data.clone(),
            Some(image_path),
            notification.app_icon.clone(),
            &image,
        )
        || set_image(
            notification.image_data,
            notification.image_path,
            notification.app_icon,
            &image,
        )
    {
        notiimp.has_image.set(true);
        let mut notiimage = notiimp.image.borrow_mut();
        *notiimage = image;
        regularbox.add(&*notiimage);
        regularbox.set_child_packing(&*notiimage, true, true, 5, PackType::End);
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

    // inline reply
    let mut has_inline_reply = false;
    for action in notification.actions.iter() {
        if action == "inline-reply" {
            has_inline_reply = true;
        }
    }
    if has_inline_reply {
        notiimp.has_inline_reply.set(true);
        let inline_reply = gtk::Entry::new();
        inline_reply.set_focus_on_click(true);
        let id_map_clone = id_map.clone();
        let mut shared_inline_reply = notiimp.inline_reply.borrow_mut();
        inline_reply.connect_activate(
            clone!(@weak window, @weak noticount, @weak mainbox => move |entry| {
                let id = notification.replaces_id;
                let text = entry.text().to_string();
                activate_inline_reply(mainbox, id, noticount, window, id_map_clone.clone(), text, mutexclone.clone());
            }),
        );
        inline_reply.connect_button_press_event(
            clone!(@weak window => @default-return Inhibit(false), move |_, _| {
                gtk_layer_shell::set_keyboard_interactivity(&window, true);
                Inhibit(false)
            }),
        );
        inline_reply.connect_focus_out_event(
            clone!(@weak window => @default-return Inhibit(false), move |_,_| {
                gtk_layer_shell::set_keyboard_interactivity(&window, false);
            Inhibit(false)
            }),
        );
        *shared_inline_reply = inline_reply;
        notibox.add(&*shared_inline_reply);
    } else {
        notiimp.has_inline_reply.set(false);
    }

    noticount.update(|x| x + 1);

    // id_map used to retrieve notification afterwards
    let id_map_clone = id_map.clone();
    let id = notibox.imp().notification_id.get();
    notibutton.connect_clicked(
        clone!(@weak noticount, @weak mainbox, @weak window => move |_| {
        let id_clone = id;
        thread::spawn(move || {
            use dbus::blocking::Connection;

            let conn = Connection::new_session().unwrap();
            let proxy = conn.with_proxy(
                "org.freedesktop.Notifications",
                "/org/freedesktop/Notifications",
                Duration::from_millis(1000),
            );
            let _: Result<(), dbus::Error> =
                proxy.method_call("org.freedesktop.Notifications", "InvokeAction", (id_clone,"default"));
        });
            remove_notification(&mainbox, &window, noticount, id, id_map.clone(), false, mutexclone2.clone());
        }),
    );

    id_map_clone
        .write()
        .unwrap()
        .insert(notification.replaces_id, noticlone);
    mainbox.add(&*notibox);

    let mut notibodybox = notiimp.bodybox.borrow_mut();
    *notibodybox = bodybox;
    let mut notibasebox = notiimp.basebox.borrow_mut();
    *notibasebox = basebox;
    let mut notiregularbox = notiimp.regularbox.borrow_mut();
    *notiregularbox = regularbox;

    // thread removes notification after timeout
    thread::spawn(clone!(@weak notibox => move || {
        thread::sleep(Duration::from_secs(config.timeout));
        while notibox.imp().reset.load(std::sync::atomic::Ordering::SeqCst) == true {
            notibox.imp().reset.store(false, std::sync::atomic::Ordering::SeqCst);
            thread::sleep(Duration::from_secs(config.timeout));
        }
        tx2.send(notibox).unwrap();
    }));
    window.show_all();
}

pub fn modify_notification(
    noticount: Arc<Cell<i32>>,
    mainbox: &Box,
    window: &Window,
    notification: Notification,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationBox>>>>,
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
    let notibodybox = notiimp.bodybox.borrow_mut();
    let notibasebox = notiimp.basebox.borrow_mut();
    let notiregularbox = notiimp.regularbox.borrow_mut();
    notiimp
        .reset
        .store(true, std::sync::atomic::Ordering::SeqCst);
    notibox_borrow.style_context().restore();
    let urgency_string = notification.urgency.to_str();
    notibox_borrow
        .style_context()
        .remove_class(&notiimp.previous_urgency.take());
    notiimp.previous_urgency.set(urgency_string.to_string());
    notibox_borrow.style_context().add_class(urgency_string);

    // progress bar
    let exists = notiimp.has_progbar.get();
    if let Some(progress) = notification.progress {
        if progress < 0 && exists {
            notibasebox.remove(&notiimp.fraction.take());
            notiimp.has_progbar.set(false);
        } else if progress > 0 {
            let mut progbar = notiimp.fraction.borrow_mut();
            if !exists {
                let newprog = ProgressBar::new();
                *progbar = newprog;
                notibasebox.add(&*progbar);
                notiimp.has_progbar.set(true);
            }
            progbar.set_fraction(progress as f64 / 100.0);
        }
    }

    // inline reply
    let mut has_inline_reply = false;
    for action in notification.actions.iter() {
        if action == "inline-reply" {
            has_inline_reply = true;
        }
    }
    let exists = notiimp.has_inline_reply.get();
    if !has_inline_reply && exists {
        notibasebox.remove(&notiimp.inline_reply.take());
        notiimp.has_inline_reply.set(false);
    } else if has_inline_reply {
        let mut entry = notiimp.inline_reply.borrow_mut();
        if !exists {
            let newentry = gtk::Entry::new();
            let mutexclone = mutex.clone();
            let id_map_clone = id_map.clone();
            newentry.connect_activate(
            clone!(@weak window, @weak noticount, @weak mainbox => move |entry| {
                let id = notification.replaces_id;
                let text = entry.text().to_string();
                activate_inline_reply(mainbox, id, noticount, window, id_map_clone.clone(), text, mutexclone.clone());
            }),
        );
            newentry.connect_button_press_event(
                clone!(@weak notiimp, @weak window => @default-return Inhibit(false), move |_, _| {
                    notiimp.reset.store(true, std::sync::atomic::Ordering::SeqCst);
                    gtk_layer_shell::set_keyboard_interactivity(&window, true);
                    Inhibit(false)
                }),
            );
            newentry.connect_focus_out_event(
                clone!(@weak notiimp, @weak window => @default-return Inhibit(false), move |_,_| {
                    notiimp.reset.store(false, std::sync::atomic::Ordering::SeqCst);
                    gtk_layer_shell::set_keyboard_interactivity(&window, false);
                Inhibit(false)
                }),
            );
            *entry = newentry;
            notibasebox.add(&*entry);
            notiimp.has_inline_reply.set(true);
        }
    }

    // summary
    let exists = notiimp.has_summary.get();
    if notification.summary == "" && exists {
        notibodybox.remove(&notiimp.summary.take());
        notiimp.has_summary.set(false);
    } else if notification.summary != "" {
        let (text, css_classes, _) = class_from_html(notification.summary);
        let mut text_borrow = notiimp.summary.borrow_mut();
        if !exists {
            *text_borrow = Label::new(None);
            notibodybox.add(&*text_borrow);
            notiimp.has_summary.set(true);
        }
        text_borrow.set_text(text.as_str());
        text_borrow.style_context().add_class("summary");
        text_borrow.style_context().add_class(&css_classes);
    }

    let mut has_body_image = false;
    let mut body_image_path = "".to_string();

    // body
    let exists = notiimp.has_body.get();
    if notification.body == "" && exists {
        notibodybox.remove(&notiimp.body.take());
        notiimp.has_body.set(false);
    } else if notification.body != "" {
        let (text, css_classes, has_image) = class_from_html(notification.body);
        let mut text_borrow = notiimp.body.borrow_mut();
        if !exists {
            *text_borrow = Label::new(None);
            notibodybox.add(&*text_borrow);
            notiimp.has_body.set(true);
        }
        if has_image {
            has_body_image = has_image;
            body_image_path = css_classes;
        } else {
            text_borrow.style_context().add_class(&css_classes);
        }
        text_borrow.set_text(text.as_str());
        text_borrow.style_context().add_class("text");
    }

    // image
    let exists = notiimp.has_image.get();
    let mut image_path = "".to_string();
    if notification.image_path.is_some() {
        image_path = notification.image_path.unwrap();
    }
    if image_path == "" && notification.app_icon == "" && exists {
        notiregularbox.remove(&notiimp.image.take());
        notiimp.has_image.set(false);
    } else {
        let mut image_borrow = notiimp.image.borrow_mut();
        if !exists {
            let img = Image::new();
            *image_borrow = img;
            notiregularbox.add(&*image_borrow);
            notiimp.has_image.set(true);
        }
        if has_body_image {
            set_image(
                notification.image_data.clone(),
                Some(body_image_path),
                notification.app_icon.clone(),
                &image_borrow,
            );
        } else {
            set_image(
                notification.image_data,
                Some(image_path),
                notification.app_icon,
                &image_borrow,
            );
        }
    }
}

pub fn initialize_ui(css_string: String, config_file: String) {
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
        let config = Arc::new(parse_config(&config_file));
        let configrc = config.clone();
        thread::spawn(move || {
            let mut server = NotificationServer::create(tx);
            server.run(configrc);
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
        window.connect_button_press_event(
            clone!(@weak window => @default-return Inhibit(false), move |_,_| {
                gtk_layer_shell::set_keyboard_interactivity(&window, false);
            Inhibit(false)
            }),
        );
        window.set_vexpand_set(true);
        window.set_hexpand_set(false);
        window.set_default_size(120, 120);

        gtk_layer_shell::init_for_window(&window);
        gtk_layer_shell::auto_exclusive_zone_enable(&window);
        // gtk_layer_shell::set_keyboard_mode(&window, gtk_layer_shell::KeyboardMode::OnDemand);
        gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
        gtk_layer_shell::set_anchor(&window, Edge::Right, true);
        gtk_layer_shell::set_anchor(&window, Edge::Top, true);

        let windowrc = window.clone();
        let windowrc2 = windowrc.clone();

        // used in order to not close the window if we still have notifications
        let noticount = Arc::new(Cell::new(0));
        let noticount2 = noticount.clone();

        let id_map = Arc::new(RwLock::new(HashMap::<u32, Arc<NotificationBox>>::new()));
        let id_map_clone = id_map.clone();

        let action_present = SimpleAction::new("present", None);

        action_present.connect_activate(clone!(@weak window => move |_, _| {
            window.present();
        }));

        let mainbox2 = mainbox.clone();
        mainbox.set_hexpand_set(false);
        mainbox.set_vexpand_set(true);
        mainbox.set_size_request(120, 120);

        // new notification added
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
                    config.clone(),
                );
            } else {
                // modify notification if id is already in map
                modify_notification(
                    noticount.clone(),
                    &mainbox,
                    &window,
                    notification,
                    id_map.clone(),
                    lock2.clone(),
                );
            }
            glib::Continue(true)
        });
        // handle notification removal
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

fn class_from_html(mut body: String) -> (String, String, bool) {
    let mut ret: &str = "";
    let mut retstring = body.clone();
    let has_image: bool;
    if body.contains("<br><img src=\"file:///") {
        has_image = true;
        let split = retstring.split_once("<br><img src=\"file:///").unwrap();
        body = split.0.to_string() + "sent an image.".into();
        retstring = split.1.to_string();
        let split = retstring.split_once("\"");
        if split.is_some() {
            ret = split.unwrap().0;
        }
    } else {
        has_image = false;
        let mut open = false;
        for char in body.chars() {
            if char == '<' && !open {
                open = true;
            } else if open {
                ret = match char {
                    'u' => "underline",
                    _ => {
                        ret = "";
                        break;
                    }
                };
                break;
            }
        }
        body.remove_matches("<u>");
        body.remove_matches("</u>");
    }
    (body, String::from(ret), has_image)
}

fn set_image(
    data: Option<ImageData>,
    picture: Option<String>,
    icon: String,
    image: &Image,
) -> bool {
    let mut pixbuf: Option<Pixbuf> = None;
    let resize_pixbuf = |pixbuf: Option<Pixbuf>| {
        pixbuf
            .unwrap()
            .scale_simple(100, 100, gtk::gdk_pixbuf::InterpType::Bilinear)
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
    } else if data.is_some() {
        let image_data = data.unwrap();
        let bytes = gtk::glib::Bytes::from(&image_data.data);
        pixbuf = Some(Pixbuf::from_bytes(
            &bytes,
            gtk::gdk_pixbuf::Colorspace::Rgb,
            image_data.has_alpha,
            image_data.bits_per_sample,
            image_data.width,
            image_data.height,
            image_data.rowstride,
        ));
        pixbuf = resize_pixbuf(pixbuf);
        image.set_pixbuf(Some(&pixbuf.unwrap()));
        image.style_context().add_class("picture");
        return true;
    }
    false
}

pub fn activate_inline_reply(
    mainbox: Box,
    id: u32,
    noticount: Arc<Cell<i32>>,
    window: Window,
    id_map: Arc<RwLock<HashMap<u32, Arc<NotificationBox>>>>,
    text: String,
    mutex: Arc<Mutex<bool>>,
) {
    thread::spawn(move || {
        use dbus::blocking::Connection;

        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            Duration::from_millis(1000),
        );
        let _: Result<(), dbus::Error> =
            proxy.method_call("org.freedesktop.Notifications", "InlineReply", (id, text));
    });
    gtk_layer_shell::set_keyboard_interactivity(&window, false);
    remove_notification(&mainbox, &window, noticount, id, id_map, false, mutex);
}
