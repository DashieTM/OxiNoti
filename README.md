# OxiNoti
A work in progress notification daemon made with rust and gtk.\
Can also be used in tandem with [OxiDash](https://git.dashie.org/DashieTM/OxiDash), a notification center also made with rust and gtk.

![Screenshot](notification.png?raw=true)

## features
### Supported hints:
- images via image-path
- images via raw bytes
- urgency

### Dbus functions:
- ToggleNotificationCenter: disables showing of notifications when notification center is open
- DoNotDisturb: disables sending of notifications when set, note: with dnd_override in the config file, notifications with high enough urgency can circumvent this.
- GetServerInformation: name, vendor, version, spec_version
- GetCapabilities: returns server capabilities
- RemoveAllNotifications: removes all notification from persistence
- GetAllNotification: returns a vector of all currently held notifications
- CloseNotification: removes specific notification from persistence
- Notify: send notification, note: also sends notification to notification center if available -> dbus address: org.freedesktop.NotificationCenter

Notify, GetServerInformation, CloseNotification and GetCapabilities are standardized from [freedesktop.org](https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html#hints)\
The rest are additions to it, which are specific for the notification center.

### CLI parameters:
- --config: specify a path to a toml config file
- --css: specify a path to a css style sheet

### toml config:
        timeout = 3       # this sets the timeout for the notification -> how long it stays
        dnd_override = 2  # this is the minimum amount of urgency that a notification needs to be shown despite do not disturb
                          # note, values for dnd_override are: 0 for low, essentially disables dnd, 1 for normal, 2 for critical, any other value will block notifications no matter the urgency during dnd

### CSS

Base gtk CSS can be used to theme OxiNoti, an example can be found in the repository.



## notes
- WIP, don't use this for regular use yet
  - testing welcome, kinda works by now
- not sure if this is efficient, never had any proper experience with background tasks and/or async/parallel programming
  - seems to use about 40mb of ram, hope that is fine

