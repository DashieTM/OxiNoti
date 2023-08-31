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

pub mod config;
mod notificationbutton;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct NotificationBox(ObjectSubclass<notificationbutton::NotificationBox>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Actionable, gtk::Buildable, gtk::Container;
}

impl NotificationBox {
    pub fn new(orientation: gtk::Orientation, spacing: i32) -> Self {
        Object::builder()
            .property("orientation", orientation)
            .property("spacing", spacing)
            .build()
    }
}

impl Default for NotificationBox {
    fn default() -> Self {
        Self::new(gtk::Orientation::Horizontal, 0)
    }
}

unsafe impl Send for NotificationBox {}
unsafe impl Sync for NotificationBox {}
