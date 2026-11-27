//! The wire contract between `grkit-serve`, the Wi-Fi service on the
//! camera, and the GRKit phone app: the paths of the service (`routes`) and
//! the JSON bodies that both sides read and write. The service serializes
//! these types and the app deserializes them, so a field has one name and
//! one type on both sides.
//!
//! A field that the app can do without has a default, so that a body
//! without it still reads.

pub mod looks;
pub mod props;
pub mod recipes;
pub mod routes;
