//! Dbus interface to control the volume and display a notification to the user.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use simplelog::*;

mod dbus_interface;
mod notification;
mod volume;
mod volume_control;

fn setup_log() -> Result<(), Box<dyn Error>> {
    let home_folder = env::var("HOME")?;
    let log_file = Path::new(&home_folder).join(".config/audio_tools.log");
    // fs::create_dir_all(log_file.parent().unwrap())?;

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap(),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            fs::File::create(log_file)?,
        ),
    ])?;

    Ok(())
}

fn main() {
    setup_log().unwrap_or_else(|e| eprintln!("Failed to initialize logging: {}", e));
    dbus_interface::run();
}
