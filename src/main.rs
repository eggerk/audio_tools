//! Dbus interface to control the volume and display a notification to the user.

mod dbus_interface;
mod notification;
mod volume;
mod volume_control;

fn main() {
    dbus_interface::run();
}
