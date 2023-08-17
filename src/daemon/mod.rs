use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
};

use dbus::{
    arg::{self, RefArg},
    blocking::Connection,
};
use gtk::glib::Sender;

#[derive(Clone)]
pub enum Urgency {
    Low,
    Normal,
    Urgent,
}

impl Urgency {
    fn from_i32(value: i32) -> Result<Urgency, &'static str> {
        match value {
            1 => Ok(Urgency::Low),
            2 => Ok(Urgency::Normal),
            3 => Ok(Urgency::Urgent),
            _ => Err("invalid number, only 1,2 or 3 allowed"),
        }
    }
    fn to_i32(&self) -> i32 {
        match self {
            Urgency::Low => 1,
            Urgency::Normal => 2,
            Urgency::Urgent => 3,
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

pub struct Notification {
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub hints: arg::PropMap,
    pub expire_timeout: i32,
    pub urgency: Urgency,
    pub image_path: Option<String>,
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
            hints: arg::PropMap::new(),
            expire_timeout: self.expire_timeout.clone(),
            urgency: self.urgency.clone(),
            image_path: self.image_path.clone(),
        }
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
        Self {
            app_name,
            replaces_id,
            app_icon,
            summary,
            body,
            actions,
            hints,
            expire_timeout,
            urgency: Urgency::Low,
            image_path: None,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            app_name: String::from(""),
            replaces_id: 0,
            app_icon: String::from(""),
            summary: String::from(""),
            body: String::from(""),
            actions: Vec::new(),
            hints: arg::PropMap::new(),
            expire_timeout: 0,
            urgency: Urgency::Low,
            image_path: None,
        }
    }

    pub fn print(&self) {
        print!(
            "Notification {} with summary {} from app {}\n
    Body: {}\n
    timestamp: {}\n",
            self.replaces_id, self.summary, self.app_name, self.body, self.expire_timeout,
        );
    }
}

pub struct NotificationWrapper {
    pub notifications: Vec<Notification>,
    pub id_map: HashMap<u32, i32>,
    pub do_not_disturb: bool,
    pub handle: Sender<Notification>,
}

impl NotificationWrapper {
    pub fn create(handle: Sender<Notification>) -> Self {
        Self {
            notifications: Vec::new(),
            id_map: HashMap::<u32, i32>::new(),
            do_not_disturb: false,
            handle,
        }
    }
    pub fn add_notification(&mut self, notification: &mut Notification) {
        self.id_map
            .insert(notification.replaces_id, self.notifications.len() as i32);
        let urgency = notification.hints.get("urgency");
        if urgency.is_some() {
            let urg = Urgency::from_i32(urgency.unwrap().as_i64().unwrap_or_else(|| 1) as i32);
            notification.urgency = urg.unwrap_or_else(|_| -> Urgency { Urgency::Low });
        }
        let image_path = notification.hints.get("image-path");
        if image_path.is_some() {
            notification.image_path =
                Some(image_path.unwrap().as_str().unwrap_or_default().to_string());
        }
        self.notifications.push(notification.clone());
    }
    pub fn remove_notification(&mut self, id: u32) {
        let index = self.id_map.remove(&id);
        if index.is_none() {
            return;
        }
        self.notifications.remove(index.unwrap() as usize);
    }
    pub fn clear_all_notifications(&mut self) {
        self.notifications.clear();
        self.id_map.clear();
    }
    pub fn get_all_notifications(&self) -> &Vec<Notification> {
        &self.notifications
    }
    pub fn toggle_do_not_disturb(&mut self) -> bool {
        self.do_not_disturb = !self.do_not_disturb;
        self.do_not_disturb
    }
    pub fn get_latest_notification(&self) -> Option<&Notification> {
        self.notifications.last()
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

    pub fn run(&mut self) {
        let c = Connection::new_session().unwrap();
        c.request_name("org.freedesktop.Notifications2", false, true, false)
            .unwrap();
        let mut cr = dbus_crossroads::Crossroads::new();
        let token = cr.register("org.freedesktop.Notifications2", |c| {
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
                ("reply",),
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
                    if !server.do_not_disturb {
                        server
                            .handle
                            .send(notification)
                            .expect("Failed to send notification.");
                    }
                    Ok(("ok",))
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
                        notifications.push((
                            notification.app_name.clone(),
                            notification.replaces_id.clone(),
                            notification.app_icon.clone(),
                            notification.summary.clone(),
                            notification.body.clone(),
                            notification.actions.clone(),
                            notification.urgency.clone().to_i32(),
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
        });
        cr.insert(
            "/org/freedesktop/Notifications2",
            &[token],
            self.wrapper.clone(),
        );
        cr.serve(&c).unwrap();
    }

    pub fn remove_notification(&mut self, id: u32) {
        self.wrapper.lock().unwrap().remove_notification(id);
    }

    pub fn clear_all_notifications(&mut self) {
        self.wrapper.lock().unwrap().clear_all_notifications();
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
