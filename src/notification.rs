use std::error::Error;
use std::process;
use std::str;
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};

use log::{debug, error, info};
use notify_rust::{Notification, NotificationHandle};

use crate::volume::VolumeInfo;
use crate::volume_control::Interface;

struct NotificationWrapper {
    notification_handle: Option<NotificationHandle>,
    default_summary: String,
}

impl NotificationWrapper {
    fn new(default_summary: String) -> NotificationWrapper {
        NotificationWrapper {
            notification_handle: None,
            default_summary: default_summary,
        }
    }

    fn notify(&mut self, summary: Option<&str>, body: &str) -> Result<(), Box<dyn Error>> {
        if let None = self.notification_handle {
            info!(
                "Created notification \"{}\".",
                match summary {
                    Some(s) => s,
                    None => "<no summary>",
                }
            );
            let notification = Notification::new().icon("audio-headphones").show()?;
            self.notification_handle = Some(notification);
        }

        if let Some(notification_handle) = &mut self.notification_handle {
            let summary = match summary {
                Some(s) => s,
                None => &self.default_summary,
            };
            debug!("Showing notification \"{}\".", summary);
            notification_handle.summary(summary);
            notification_handle.body(&body);
            notification_handle.update();
        }

        Ok(())
    }
}

pub struct VolumeNotification {
    notification_handle: NotificationWrapper,
}

impl VolumeNotification {
    pub fn new() -> Self {
        Self {
            notification_handle: NotificationWrapper::new(String::from("Volume")),
        }
    }

    fn build_volume_string(info: &VolumeInfo) -> (String, String) {
        const NUM_BLOCKS: i32 = 20;
        let full_blocks = info.volume * NUM_BLOCKS / 100;
        let emtpy_blocks = NUM_BLOCKS - full_blocks;

        let title = format!(
            "Volume ({}%{})",
            info.volume,
            match info.muted {
                true => ", muted",
                false => "",
            }
        );
        let character = match info.muted {
            true => '░',
            false => '█',
        };
        let body = format!(
            "{}<span color=\"grey\">{}</span>",
            (0..full_blocks).map(|_| character).collect::<String>(),
            (0..emtpy_blocks).map(|_| character).collect::<String>()
        );

        (title, body)
    }

    pub fn notify(&mut self, volume_info: &VolumeInfo) -> Result<(), Box<dyn Error>> {
        debug!(
            "Showing volumen notification ({}%, muted: {}).",
            volume_info.volume, volume_info.muted
        );
        let (title, body) = VolumeNotification::build_volume_string(&volume_info);

        self.notification_handle.notify(Some(&title), &body)
    }
}

pub struct SinkNotificaton {
    notification_handle: NotificationWrapper,
}

impl SinkNotificaton {
    pub fn new() -> Self {
        Self {
            notification_handle: NotificationWrapper::new(String::from("Audio Input")),
        }
    }

    pub fn notify_start(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Notifying about sink change start.");
        self.notification_handle.notify(None, "Changing input...")
    }

    pub fn notify(&mut self, interfaces: &Vec<Interface>) -> Result<(), Box<dyn Error>> {
        debug!("Showing sink notification.");
        let body = interfaces
            .iter()
            .map(|i| {
                if i.active {
                    format!("→ {}", i.name)
                } else {
                    format!("<span color=\"grey\">{}</span>", i.name)
                }
            })
            .collect::<Vec<String>>()
            .join("\n");

        self.notification_handle.notify(None, &body)
    }
}

pub struct SoundPlayer {
    thread: Option<JoinHandle<()>>,
    thread_running: Arc<Mutex<bool>>,
}

impl SoundPlayer {
    pub fn new() -> Self {
        SoundPlayer {
            thread: None,
            thread_running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn play_sound_if_not_used(&mut self, interface: &Interface) {
        debug!("Request to play sound. Interface state: {}", interface.state);
        if interface.state != "RUNNING" {
            debug!("Interface is NOT running. Playing a sound.");
            let index = interface.index.to_string();
            if let Ok(thread_running) = self.thread_running.lock() {
                if *thread_running {
                    debug!("A sound is already playing -- aborting.");
                    return;
                }
            }

            let thread_running_copy = self.thread_running.clone();
            self.thread = Some(spawn(move || {
                debug!("Playing sound now.");
                if let Err(e) = process::Command::new("paplay")
                    .args(&[
                        "-d",
                        &index,
                        "/usr/share/sounds/freedesktop/stereo/message.oga",
                    ])
                    .output()
                {
                    error!("Failed to play sound: {}", e);
                }

                if let Ok(mut thread_running) = thread_running_copy.lock() {
                    *thread_running = false;
                }
            }));

            if let Ok(mut thread_running) = self.thread_running.lock() {
                *thread_running = true;
            }
        }
    }
}
