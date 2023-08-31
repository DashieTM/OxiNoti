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

use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    hash::Hash,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use dbus::{
    arg::{self, cast, prop_cast, RefArg},
    blocking::Connection,
};
use dbus_crossroads::Context;
use gtk::glib::Sender;

use crate::ui::utils::config::Config;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct ImageData {
    pub width: i32,
    pub height: i32,
    pub rowstride: i32,
    pub has_alpha: bool,
    pub bits_per_sample: i32,
    pub channels: i32,
    pub data: Vec<u8>,
}

impl ImageData {
    pub fn empty() -> Self {
        Self {
            width: -1,
            height: -1,
            rowstride: -1,
            has_alpha: false,
            bits_per_sample: -1,
            channels: -1,
            data: Vec::new(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Urgency {
    Low,
    Normal,
    Urgent,
}

impl Urgency {
    fn from_i32(value: i32) -> Result<Urgency, &'static str> {
        match value {
            0 => Ok(Urgency::Low),
            1 => Ok(Urgency::Normal),
            2 => Ok(Urgency::Urgent),
            _ => Err("invalid number, only 1,2 or 3 allowed"),
        }
    }
    fn to_i32(&self) -> i32 {
        match self {
            Urgency::Low => 0,
            Urgency::Normal => 1,
            Urgency::Urgent => 2,
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            Urgency::Low => "NotificationLow",
            Urgency::Normal => "NotificationNormal",
            Urgency::Urgent => "NotificationUrgent",
        }
    }
}

impl Display for Urgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_i32())
    }
}

#[derive(Eq, PartialEq, PartialOrd, Ord)]
pub struct Notification {
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub expire_timeout: i32,
    pub urgency: Urgency,
    pub image_path: Option<String>,
    pub progress: Option<i32>,
    pub image_data: Option<ImageData>,
}

impl Clone for Notification {
    fn clone(&self) -> Self {
        Self {
            app_name: self.app_name.clone(),
            replaces_id: self.replaces_id.clone(),
            app_icon: self.app_icon.clone(),
            summary: self.summary.clone(),
            body: self.body.clone(),
            actions: self.actions.clone(),
            expire_timeout: self.expire_timeout.clone(),
            urgency: self.urgency.clone(),
            image_path: self.image_path.clone(),
            progress: self.progress.clone(),
            image_data: self.image_data.clone(),
        }
    }
}

impl Hash for Notification {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.app_name.hash(state);
        self.replaces_id.hash(state);
        self.app_icon.hash(state);
        self.summary.hash(state);
        self.body.hash(state);
        self.actions.hash(state);
        self.expire_timeout.hash(state);
        self.urgency.to_i32().hash(state);
        self.image_path.hash(state);
        self.progress.hash(state);
    }
}

impl Notification {
    pub fn create(
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: arg::PropMap,
        expire_timeout: i32,
    ) -> Self {
        let mut urgency = Urgency::Low;
        let urgency_opt = hints.get("urgency");
        if urgency_opt.is_some() {
            let urg = Urgency::from_i32(urgency_opt.unwrap().as_i64().unwrap_or_else(|| 1) as i32);
            urgency = urg.unwrap_or_else(|_| -> Urgency { Urgency::Low });
        }
        let mut image_path = None;
        let image_path_opt = hints.get("image-path");
        if image_path_opt.is_some() {
            image_path = Some(
                image_path_opt
                    .unwrap()
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            );
        }
        for action in actions.iter() {
            println!("action: {}", action);
        }
        let mut image_data = None;
        let image_data_opt: Option<&VecDeque<Box<dyn RefArg>>> = prop_cast(&hints, "image-data");
        if image_data_opt.is_some() {
            let raw = image_data_opt.unwrap();
            image_data = Some(ImageData {
                width: *cast::<i32>(&raw[0]).unwrap(),
                height: *cast::<i32>(&raw[1]).unwrap(),
                rowstride: *cast::<i32>(&raw[2]).unwrap(),
                has_alpha: *cast::<bool>(&raw[3]).unwrap(),
                bits_per_sample: *cast::<i32>(&raw[4]).unwrap(),
                channels: *cast::<i32>(&raw[5]).unwrap(),
                data: cast::<Vec<u8>>(&raw[6]).unwrap().clone(),
            });
        }
        let mut progress = None;
        let progress_opt = hints.get("progress");
        if progress_opt.is_some() {
            progress = Some(
                progress_opt
                    .unwrap()
                    .as_i64()
                    .unwrap_or_else(|| -1)
                    .clamp(-1, 100) as i32,
            );
        }
        Self {
            app_name,
            replaces_id,
            app_icon,
            summary,
            body,
            actions,
            expire_timeout,
            urgency,
            image_path,
            progress,
            image_data,
        }
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        print!(
            "Notification {} with summary {} from app {}\n
    Body: {}\n
    timestamp: {}\n,
    image-path: {}\n,
    icon: {}\n",
            self.replaces_id,
            self.summary,
            self.app_name,
            self.body,
            self.expire_timeout,
            self.app_icon,
            self.image_path
                .clone()
                .unwrap_or_else(|| "nopic".to_string())
        );
    }
}

