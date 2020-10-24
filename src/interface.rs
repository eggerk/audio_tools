use failure::Error;
use log::error;

use crate::notification::{SinkNotificaton, SoundPlayer, VolumeNotification};
use crate::volume::VolumeInfo;
use crate::volume_control::VolumeControl;

pub struct Interface {
    volume_control: VolumeControl,
    volume_notification: VolumeNotification,
    sink_notification: SinkNotificaton,
    sound_player: SoundPlayer,
}

impl Interface {
    pub fn new(volume_notification_id: Option<u32>, sink_notification_id: Option<u32>) -> Self {
        Interface {
            volume_control: VolumeControl::new().expect("Failed to create volume notification."),
            volume_notification: VolumeNotification::new(volume_notification_id),
            sink_notification: SinkNotificaton::new(sink_notification_id),
            sound_player: SoundPlayer::new(),
        }
    }

    pub fn get_notification_ids(&self) -> (Option<u32>, Option<u32>) {
        (self.volume_notification.get_id(), self.sink_notification.get_id())
    }

    pub fn show_volume_notification(&mut self, always_play_sound: bool) -> Result<(), Error> {
        match VolumeInfo::get_volume() {
            Err(e) => error!("Failed to get volume status: {}", e),
            Ok(volume) => {
                self.volume_notification
                    .notify(&volume)
                    .unwrap_or_else(|e| eprintln!("Failed to notify: {}", e));
            }
        }
        self.play_sound(always_play_sound)?;
        Ok(())
    }

    fn play_sound(&mut self, always_play_sound: bool) -> Result<(), Error> {
        self.volume_control = VolumeControl::new()?;
        if let Some(active_interface) = &self.volume_control.active_interface {
            self.sound_player
                .play_sound(&active_interface, always_play_sound);
        }
        Ok(())
    }

    pub fn change_volume(&mut self, amount: i32) -> Result<(), Error> {
        self.volume_control
            .change_volume(amount)
            .unwrap_or_else(|e| error!("Failed to change volume: {}", e));
        self.show_volume_notification(false)?;
        Ok(())
    }

    pub fn toggle_mute(&mut self) -> Result<(), Error> {
        self.volume_control
            .toggle_mute()
            .unwrap_or_else(|e| error!("Failed to toggle mute: {}", e));
        self.show_volume_notification(false)?;

        Ok(())
    }

    pub fn cycle_through_interfaces(&mut self) -> Result<(), Error> {
        self.sink_notification
            .notify_start()
            .unwrap_or_else(|e| error!("Failed to send the notification: {}", e));
        match self.volume_control.cycle_through_interfaces() {
            Err(e) => error!("Failed to change input: {}", e),
            Ok(_) => {
                let available_inputs = self.volume_control.get_available_interfaces();
                match available_inputs {
                    Ok(list) => self
                        .sink_notification
                        .notify(list)
                        .unwrap_or_else(|e| error!("Failed to notify: {}", e)),
                    Err(e) => error!("Failed to list available inputs: {}", e),
                }
            }
        };
        self.play_sound(false)?;
        Ok(())
    }
}
