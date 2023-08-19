use std::{
    collections::HashMap,
    fmt::Display,
    hash::Hash,
    sync::{Arc, Mutex},
};

use dbus::{
    arg::{self, RefArg},
    blocking::Connection,
};
use gtk::glib::Sender;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
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
            expire_timeout: 0,
            urgency: Urgency::Low,
            image_path: None,
            progress: None,
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
    pub notifications: HashMap<u32, Notification>,
    pub last_notification_id: u32,
    pub do_not_disturb: bool,
    pub handle: Sender<Notification>,
}

impl NotificationWrapper {
    pub fn create(handle: Sender<Notification>) -> Self {
        Self {
            notifications: HashMap::new(),
            last_notification_id: 0,
            do_not_disturb: false,
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
    pub fn get_latest_notification(&self) -> Option<&Notification> {
        self.notifications.get(&self.last_notification_id)
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
                            notification.expire_timeout.clone(),
                            notification.urgency.clone().to_i32(),
                            notification
                                .image_path
                                .clone()
                                .unwrap_or_else(|| "".to_string()),
                            notification.progress.clone().unwrap_or_else(|| 0),
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
