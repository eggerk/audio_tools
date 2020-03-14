use regex::Regex;
use std::error::Error;
use std::process;
use std::str;

#[derive(Clone, Debug)]
pub struct Interface {
    pub index: i32,
    pub name: String,
    pub active: bool,
    pub state: String,
}

pub struct VolumeControl {
    interfaces: Vec<Interface>,
    pub active_interface: Option<Interface>,
}

fn get_current_audio_outputs() -> Result<Vec<Interface>, Box<dyn Error>> {
    let sinks_list = process::Command::new("pacmd")
        .args(&["list-sinks"])
        .output()?;
    let sinks_output = str::from_utf8(&sinks_list.stdout)?;

    let mut all_interfaces = Vec::new();
    let mut next_interface: Option<Interface> = None;

    let add_to_list = |all_interfaces: &mut Vec<Interface>, next_interface| {
        if let Some(interface) = next_interface {
            all_interfaces.push(interface);
        }
    };

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
                name: String::new(),
                active: active,
                state: String::new(),
            });
        } else if line.contains("state:") {
            if let Some(interface) = next_interface {
                let mut interface = interface;
                if let Some(state) = line.rsplit(' ').next() {
                    interface.state = state.to_string();
                }
                next_interface = Some(interface);
            }
        } else if line.contains("device.description") {
            if let Some(interface) = next_interface {
                let mut interface = interface;
                let re = Regex::new(r#"^.*device.description = "(?P<n>.*)".*$"#)?;
                interface.name = re.replace_all(line, "$n").to_string();
                next_interface = Some(interface);
            }
        }
    }

    add_to_list(&mut all_interfaces, next_interface);

    Ok(all_interfaces)
}

pub fn get_active_interface(interfaces: &Vec<Interface>) -> Option<Interface> {
    for interface in interfaces {
        if interface.active {
            return Some(interface.clone());
        }
    }
    None
}

impl VolumeControl {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let interfaces = get_current_audio_outputs()?;
        let active_interface = get_active_interface(&interfaces);
        Ok(Self {
            interfaces: interfaces,
            active_interface: active_interface,
        })
    }

    pub fn change_volume(&self, amount: i32) -> Result<(), Box<dyn Error>> {
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
            eprintln!("{}", str::from_utf8(&output.stderr).unwrap());
        }

        Ok(())
    }

    pub fn toggle_mute(&mut self) -> Result<(), Box<dyn Error>> {
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

    pub fn get_available_interfaces(&mut self) -> Result<&Vec<Interface>, Box<dyn Error>> {
        self.interfaces = get_current_audio_outputs()?;
        self.active_interface = get_active_interface(&self.interfaces);
        Ok(&self.interfaces)
    }

    pub fn cycle_through_interfaces(&mut self) -> Result<(), Box<dyn Error>> {
        self.interfaces = get_current_audio_outputs()?;

        let active_interfaces: Vec<usize> = self
            .interfaces
            .iter()
            .enumerate()
            .filter(|(_, interface)| interface.active)
            .map(|(idx, _)| idx)
            .collect();

        if active_interfaces.len() != 1 {
            return Err(String::from("Not enough active interfaces.").into());
        }

        let next_interface = (active_interfaces[0] + 1) % self.interfaces.len();

        process::Command::new("pactl")
            .args(&["set-default-sink", &next_interface.to_string()])
            .output()?;

        let sink_inputs = list_sink_inputs()?;
        for sink in sink_inputs.iter() {
            process::Command::new("pacmd")
                .args(&[
                    "move-sink-input",
                    &sink.to_string(),
                    &next_interface.to_string(),
                ])
                .output()?;
        }

        Ok(())
    }
}

fn list_sink_inputs() -> Result<Vec<i32>, Box<dyn Error>> {
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
