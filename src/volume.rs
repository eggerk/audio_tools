use std::error::Error;
use std::process;
use std::str;

use regex::Regex;

pub struct VolumeInfo {
    pub volume: i32,
    pub muted: bool,
}

impl VolumeInfo {
    pub fn get_volume() -> Result<Self, Box<dyn Error>> {
        let volume = process::Command::new("amixer")
            .args(&["-D", "pulse", "sget", "Master"])
            .output()?;
        if !volume.status.success() {
            eprintln!("{}", str::from_utf8(&volume.stderr).unwrap());
        }
        let amixer_output = str::from_utf8(&volume.stdout)?;
        let re = Regex::new(r"\[([0-9]+)%\] \[([A-Za-z]+)\]")?;
        for cap in re.captures_iter(amixer_output) {
            if cap.len() == 3 {
                let volume: i32 = cap[1].parse()?;
                let muted = match &cap[2] {
                    "on" => false,
                    "off" => true,
                    &_ => return Err(String::from("Error parsing mute status.").into()),
                };

                return Ok(Self { volume, muted });
            }
        }

        Err(String::from("Could not parse volume.").into())
    }
}
