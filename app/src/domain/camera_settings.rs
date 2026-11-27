//! The GRKit settings of the camera's MENU (`/v1/params/settings`): Enable at
//! Start, Shooting Settings, Display Settings, and Shutdown Screen. A setting
//! that the camera does not have is `None`, and the app does not show it.

use serde_json::{Value, json};

/// Enable at Start: GRKit on, off, or as in the last session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartMode {
    On,
    Off,
    Last,
}

impl StartMode {
    pub const ALL: [StartMode; 3] = [StartMode::On, StartMode::Off, StartMode::Last];

    fn key(self) -> &'static str {
        match self {
            StartMode::On => "on",
            StartMode::Off => "off",
            StartMode::Last => "last",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistanceUnit {
    Meters,
    Feet,
}

impl DistanceUnit {
    pub const ALL: [DistanceUnit; 2] = [DistanceUnit::Meters, DistanceUnit::Feet];

    fn key(self) -> &'static str {
        match self {
            DistanceUnit::Meters => "meters",
            DistanceUnit::Feet => "feet",
        }
    }
}

/// The on/off settings, by their key in the service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingSwitch {
    UncroppedRaw,
    SimpleBattery,
    BatteryPercent,
    PlaybackIcons,
    CustomPicture,
    ShowText,
}

impl SettingSwitch {
    const ALL: [SettingSwitch; 6] = [
        SettingSwitch::UncroppedRaw,
        SettingSwitch::SimpleBattery,
        SettingSwitch::BatteryPercent,
        SettingSwitch::PlaybackIcons,
        SettingSwitch::CustomPicture,
        SettingSwitch::ShowText,
    ];

    fn key(self) -> &'static str {
        match self {
            SettingSwitch::UncroppedRaw => "uncroppedRaw",
            SettingSwitch::SimpleBattery => "simpleBattery",
            SettingSwitch::BatteryPercent => "batteryPercent",
            SettingSwitch::PlaybackIcons => "playbackIcons",
            SettingSwitch::CustomPicture => "customPicture",
            SettingSwitch::ShowText => "showText",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CameraSettings {
    pub start: Option<StartMode>,
    pub unit: Option<DistanceUnit>,
    switches: [Option<bool>; 6],
}

impl CameraSettings {
    pub fn switch(&self, switch: SettingSwitch) -> Option<bool> {
        self.switches[switch.index()]
    }

    /// The settings after `change`, as the camera answers once it applies it.
    pub fn with(&self, change: SettingChange) -> CameraSettings {
        let mut next = self.clone();
        match change {
            SettingChange::Start(mode) => next.start = Some(mode),
            SettingChange::Unit(unit) => next.unit = Some(unit),
            SettingChange::Switch(switch, on) => next.switches[switch.index()] = Some(on),
        }
        next
    }
}

/// One change of a setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingChange {
    Start(StartMode),
    Unit(DistanceUnit),
    Switch(SettingSwitch, bool),
}

impl SettingChange {
    /// The body of `PATCH /v1/params/settings`.
    pub fn body(self) -> Value {
        match self {
            SettingChange::Start(mode) => json!({ "start": mode.key() }),
            SettingChange::Unit(unit) => json!({ "distanceUnit": unit.key() }),
            SettingChange::Switch(switch, on) => json!({ switch.key(): on }),
        }
    }
}

/// The answer of `GET` or `PATCH /v1/params/settings`. A value that this app
/// does not know reads as a setting that the camera does not have.
pub fn parse_camera_settings(bytes: &[u8]) -> Result<CameraSettings, String> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let settings = value
        .get("settings")
        .and_then(Value::as_object)
        .ok_or("The answer has no settings")?;
    let text = |key: &str| settings.get(key).and_then(Value::as_str);
    let mut out = CameraSettings {
        start: StartMode::ALL
            .into_iter()
            .find(|mode| text("start") == Some(mode.key())),
        unit: DistanceUnit::ALL
            .into_iter()
            .find(|unit| text("distanceUnit") == Some(unit.key())),
        ..CameraSettings::default()
    };
    for switch in SettingSwitch::ALL {
        out.switches[switch.index()] = settings.get(switch.key()).and_then(Value::as_bool);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_settings_and_leaves_out_what_the_camera_lacks() {
        let settings = parse_camera_settings(
            br#"{"settings":{"start":"last","distanceUnit":"feet","uncroppedRaw":true,"showText":false,"future":1}}"#,
        )
        .unwrap();
        assert_eq!(settings.start, Some(StartMode::Last));
        assert_eq!(settings.unit, Some(DistanceUnit::Feet));
        assert_eq!(settings.switch(SettingSwitch::UncroppedRaw), Some(true));
        assert_eq!(settings.switch(SettingSwitch::ShowText), Some(false));
        assert_eq!(settings.switch(SettingSwitch::BatteryPercent), None);
        let change = SettingChange::Switch(SettingSwitch::BatteryPercent, true);
        assert_eq!(change.body(), json!({"batteryPercent": true}));
        assert_eq!(
            settings.with(change).switch(SettingSwitch::BatteryPercent),
            Some(true)
        );
    }
}
