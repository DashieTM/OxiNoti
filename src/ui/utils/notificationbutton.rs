use std::cell::Cell;
use std::sync::RwLock;

use gtk::glib;
use gtk::subclass::prelude::*;

#[derive(Default)]
pub struct NotificationButton {
    pub notification_id: Cell<u32>,
    pub removed: Cell<bool>,
    // pub notification_count: Cell<i32>,
}

#[glib::object_subclass]
impl ObjectSubclass for NotificationButton {
    const NAME: &'static str = "NotificationButton";
    type Type = super::NotificationButton;
    type ParentType = gtk::Box;
}

// impl NotificationButton {
//     pub fn increase(&self) {
//         self.notification_count.update(|x| x + 1);
//     }
//     pub fn decrease(&self) {
//         self.notification_count.update(|x| x - 1);
//     }
// }

impl ObjectImpl for NotificationButton {}

impl WidgetImpl for NotificationButton {}

impl BoxImpl for NotificationButton {}

impl ButtonImpl for NotificationButton {}
