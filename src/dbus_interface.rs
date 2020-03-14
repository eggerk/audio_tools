use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use dbus::blocking::LocalConnection;
use dbus::tree::{Factory, MTFnMut, MethodInfo};

use crate::notification::{play_sound_if_not_used, SinkNotificaton, VolumeNotification};
use crate::volume::VolumeInfo;
use crate::volume_control::VolumeControl;

pub fn run() {
    let mut dbus_connection = LocalConnection::new_session().expect("Couldn't connect to dbus.");
    dbus_connection
        .request_name("ch.eggerk.volume_notification", false, true, false)
        .expect("Failed to requst dbus name.");

    let f = Factory::new_fnmut::<()>();

    let volume_notification = RefCell::new(VolumeNotification::new());
    let volume_control = Rc::new(RefCell::new(
        VolumeControl::new().expect("Failed to create volume notification."),
    ));
    let sink_notification = RefCell::new(SinkNotificaton::new());

    let play_sound_if_not_used_closure = Rc::new(move |volume_control: &VolumeControl| {
        if let Some(active_interface) = volume_control.get_active_interface() {
            if let Err(e) = play_sound_if_not_used(active_interface) {
                eprintln!("Failed to play sound: {}", e);
            }
        }
    });

    let volume_control_copy = Rc::clone(&volume_control);
    let play_sound_if_not_used_closure_copy = Rc::clone(&play_sound_if_not_used_closure);
    let notify_callback_base = Rc::new(move |m: &MethodInfo<'_, MTFnMut, ()>| {
        let mut notification = volume_notification.borrow_mut();
        match VolumeInfo::get_volume() {
            Err(e) => eprintln!("Failed to get volume status: {}", e),
            Ok(volume) => {
                notification
                    .notify(&volume)
                    .unwrap_or_else(|e| eprintln!("Failed to notify: {}", e));
            }
        }

        play_sound_if_not_used_closure_copy(&volume_control_copy.borrow());

        let mret = m.msg.method_return();
        Ok(vec![mret])
    });

    let notification_callback_copy = Rc::clone(&notify_callback_base);
    let notify_callback = move |m: &MethodInfo<'_, MTFnMut, ()>| notification_callback_copy(m);

    let volume_control_copy = Rc::clone(&volume_control);
    let volume_change_callback_base = Rc::new(move |amount| {
        if let Err(e) = volume_control_copy.borrow_mut().change_volume(amount) {
            eprintln!("Failed to change volume: {}", e);
        }
    });

    let notification_callback_copy = Rc::clone(&notify_callback_base);
    let volume_change_callback_base_copy = Rc::clone(&volume_change_callback_base);
    let raise_callback = move |m: &MethodInfo<'_, MTFnMut, ()>| {
        volume_change_callback_base_copy(5);
        notification_callback_copy(m)
    };

    let notification_callback_copy = Rc::clone(&notify_callback_base);
    let volume_change_callback_base_copy = Rc::clone(&volume_change_callback_base);
    let lower_callback = move |m: &MethodInfo<'_, MTFnMut, ()>| {
        volume_change_callback_base_copy(-5);
        notification_callback_copy(m)
    };

    let notification_callback_copy = Rc::clone(&notify_callback_base);
    let volume_control_copy = Rc::clone(&volume_control);
    let mute_callback = move |m: &MethodInfo<'_, MTFnMut, ()>| {
        volume_control_copy
            .borrow_mut()
            .toggle_mute()
            .unwrap_or_else(|e| eprintln!("Failed to toggle mute: {}", e));
        notification_callback_copy(m)
    };

    let cycle_interface_callback = move |m: &MethodInfo<'_, MTFnMut, ()>| {
        let mret = m.msg.method_return();
        if let Err(e) = sink_notification.borrow_mut().notify_start() {
            eprintln!("Failed to send the notification: {}", e);
        }
        let mut volume_control = volume_control.borrow_mut();
        if let Err(e) = volume_control.cycle_through_interfaces() {
            eprintln!("Failed to change input: {}", e);
        } else {
            let available_inputs = volume_control.get_available_interfaces();
            match available_inputs {
                Ok(list) => {
                    if let Err(e) = sink_notification.borrow_mut().notify(list) {
                        eprintln!("Failed to notify: {}", e);
                    }
                }
                Err(e) => eprintln!("Failed to list available inputs: {}", e),
            }
        }

        play_sound_if_not_used_closure(&volume_control);

        Ok(vec![mret])
    };

    let tree = f
        .tree(())
        .add(
            f.object_path("/volume_control", ()).introspectable().add(
                f.interface("ch.eggerk.volume_notification", ())
                    .add_m(f.method("VolumeNotify", (), notify_callback))
                    .add_m(f.method("VolumeRaise", (), raise_callback))
                    .add_m(f.method("VolumeLower", (), lower_callback))
                    .add_m(f.method("VolumeToggleMute", (), mute_callback))
                    .add_m(f.method("CycleInputs", (), cycle_interface_callback)),
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
