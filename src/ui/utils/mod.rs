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

mod notificationbutton;
pub mod config;

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
