mod notificationbutton;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct NotificationButton(ObjectSubclass<notificationbutton::NotificationButton>)
        @extends gtk::Box, gtk::Button, gtk::Widget,
        @implements gtk::Actionable, gtk::Buildable, gtk::Container;
}

impl NotificationButton {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

impl Default for NotificationButton {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for NotificationButton {}
unsafe impl Sync for NotificationButton {}
