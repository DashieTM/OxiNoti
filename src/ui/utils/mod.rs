mod notificationbutton;

use glib::Object;
use gtk::{glib, Orientation};

glib::wrapper! {
    pub struct NotificationButton(ObjectSubclass<notificationbutton::NotificationButton>)
        @extends gtk::Box, gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl NotificationButton {
    #![feature(cell_update)]
    pub fn new(orientation: Orientation, spacing: i32) -> Self {
        Object::builder()
            .property("orientation", orientation)
            .property("spacing", spacing)
            .build()
    }
}

impl Default for NotificationButton {
    fn default() -> Self {
        Self::new(Orientation::Vertical, 5)
    }
}