pub struct NotificationWrapper {
    pub notifications: HashMap<u32, Notification>,
    pub last_notification_id: u32,
    pub do_not_disturb: bool,
    pub notification_center: bool,
    pub handle: Sender<Notification>,
}

impl NotificationWrapper {
    pub fn create(handle: Sender<Notification>) -> Self {
        Self {
            notifications: HashMap::new(),
            last_notification_id: 0,
            do_not_disturb: false,
            notification_center: false,
            handle,
        }
    }
    pub fn add_notification(&mut self, notification: &mut Notification) {
        self.notifications
            .insert(notification.replaces_id, notification.clone());
        self.last_notification_id = notification.replaces_id;
    }
    pub fn remove_notification(&mut self, id: u32) {
        self.notifications.remove(&id);
    }
    pub fn clear_all_notifications(&mut self) {
        self.notifications.clear();
    }
    pub fn get_all_notifications(&self) -> Vec<Notification> {
        let mut notifications = Vec::new();
        for notification in self.notifications.values().cloned() {
            notifications.push(notification);
        }
        notifications
    }
    pub fn toggle_do_not_disturb(&mut self) -> bool {
        self.do_not_disturb = !self.do_not_disturb;
        self.do_not_disturb
    }
    pub fn toggle_notification_center(&mut self) -> bool {
        self.notification_center = !self.notification_center;
        self.notification_center
    }
}

pub struct NotificationServer {
    wrapper: Arc<Mutex<NotificationWrapper>>,
}

impl NotificationServer {
    pub fn create(handle: Sender<Notification>) -> Self {
        Self {
            wrapper: Arc::new(Mutex::new(NotificationWrapper::create(handle))),
        }
    }

