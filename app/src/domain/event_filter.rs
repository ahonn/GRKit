//! Pure matching for BLE events scoped to one peripheral / characteristic.

/// True when an event for `event_peripheral` (and optional characteristic)
/// belongs to the in-flight operation targeting `want_peripheral`.
pub fn matches_operation(
    event_peripheral: &str,
    event_characteristic: Option<&str>,
    want_peripheral: &str,
    want_characteristic: Option<&str>,
) -> bool {
    if !event_peripheral.eq_ignore_ascii_case(want_peripheral) {
        return false;
    }
    match (event_characteristic, want_characteristic) {
        (_, None) => true,
        (Some(got), Some(want)) => got.eq_ignore_ascii_case(want),
        (None, Some(_)) => false,
    }
}

/// Retains AP readiness when the notification arrives before the write response.
#[derive(Debug)]
pub struct ApReadyLatch {
    peripheral_id: String,
    ready: bool,
}

impl ApReadyLatch {
    pub fn new(peripheral_id: &str) -> Self {
        Self {
            peripheral_id: peripheral_id.to_owned(),
            ready: false,
        }
    }

    pub fn reset(&mut self) {
        self.ready = false;
    }

    pub fn observe(&mut self, peripheral_id: &str, characteristic: &str, value: &[u8]) {
        if matches_operation(
            peripheral_id,
            Some(characteristic),
            &self.peripheral_id,
            Some(super::uuids::NETWORK_TYPE),
        ) && value.first() == Some(&1)
        {
            self.ready = true;
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_same_peripheral_any_characteristic() {
        assert!(matches_operation("AAA", Some("ssid"), "aaa", None));
    }

    #[test]
    fn rejects_other_peripheral() {
        assert!(!matches_operation("AAA", None, "BBB", None));
    }

    #[test]
    fn rejects_wrong_characteristic_when_required() {
        assert!(!matches_operation("AAA", Some("ssid"), "AAA", Some("pass")));
        assert!(matches_operation("AAA", Some("ssid"), "AAA", Some("SSID")));
    }

    #[test]
    fn rejects_missing_characteristic_when_required() {
        assert!(!matches_operation("AAA", None, "AAA", Some("ssid")));
    }
}

#[cfg(test)]
mod ap_ready_tests {
    use super::*;
    use crate::domain::uuids::{NETWORK_TYPE, SSID};

    #[test]
    fn retains_early_notification_until_next_attempt() {
        let mut latch = ApReadyLatch::new("camera");
        latch.observe("CAMERA", NETWORK_TYPE, &[1]);
        latch.observe("camera", NETWORK_TYPE, &[0]);
        assert!(latch.is_ready());
        latch.reset();
        assert!(!latch.is_ready());
    }

    #[test]
    fn ignores_other_camera_characteristic_and_non_ready_values() {
        let mut latch = ApReadyLatch::new("camera");
        latch.observe("other", NETWORK_TYPE, &[1]);
        latch.observe("camera", SSID, &[1]);
        latch.observe("camera", NETWORK_TYPE, &[]);
        latch.observe("camera", NETWORK_TYPE, &[0]);
        assert!(!latch.is_ready());
    }
}

/// CoreBluetooth errors that indicate missing authentication or a removed bond.
pub fn is_ble_pairing_error(domain: &str, code: isize) -> bool {
    (domain == "CBATTErrorDomain" && matches!(code, 5 | 15))
        || (domain == "CBErrorDomain" && code == 14)
}

#[cfg(test)]
mod pairing_error_tests {
    use super::*;

    #[test]
    fn recognizes_att_authentication_and_encryption_and_removed_bond() {
        assert!(is_ble_pairing_error("CBATTErrorDomain", 5));
        assert!(is_ble_pairing_error("CBATTErrorDomain", 15));
        assert!(is_ble_pairing_error("CBErrorDomain", 14));
    }

    #[test]
    fn does_not_confuse_identical_codes_in_other_domains() {
        assert!(!is_ble_pairing_error("CBErrorDomain", 5));
        assert!(!is_ble_pairing_error("CBErrorDomain", 15));
        assert!(!is_ble_pairing_error("CBATTErrorDomain", 14));
        assert!(!is_ble_pairing_error("OtherDomain", 5));
    }
}
