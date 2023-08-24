use std::cell::{Cell, RefCell};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use gtk::subclass::prelude::*;
use gtk::{glib, Image, Label, ProgressBar};

#[derive(Default)]
pub struct NotificationButton {
    pub notification_id: Cell<u32>,
    pub removed: Mutex<bool>,
    pub fraction: RefCell<ProgressBar>,
    pub body: RefCell<Label>,
    pub summary: RefCell<Label>,
    pub image: RefCell<Image>,
    pub reset: AtomicBool,
    pub has_body: Cell<bool>,
    pub has_summary: Cell<bool>,
    pub has_image: Cell<bool>,
    pub has_progbar: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for NotificationButton {
    const NAME: &'static str = "NotificationButton";
    type Type = super::NotificationButton;
    type ParentType = gtk::Button;
}

impl ObjectImpl for NotificationButton {}

impl WidgetImpl for NotificationButton {}

impl ContainerImpl for NotificationButton {}

impl BoxImpl for NotificationButton {}

impl BinImpl for NotificationButton {}

impl ButtonImpl for NotificationButton {}

