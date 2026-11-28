//! BLE service and characteristic UUIDs used to find and control the camera.

/// Camera Information service (advertised while the camera is on).
pub const CAMERA_INFORMATION_SERVICE: &str = "9A5ED1C5-74CC-4C50-B5B6-66A48E7CCFF1";
/// Bluetooth Control service (advertised while the camera is off).
pub const BLUETOOTH_CONTROL_SERVICE: &str = "0F291746-0C80-4726-87A7-3C501FD3B4B6";
/// Camera service that holds power and related controls.
pub const CAMERA_SERVICE: &str = "4B445988-CAA0-4DD3-941D-37B4F52ACA86";
/// WLAN Control service for AP start and credentials.
pub const WLAN_CONTROL_SERVICE: &str = "F37F568F-9071-445D-A938-5441F2E82399";

/// Camera Power characteristic (`0` off / `1` on).
pub const CAMERA_POWER: &str = "B58CE84C-0666-4DE9-BEC8-2D27B27B3211";
/// Network Type characteristic (`0` off / `1` start access point).
pub const NETWORK_TYPE: &str = "9111CDD0-9F01-45C4-A2D4-E09E8FB0424D";
/// SSID characteristic (UTF-8, null-padded).
pub const SSID: &str = "90638E5A-E77D-409D-B550-78F7E1CA5AB4";
/// Passphrase characteristic (UTF-8, null-padded).
pub const PASSPHRASE: &str = "0F38279C-FE9E-461B-8596-81287E8C9A81";
/// Model number characteristic (UTF-8).
pub const MODEL_NUMBER: &str = "35FE6272-6AA5-44D9-88E1-F09427F51A71";
/// Firmware revision characteristic (UTF-8).
pub const FIRMWARE_REVISION: &str = "B4EB8905-7411-40A6-A367-2834C2157EA7";

/// Ricoh Imaging company id in manufacturer data (little-endian bytes).
pub const COMPANY_ID: [u8; 2] = [0x5F, 0x06];
/// Format byte after the company id.
pub const ADV_FORMAT: u8 = 0xDA;