    pub fn run(&mut self, config: Arc<Config>) {
        let c = Connection::new_session().unwrap();
        c.request_name("org.freedesktop.Notifications", false, true, false)
            .unwrap();
        let mut cr = dbus_crossroads::Crossroads::new();
        let token = cr.register("org.freedesktop.Notifications", |c| {
            let action_invoked = c
                .signal::<(u32, String), _>("ActionInvoked", ("id", "action_key"))
                .msg_fn();
            let inline_replied = c
                .signal::<(u32, String), _>("NotificationReplied", ("id", "text"))
                .msg_fn();
            c.method(
                "Notify",
                (
                    "app_name",
                    "replaces_id",
                    "app_icon",
                    "summary",
                    "body",
                    "actions",
                    "hints",
                    "expire_timeout",
                ),
                ("id",),
                move |_,
                      serverref: &mut Arc<Mutex<NotificationWrapper>>,
                      (
                    app_name,
                    replaces_id,
                    app_icon,
                    summary,
                    body,
                    actions,
                    hints,
                    expire_timeout,
                ): (
                    String,
                    u32,
                    String,
                    String,
                    String,
                    Vec<String>,
                    arg::PropMap,
                    i32,
                )| {
                    let mut notification = Notification::create(
                        app_name,
                        replaces_id,
                        app_icon,
                        summary,
                        body,
                        actions,
                        hints,
                        expire_timeout,
                    );
                    notification.print();
                    let mut server = serverref.lock().unwrap();
                    server.add_notification(&mut notification);
                    if urgency_should_ignore_dnd(
                        server.do_not_disturb,
                        config.dnd_override,
                        &notification.urgency,
                    ) && !server.notification_center
                    {
                        server
                            .handle
                            .send(notification)
                            .expect("Failed to send notification.");
                    } else {
                        thread::spawn(move || {
                            let conn = Connection::new_session().unwrap();
                            let proxy = conn.with_proxy(
                                "org.freedesktop.NotificationCenter",
                                "/org/freedesktop/NotificationCenter",
                                Duration::from_millis(1000),
                            );
                            let raw_data: ImageData;
                            if notification.image_data.is_some() {
                                raw_data = notification.image_data.clone().unwrap();
                            } else {
                                raw_data = ImageData::empty();
                            }
                            let image_data = (
                                raw_data.width,
                                raw_data.height,
                                raw_data.rowstride,
                                raw_data.has_alpha,
                                raw_data.bits_per_sample,
                                raw_data.channels,
                                raw_data.data,
                            );
                            let _: Result<(), dbus::Error> = proxy.method_call(
                                "org.freedesktop.NotificationCenter",
                                "Notify",
                                (
                                    notification.app_name,
                                    notification.replaces_id,
                                    notification.app_icon,
                                    notification.summary,
                                    notification.body,
                                    notification.actions,
                                    notification.expire_timeout,
                                    notification.urgency.to_i32(),
                                    notification.image_path.unwrap_or_else(|| "".to_string()),
                                    notification.progress.unwrap_or_else(|| -1),
                                    image_data,
                                ),
                            );
                        });
                    }
                    Ok((replaces_id,))
                },
            );
            c.method(
                "CloseNotification",
                ("id",),
                ("response",),
                move |_, serverref: &mut Arc<Mutex<NotificationWrapper>>, (id,): (u32,)| {
                    serverref.lock().unwrap().remove_notification(id);
                    Ok(("ok",))
                },
            );
            c.method(
                "GetAllNotifications",
                (),
                ("notifications",),
                move |_, serverref: &mut Arc<Mutex<NotificationWrapper>>, ()| {
                    let mut notifications = Vec::new();
                    for notification in serverref.lock().unwrap().get_all_notifications().iter() {
                        let raw_data: ImageData;
                        if notification.image_data.is_some() {
                            raw_data = notification.image_data.clone().unwrap();
                        } else {
                            raw_data = ImageData::empty();
                        }
                        let image_data = (
                            raw_data.width,
                            raw_data.height,
                            raw_data.rowstride,
                            raw_data.has_alpha,
                            raw_data.bits_per_sample,
                            raw_data.channels,
                            raw_data.data,
                        );
                        notifications.push((
                            notification.app_name.clone(),
                            notification.replaces_id.clone(),
                            notification.app_icon.clone(),
                            notification.summary.clone(),
                            notification.body.clone(),
                            notification.actions.clone(),
                            notification.expire_timeout.clone(),
                            notification.urgency.clone().to_i32(),
                            notification
                                .image_path
                                .clone()
                                .unwrap_or_else(|| "".to_string()),
                            notification.progress.clone().unwrap_or_else(|| -1),
                            image_data,
                        ));
                    }
                    Ok((notifications,))
                },
            );
            c.method(
                "RemoveAllNotifications",
                (),
                ("response",),
                move |_, serverref: &mut Arc<Mutex<NotificationWrapper>>, ()| {
                    serverref.lock().unwrap().clear_all_notifications();
                    Ok(("ok",))
                },
            );
            c.method(
                "GetServerInformation",
                (),
                ("name", "vendor", "version", "spec_version"),
                move |_, _, ()| {
                    let name = "Oxidash";
                    let vendor = "dashie";
                    let version = "0";
                    let spec_version = "wat";
                    Ok((name, vendor, version, spec_version))
                },
            );
            c.method("GetCapabilities", (), ("capabilities",), move |_, _, ()| {
                Ok((get_capabilities(),))
            });
            c.method(
                "DoNotDisturb",
                (),
                ("status",),
                move |_, serverref: &mut Arc<Mutex<NotificationWrapper>>, ()| {
                    let result = serverref.lock().unwrap().toggle_do_not_disturb();
                    Ok((result,))
                },
            );
            c.method(
                "ToggleNotificationCenter",
                (),
                ("result",),
                move |_, serverref: &mut Arc<Mutex<NotificationWrapper>>, ()| {
                    let res = serverref.lock().unwrap().toggle_notification_center();
                    Ok((res,))
                },
            );
            c.method(
                "InvokeAction",
                ("id", "action"),
                (),
                move |ctx: &mut Context, _, (id, action): (u32, String)| {
                    let signal = action_invoked(ctx.path(), &(id, action));
                    ctx.push_msg(signal);
                    Ok(())
                },
            );
            c.method(
                "InlineReply",
                ("id", "text"),
                (),
                move |ctx: &mut Context, _, (id, text): (u32, String)| {
                    let signal = inline_replied(ctx.path(), &(id, text));
                    ctx.push_msg(signal);
                    Ok(())
                },
            );
        });
        cr.insert(
            "/org/freedesktop/Notifications",
            &[token],
            self.wrapper.clone(),
        );
        cr.serve(&c).unwrap();
    }
}

pub fn get_capabilities() -> Vec<String> {
    [
        "action-icons".to_string(),
        "actions".to_string(),
        "body-hyprlinks".to_string(),
        "body-images".to_string(),
        "body-markup".to_string(),
        "icon-static".to_string(),
        "persistence".to_string(),
    ]
    .into()
}

fn urgency_should_ignore_dnd(
    dnd_enabled: bool,
    dnd_ignore_threshold: i32,
    urgency: &Urgency,
) -> bool {
    if !dnd_enabled {
        return true;
    }
    let necessary_urgency = match dnd_ignore_threshold {
        0 => Some(Urgency::Low),
        1 => Some(Urgency::Normal),
        2 => Some(Urgency::Urgent),
        _ => None,
    };
    if necessary_urgency.is_none() || urgency < &necessary_urgency.unwrap() {
        return false;
    }
    true
}
