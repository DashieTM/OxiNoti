use std::cell::{Cell, RefCell};

use gtk::{glib, ProgressBar};
use gtk::subclass::prelude::*;

#[derive(Default)]
pub struct NotificationButton {
    pub notification_id: Cell<u32>,
    pub removed: Cell<bool>,
    pub fraction: RefCell<ProgressBar>
    // pub notification_count: Cell<i32>,
}

#[glib::object_subclass]
impl ObjectSubclass for NotificationButton {
    const NAME: &'static str = "NotificationButton";
    type Type = super::NotificationButton;
    type ParentType = gtk::Button;
}

impl ObjectImpl for NotificationButton {}

impl WidgetImpl for NotificationButton {}

impl BoxImpl for NotificationButton {}

impl ButtonImpl for NotificationButton {}
