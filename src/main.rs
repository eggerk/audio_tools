use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use failure::{bail, format_err, Error};
use log::info;
use simplelog::*;

mod interface;
mod notification;
mod volume;
mod volume_control;

use crate::interface::Interface;

fn get_data_file_path() -> Result<PathBuf, Error> {
    let p = env::var("XDG_RUNTIME_DIR")?;
    Ok(Path::new(&p).join("audio_tools_notification_id"))
}

fn load_data_from_file() -> Result<(Option<u32>, Option<u32>), Error> {
    let path = match get_data_file_path() {
        Err(_) => {
            println!("Unknown path to load from!");
            return Ok((None, None));
        }
        Ok(p) => p,
    };
    let mut s = String::new();
    match File::open(&path) {
        Err(_) => println!("Could not open file at {:?}!", path),
        Ok(mut file) => match file.read_to_string(&mut s) {
            Err(_) => println!("Could not read file at {:?}!", path),
            Ok(_) => {}
        },
    };

    let mut s = s.split(';');
    let volume_notification_id = match s
        .next()
        .ok_or_else(|| format_err!("No data could be loaded."))?
        .parse::<u32>()
    {
        Ok(id) => Some(id),
        Err(_) => None,
    };
    let sink_notification_id = match s
        .next()
        .ok_or_else(|| format_err!("No data could be loaded."))?
        .parse::<u32>()
    {
        Ok(id) => Some(id),
        Err(_) => None,
    };
    Ok((volume_notification_id, sink_notification_id))
}

fn write_data_to_file(volume_id: Option<u32>, sink_id: Option<u32>) -> Result<(), Error> {
    let p = get_data_file_path()?;
    let volume_id = match volume_id {
        Some(id) => format!("{}", id),
        None => "".to_string(),
    };
    let sink_id = match sink_id {
        Some(id) => format!("{}", id),
        None => "".to_string(),
    };
    File::create(p)?.write_all(format!("{};{}", volume_id, sink_id).as_bytes())?;
    Ok(())
}

#[derive(PartialEq)]
enum CommandType {
    NextInput,
    VolumeLower,
    VolumeRaise,
    VolumeToggleMute,
    VolumeNotification,
}

fn parse_args() -> Result<CommandType, Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        bail!("Not enough arguments!");
    }

    let ret = match &args[1].as_str() {
        &"next_input" => CommandType::NextInput,
        &"lower" => CommandType::VolumeLower,
        &"raise" => CommandType::VolumeRaise,
        &"mute" => CommandType::VolumeToggleMute,
        &"volume_notification" => CommandType::VolumeNotification,
        command => {
            bail!("Unknown command \"{}\"!", command);
        }
    };
    Ok(ret)
}

fn setup_log() -> Result<(), Error> {
    let home_folder = env::var("HOME")?;
    let log_file = Path::new(&home_folder).join(".config/audio_tools.log");
    // fs::create_dir_all(log_file.parent().unwrap())?;

    CombinedLogger::init(vec![
        // TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap(),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            fs::File::create(log_file)?,
        ),
    ])?;

    Ok(())
}

fn main() -> Result<(), Error> {
    setup_log()?;
    let (volume_notification_id, sink_notification_id) =
        load_data_from_file().unwrap_or((None, None));

    let mut interface = Interface::new(volume_notification_id, sink_notification_id);

    let command = parse_args()?;
    match command {
        CommandType::NextInput => {
            info!("Received: CycleInputs");
            interface.cycle_through_interfaces()
        }
        CommandType::VolumeLower => {
            info!("Received: VolumeLower");
            interface.change_volume(-5)
        }
        CommandType::VolumeRaise => {
            info!("Received: VolumeRaise");
            interface.change_volume(5)
        }
        CommandType::VolumeToggleMute => {
            info!("Received: VolumeToggleMute");
            interface.toggle_mute()
        }
        CommandType::VolumeNotification => {
            info!("Received: ShowVolume");
            interface.show_volume_notification(true)
        }
    }?;

    let (volume_notification_id, sink_notification_id) = interface.get_notification_ids();
    write_data_to_file(volume_notification_id, sink_notification_id)?;

    Ok(())
}
