//! Parse camera BLE advertisements and map model codes.

use crate::domain::CameraModel;
use crate::domain::uuids::{
    ADV_FORMAT, BLUETOOTH_CONTROL_SERVICE, CAMERA_INFORMATION_SERVICE, COMPANY_ID,
};

/// Fixed value lengths after each type byte in the manufacturer payload.
const FIELD_LENGTHS: &[(u8, usize)] = &[(0x01, 1), (0x02, 4), (0x03, 1)];

/// Decoded advertisement fields used for discovery and wake decisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Advertisement {
    pub model: Option<CameraModel>,
    pub model_code: Option<u8>,
    pub powered_on: Option<bool>,
    pub advertises_camera_information: bool,
    pub advertises_bluetooth_control: bool,
}

impl Advertisement {
    /// Both power and the Camera Information service must agree the camera is on.
    pub fn shows_powered_on(&self) -> bool {
        self.powered_on == Some(true) && self.advertises_camera_information
    }
}

/// Map a manufacturer model code to a UI model. HDF / mono variants fold into
/// the base family the app updates.
pub fn model_from_code(code: u8) -> Option<CameraModel> {
    match code {
        0x01 | 0x05 => Some(CameraModel::Gr3),
        0x03 | 0x06 => Some(CameraModel::Gr3x),
        0x07..=0x09 => Some(CameraModel::Gr4),
        _ => None,
    }
}

/// Parse manufacturer data (company id first) plus advertised service UUIDs.
pub fn parse_advertisement(
    manufacturer_data: Option<&[u8]>,
    service_uuids: &[impl AsRef<str>],
) -> Advertisement {
    let advertises_camera_information = service_uuids
        .iter()
        .any(|u| u.as_ref().eq_ignore_ascii_case(CAMERA_INFORMATION_SERVICE));
    let advertises_bluetooth_control = service_uuids
        .iter()
        .any(|u| u.as_ref().eq_ignore_ascii_case(BLUETOOTH_CONTROL_SERVICE));

    let mut powered_on = None;
    let mut model_code = None;

    if let Some(bytes) = manufacturer_data
        && bytes.len() >= 3
        && bytes[0] == COMPANY_ID[0]
        && bytes[1] == COMPANY_ID[1]
        && bytes[2] == ADV_FORMAT
    {
        let mut offset = 3;
        while offset < bytes.len() {
            let typ = bytes[offset];
            let Some(&(_, len)) = FIELD_LENGTHS.iter().find(|(t, _)| *t == typ) else {
                break;
            };
            if offset + 1 + len > bytes.len() {
                break;
            }
            let value = bytes[offset + 1];
            match typ {
                0x01 => model_code = Some(value),
                0x03 => {
                    powered_on = match value {
                        0x00 => Some(false),
                        0x01 => Some(true),
                        _ => None,
                    };
                }
                _ => {}
            }
            offset += 1 + len;
        }
    }

    Advertisement {
        model: model_code.and_then(model_from_code),
        model_code,
        powered_on,
        advertises_camera_information,
        advertises_bluetooth_control,
    }
}

/// True when the advertisement is from a supported camera family.
pub fn is_supported_camera(adv: &Advertisement) -> bool {
    adv.model.is_some() && (adv.advertises_camera_information || adv.advertises_bluetooth_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gr3_on_payload() -> Vec<u8> {
        vec![
            0x5F, 0x06, 0xDA, // header
            0x01, 0x01, // model GR III
            0x02, 0x01, 0x00, 0x00, 0x00, // serial
            0x03, 0x01, // power on
        ]
    }

    #[test]
    fn parses_gr3_powered_on() {
        let adv = parse_advertisement(Some(&gr3_on_payload()), &[CAMERA_INFORMATION_SERVICE]);
        assert_eq!(adv.model, Some(CameraModel::Gr3));
        assert!(adv.shows_powered_on());
    }

    #[test]
    fn powered_on_requires_camera_info_service() {
        let adv = parse_advertisement(Some(&gr3_on_payload()), &[BLUETOOTH_CONTROL_SERVICE]);
        assert_eq!(adv.powered_on, Some(true));
        assert!(!adv.shows_powered_on());
    }

    #[test]
    fn maps_hdf_and_gr4_codes() {
        assert_eq!(model_from_code(0x05), Some(CameraModel::Gr3));
        assert_eq!(model_from_code(0x06), Some(CameraModel::Gr3x));
        assert_eq!(model_from_code(0x07), Some(CameraModel::Gr4));
        assert_eq!(model_from_code(0x08), Some(CameraModel::Gr4));
        assert_eq!(model_from_code(0x00), None);
    }

    #[test]
    fn off_camera_still_supported() {
        let payload = vec![0x5F, 0x06, 0xDA, 0x01, 0x03, 0x03, 0x00];
        let adv = parse_advertisement(Some(&payload), &[BLUETOOTH_CONTROL_SERVICE]);
        assert_eq!(adv.model, Some(CameraModel::Gr3x));
        assert!(is_supported_camera(&adv));
        assert!(!adv.shows_powered_on());
    }
}
