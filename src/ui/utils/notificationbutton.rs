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

use std::cell::{Cell, RefCell};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use gtk::subclass::prelude::*;
use gtk::{glib, Image, Label, ProgressBar};

#[derive(Default)]
pub struct NotificationBox {
    pub notification_id: Cell<u32>,
    pub removed: Mutex<bool>,
    pub fraction: RefCell<ProgressBar>,
    pub inline_reply: RefCell<gtk::Entry>,
    pub body: RefCell<Label>,
    pub summary: RefCell<Label>,
    pub image: RefCell<Image>,
    pub basebox: RefCell<gtk::Box>,
    pub regularbox: RefCell<gtk::Box>,
    pub bodybox: RefCell<gtk::Box>,
    pub reset: AtomicBool,
    pub has_body: Cell<bool>,
    pub has_summary: Cell<bool>,
    pub has_image: Cell<bool>,
    pub has_progbar: Cell<bool>,
    pub has_inline_reply: Cell<bool>,
    pub previous_urgency: Cell<String>,
}

#[glib::object_subclass]
impl ObjectSubclass for NotificationBox {
    const NAME: &'static str = "NotificationBox";
    type Type = super::NotificationBox;
    type ParentType = gtk::Box;
}

impl ObjectImpl for NotificationBox {}

impl WidgetImpl for NotificationBox {}

impl ContainerImpl for NotificationBox {}

impl BoxImpl for NotificationBox {}

impl BinImpl for NotificationBox {}

