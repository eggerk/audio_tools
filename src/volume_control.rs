use regex::Regex;
use std::process;
use std::str;

use failure::{bail, Error};
use log::{debug, error};

#[derive(Clone, Debug)]
pub struct Interface {
    pub index: i32,
    pub active: bool,
    pub state: String,
    pub name: String,
}

pub struct VolumeControl {
    interfaces: Vec<Interface>,
    pub active_interface: Option<Interface>,
}

fn get_current_audio_outputs() -> Result<Vec<Interface>, Error> {
    let sinks_list = process::Command::new("pacmd")
        .args(&["list-sinks"])
        .output()?;
    let sinks_output = str::from_utf8(&sinks_list.stdout)?;

    let mut all_interfaces = Vec::new();
    let mut next_interface: Option<Interface> = None;

    let add_to_list = |all_interfaces: &mut Vec<Interface>, next_interface: Option<Interface>| {
        if let Some(interface) = next_interface {
            debug!("  - Found sink: {:?}", interface);
            all_interfaces.push(interface);
        }
    };

    debug!("Collecting current audio sinks:");
    for line in sinks_output.lines() {
        if line.contains("index") {
            // Next sink.
            // Add previous sink to list.
            add_to_list(&mut all_interfaces, next_interface);

            let active = line.trim().starts_with("*");
            let re = Regex::new(r"^.*index: (?P<i>[0-9]*).*$")?;
            let index: i32 = re.replace(line, "$i").parse()?;
            next_interface = Some(Interface {
                index: index,
                active: active,
                state: String::new(),
                name: String::new(),
            });
        } else if line.contains("state:") {
            if let Some(interface) = &mut next_interface {
                if let Some(state) = line.rsplit(' ').next() {
                    interface.state = state.to_string();
                }
            }
        } else if line.contains("device.description") {
            if let Some(interface) = &mut next_interface {
                let re = Regex::new(r#"^.*device.description = "(?P<n>.*)".*$"#)?;
                interface.name = re.replace_all(line, "$n").to_string();
            }
        }
    }

    add_to_list(&mut all_interfaces, next_interface);

    Ok(all_interfaces)
}

pub fn get_active_interface(interfaces: &Vec<Interface>) -> Option<Interface> {
    for interface in interfaces {
        if interface.active {
            debug!("Found active interface: {:?}", interface);
            return Some(interface.clone());
        }
    }
    None
}

impl VolumeControl {
    pub fn new() -> Result<Self, Error> {
        let interfaces = get_current_audio_outputs()?;
        let active_interface = get_active_interface(&interfaces);
        Ok(Self {
            interfaces: interfaces,
            active_interface: active_interface,
        })
    }

    pub fn change_volume(&self, amount: i32) -> Result<(), Error> {
        let amount_absolute = amount.abs();
        let direction_sign = if amount >= 0 { '+' } else { '-' };
        let output = process::Command::new("amixer")
            .args(&[
                "-D",
                "pulse",
                "sset",
                "Master",
                &format!("{}%{}", amount_absolute, direction_sign),
            ])
            .output()?;
        if !output.status.success() {
            error!(
                "Failed to change volume: {}",
                str::from_utf8(&output.stderr).unwrap()
            );
        }

        Ok(())
    }

    pub fn toggle_mute(&mut self) -> Result<(), Error> {
        self.interfaces = get_current_audio_outputs()?;

        if let Some(active_interface) = &self.active_interface {
            process::Command::new("pactl")
                .args(&[
                    "set-sink-mute",
                    &active_interface.index.to_string(),
                    "toggle",
                ])
                .output()?;
        }

        Ok(())
    }

    pub fn get_available_interfaces(&mut self) -> Result<&Vec<Interface>, Error> {
        self.interfaces = get_current_audio_outputs()?;
        self.active_interface = get_active_interface(&self.interfaces);
        Ok(&self.interfaces)
    }

    pub fn cycle_through_interfaces(&mut self) -> Result<(), Error> {
        self.interfaces = get_current_audio_outputs()?;

        if self.interfaces.len() <= 1 {
            bail!("Not enough active interfaces.");
        }

        let current_index = match self.interfaces.iter().position(|i| i.active) {
            Some(index) => index,
            None => 0,
        };
        let next_interface_index = (current_index + 1) % self.interfaces.len();
        debug!(
            "Switching to the next interface: {} -> {}",
            self.interfaces[current_index].index, self.interfaces[next_interface_index].index
        );

        let next_interface_index = self.interfaces[next_interface_index].index;
        process::Command::new("pactl")
            .args(&["set-default-sink", &next_interface_index.to_string()])
            .output()?;

        let sink_inputs = list_sink_inputs()?;
        for sink in sink_inputs.iter() {
            debug!("Moving sink input {:?} to new output.", sink);
            process::Command::new("pacmd")
                .args(&[
                    "move-sink-input",
                    &sink.to_string(),
                    &next_interface_index.to_string(),
                ])
                .output()?;
        }

        Ok(())
    }
}

fn list_sink_inputs() -> Result<Vec<i32>, Error> {
    let output = process::Command::new("pacmd")
        .args(&["list-sink-inputs"])
        .output()?;
    let output = str::from_utf8(&output.stdout)?;
    let mut result = Vec::new();
    let re = Regex::new(r"^.*index: (?P<i>[0-9]+).*$")?;
    for line in output.lines() {
        if line.contains("index") {
            let index: i32 = re.replace(line, "$i").parse()?;
            result.push(index);
        }
    }

    Ok(result)
}
