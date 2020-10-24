use std::process;
use std::str;

use failure::{bail, Error};
use log::{debug, error};
use notify_rust::Notification;

use crate::volume::VolumeInfo;
use crate::volume_control::Interface;

struct NotificationWrapper {
    id: Option<u32>,
    default_summary: String,
}

impl NotificationWrapper {
    fn new(id: Option<u32>, default_summary: String) -> NotificationWrapper {
        NotificationWrapper {
            id: id,
            default_summary: default_summary,
        }
    }

    fn notify(&mut self, summary: Option<&str>, body: &str) -> Result<(), Error> {
        let mut notification = Notification::new().icon("audio-headphones").finalize();

        let summary = match summary {
            Some(s) => s,
            None => &self.default_summary,
        };
        debug!("Showing notification \"{}\".", summary);
        let mut notification = notification.summary(summary).body(&body).finalize();
        let notification = match self.id {
            Some(id) => notification.id(id).finalize(),
            None => notification,
        };
        match notification.show() {
            Err(e) => bail!("{:?}", e),
            Ok(handle) => {
                self.id = Some(handle.id());
                Ok(())
            }
        }
    }

    fn get_id(&self) -> Option<u32> {
        self.id
    }
}

pub struct VolumeNotification {
    notification_handle: NotificationWrapper,
}

impl VolumeNotification {
    pub fn new(id: Option<u32>) -> Self {
        Self {
            notification_handle: NotificationWrapper::new(id, String::from("Volume")),
        }
    }

    pub fn get_id(&self) -> Option<u32> {
        self.notification_handle.get_id()
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

    pub fn notify(&mut self, volume_info: &VolumeInfo) -> Result<(), Error> {
        debug!(
            "Showing volume notification ({}%, muted: {}).",
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
    pub fn new(id: Option<u32>) -> Self {
        Self {
            notification_handle: NotificationWrapper::new(id, String::from("Audio Input")),
        }
    }

    pub fn get_id(&self) -> Option<u32> {
        self.notification_handle.get_id()
    }

    pub fn notify_start(&mut self) -> Result<(), Error> {
        debug!("Notifying about sink change start.");
        self.notification_handle.notify(None, "Changing input...")
    }

    pub fn notify(&mut self, interfaces: &Vec<Interface>) -> Result<(), Error> {
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
    play_sound_process: Option<process::Child>,
}

impl SoundPlayer {
    pub fn new() -> Self {
        SoundPlayer {
            play_sound_process: None,
        }
    }

    pub fn play_sound(&mut self, interface: &Interface, always_play_sound: bool) {
        debug!(
            "Request to play sound. Interface state: {}",
            interface.state
        );
        if always_play_sound || interface.state != "RUNNING" {
            debug!("Interface is NOT running. Playing a sound.");
            let index = interface.index.to_string();
            if let Some(play_sound_process) = &mut self.play_sound_process {
                match play_sound_process.try_wait() {
                    Ok(Some(_)) => self.play_sound_process = None,
                    Ok(None) => {
                        debug!(
                            "Play sound process is still in progress. Not starting another one."
                        );
                        return;
                    }
                    Err(e) => {
                        error!("Failed to wait for the play sound process: {}", e);
                        return;
                    }
                }
            }

            match process::Command::new("paplay")
                .args(&[
                    "-d",
                    &index,
                    "/usr/share/sounds/freedesktop/stereo/message.oga",
                ])
                .spawn()
            {
                Ok(process) => self.play_sound_process = Some(process),
                Err(e) => error!("Failed to start play sound process: {}", e),
            }
        }
    }
}
