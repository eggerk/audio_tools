use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use dbus::blocking::LocalConnection;
use dbus::tree::{Factory, MTFnMut, MethodInfo, MethodResult};

use crate::notification::{SinkNotificaton, SoundPlayer, VolumeNotification};
use crate::volume::VolumeInfo;
use crate::volume_control::VolumeControl;

struct DbusInterface {
    volume_control: VolumeControl,
    volume_notification: VolumeNotification,
    sink_notification: SinkNotificaton,
    sound_player: SoundPlayer,
}

impl DbusInterface {
    fn new() -> Self {
        DbusInterface {
            volume_control: VolumeControl::new().expect("Failed to create volume notification."),
            volume_notification: VolumeNotification::new(),
            sink_notification: SinkNotificaton::new(),
            sound_player: SoundPlayer::new(),
        }
    }

    pub fn show_volume_notification(&mut self) {
        match VolumeInfo::get_volume() {
            Err(e) => eprintln!("Failed to get volume status: {}", e),
            Ok(volume) => {
                self.volume_notification
                    .notify(&volume)
                    .unwrap_or_else(|e| eprintln!("Failed to notify: {}", e));
            }
        }
        self.play_sound_if_not_used();
    }

    fn play_sound_if_not_used(&mut self) {
        match VolumeControl::new() {
            Ok(vol) => self.volume_control = vol,
            Err(e) => eprintln!("Failed to get volume info: {}", e),
        }
        if let Some(active_interface) = &self.volume_control.active_interface {
            self.sound_player.play_sound_if_not_used(&active_interface);
        }
    }

    pub fn change_volume(&mut self, m: &MethodInfo<'_, MTFnMut, ()>, amount: i32) -> MethodResult {
        self.volume_control
            .change_volume(amount)
            .unwrap_or_else(|e| eprintln!("Failed to change volume: {}", e));
        self.show_volume_notification();
        Ok(vec![m.msg.method_return()])
    }

    pub fn toggle_mute(&mut self, m: &MethodInfo<'_, MTFnMut, ()>) -> MethodResult {
        self.volume_control
            .toggle_mute()
            .unwrap_or_else(|e| eprintln!("Failed to toggle mute: {}", e));
        self.show_volume_notification();

        Ok(vec![m.msg.method_return()])
    }

    pub fn cycle_through_interfaces(&mut self, m: &MethodInfo<'_, MTFnMut, ()>) -> MethodResult {
        self.sink_notification
            .notify_start()
            .unwrap_or_else(|e| eprintln!("Failed to send the notification: {}", e));
        match self.volume_control.cycle_through_interfaces() {
            Err(e) => eprintln!("Failed to change input: {}", e),
            Ok(_) => {
                let available_inputs = self.volume_control.get_available_interfaces();
                match available_inputs {
                    Ok(list) => self
                        .sink_notification
                        .notify(list)
                        .unwrap_or_else(|e| eprintln!("Failed to notify: {}", e)),
                    Err(e) => eprintln!("Failed to list available inputs: {}", e),
                }
            }
        };
        self.play_sound_if_not_used();
        Ok(vec![m.msg.method_return()])
    }
}

pub fn run() {
    let mut dbus_connection = LocalConnection::new_session().expect("Couldn't connect to dbus.");
    dbus_connection
        .request_name("ch.eggerk.volume_notification", false, true, false)
        .expect("Failed to requst dbus name.");

    let f = Factory::new_fnmut::<()>();

    let interface = Rc::new(RefCell::new(DbusInterface::new()));

    // Make a few copies to pass the callbacks.
    let interface_lower = Rc::clone(&interface);
    let interface_raise = Rc::clone(&interface);
    let interface_mute = Rc::clone(&interface);
    let interface_cycle = Rc::clone(&interface);

    let tree = f
        .tree(())
        .add(
            f.object_path("/volume_control", ()).introspectable().add(
                f.interface("ch.eggerk.volume_notification", ())
                    .add_m(f.method("VolumeRaise", (), move |m| {
                        interface_raise.borrow_mut().change_volume(m, 5)
                    }))
                    .add_m(f.method("VolumeLower", (), move |m| {
                        interface_lower.borrow_mut().change_volume(m, -5)
                    }))
                    .add_m(f.method("VolumeToggleMute", (), move |m| {
                        interface_mute.borrow_mut().toggle_mute(m)
                    }))
                    .add_m(f.method("CycleInputs", (), move |m| {
                        interface_cycle.borrow_mut().cycle_through_interfaces(m)
                    })),
            ),
        )
        .add(f.object_path("/", ()).introspectable());
    tree.start_receive(&dbus_connection);

    loop {
        dbus_connection
            .process(Duration::from_millis(1000))
            .unwrap();
    }
}
