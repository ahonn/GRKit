//! How the remembered camera is reached when the user asks to connect: the
//! network first, in case the phone is already on the camera network; then a
//! short look for its advertisement; then a Bluetooth wake, bounded when the
//! camera was not seen. The camera Wi-Fi is never joined without the wake.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// The opening "already on the camera network" read. It almost always
/// fails, and every miss spends the whole budget, so it is sized to the
/// slowest observed success.
pub const DIRECT_PROBE_TIMEOUT: Duration = Duration::from_millis(1_300);
/// How long to listen for the camera's advertisement.
pub const PEEK_TIMEOUT: Duration = Duration::from_secs(3);
/// One short second look: the first can miss a frame of a live camera.
pub const PEEK_RETRY_TIMEOUT: Duration = Duration::from_secs(1);

/// What a look for the camera's advertisement found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Peek {
    pub found: bool,
    /// The power the frame advertised, when it said.
    pub powered_on: Option<bool>,
}

/// How the attempt goes on after the opening steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SavedRoute<C> {
    /// The camera answered on the network.
    Direct(C),
    /// Wake the camera over Bluetooth.
    Wake(WakePlan),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WakePlan {
    /// A frame said the camera is on: the wake write can be skipped.
    pub advertised_powered_on: bool,
    /// No frame was seen: one bounded connect decides whether the camera is
    /// there at all.
    pub unseen: bool,
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// The operations the opening steps use.
pub trait SavedTransport {
    /// What a successful direct read returns.
    type Camera;
    /// Read the remembered camera over the network within `timeout`. `None`
    /// when nothing answered or another camera did.
    fn direct(&mut self, timeout: Duration) -> BoxFut<'_, Option<Self::Camera>>;
    /// Listen for the remembered camera's advertisement within `timeout`.
    fn peek(&mut self, timeout: Duration) -> BoxFut<'_, Peek>;
}

/// The direct read, then the advertisement look with its one retry.
pub async fn plan_saved<T: SavedTransport>(transport: &mut T) -> SavedRoute<T::Camera> {
    if let Some(camera) = transport.direct(DIRECT_PROBE_TIMEOUT).await {
        return SavedRoute::Direct(camera);
    }
    let mut peek = transport.peek(PEEK_TIMEOUT).await;
    if !peek.found {
        peek = transport.peek(PEEK_RETRY_TIMEOUT).await;
    }
    SavedRoute::Wake(WakePlan {
        advertised_powered_on: peek.powered_on == Some(true),
        unseen: !peek.found,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::link::block_on_ready;

    struct Script {
        direct: Option<&'static str>,
        peeks: Vec<Peek>,
        log: Vec<String>,
    }

    impl SavedTransport for Script {
        type Camera = &'static str;
        fn direct(&mut self, timeout: Duration) -> BoxFut<'_, Option<&'static str>> {
            Box::pin(async move {
                self.log.push(format!("direct {}", timeout.as_millis()));
                self.direct
            })
        }
        fn peek(&mut self, timeout: Duration) -> BoxFut<'_, Peek> {
            Box::pin(async move {
                self.log.push(format!("peek {}", timeout.as_millis()));
                if self.peeks.is_empty() {
                    Peek::default()
                } else {
                    self.peeks.remove(0)
                }
            })
        }
    }

    fn plan(
        direct: Option<&'static str>,
        peeks: Vec<Peek>,
    ) -> (SavedRoute<&'static str>, Vec<String>) {
        let mut script = Script {
            direct,
            peeks,
            log: Vec::new(),
        };
        let route = block_on_ready(plan_saved(&mut script));
        (route, script.log)
    }

    #[test]
    fn a_camera_on_the_network_needs_no_bluetooth() {
        let (route, log) = plan(Some("camera"), Vec::new());
        assert_eq!(route, SavedRoute::Direct("camera"));
        assert_eq!(log, ["direct 1300"]);
    }

    #[test]
    fn a_seen_camera_is_woken_with_its_advertised_power() {
        let on = Peek {
            found: true,
            powered_on: Some(true),
        };
        let (route, log) = plan(None, vec![on]);
        assert_eq!(
            route,
            SavedRoute::Wake(WakePlan {
                advertised_powered_on: true,
                unseen: false
            })
        );
        assert_eq!(log, ["direct 1300", "peek 3000"]);
        // A standby frame, or one without power, keeps the wake write.
        let standby = Peek {
            found: true,
            powered_on: Some(false),
        };
        assert_eq!(
            plan(None, vec![standby]).0,
            SavedRoute::Wake(WakePlan {
                advertised_powered_on: false,
                unseen: false
            })
        );
    }

    #[test]
    fn a_missed_frame_gets_one_short_retry_then_a_bounded_wake() {
        let late = Peek {
            found: true,
            powered_on: None,
        };
        let (route, log) = plan(None, vec![Peek::default(), late]);
        assert_eq!(
            route,
            SavedRoute::Wake(WakePlan {
                advertised_powered_on: false,
                unseen: false
            })
        );
        assert_eq!(log, ["direct 1300", "peek 3000", "peek 1000"]);
        let (route, _) = plan(None, Vec::new());
        assert_eq!(
            route,
            SavedRoute::Wake(WakePlan {
                advertised_powered_on: false,
                unseen: true
            })
        );
    }
}
