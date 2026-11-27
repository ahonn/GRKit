//! Alerts that the store asks for and the pages show: a change the camera
//! refused, or a restart the person must do. The store says what happened;
//! `pages::present_alerts` words it and shows it.

use crate::domain::{LookError, recipes::RecipeError};

use super::AppState;

/// What an alert is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Alert {
    /// A change to the Looks failed.
    Look(LookError),
    /// A change to the modules failed.
    Module(LookError),
    /// A change to the recipes failed.
    Recipe(RecipeError),
    /// The camera has no standby to wake from: the person restarts it.
    ManualRestart,
}

/// An alert to show. `closed` hears when the person closed it, for a store
/// task that waits for that.
pub struct AlertRequest {
    pub alert: Alert,
    pub closed: Option<futures_channel::oneshot::Sender<()>>,
}

/// The alerts that wait for the pages, in order.
pub(super) struct AlertQueue {
    tx: async_channel::Sender<AlertRequest>,
    rx: async_channel::Receiver<AlertRequest>,
}

impl AlertQueue {
    pub(super) fn new() -> Self {
        let (tx, rx) = async_channel::unbounded();
        Self { tx, rx }
    }
}

impl AppState {
    /// Ask the pages to show `alert`.
    pub(crate) fn alert(self, alert: Alert) {
        self.push_alert(AlertRequest {
            alert,
            closed: None,
        });
    }

    /// Show `alert` and wait until the person closes it.
    pub(super) async fn alert_closed(self, alert: Alert) {
        let (tx, rx) = futures_channel::oneshot::channel();
        self.push_alert(AlertRequest {
            alert,
            closed: Some(tx),
        });
        let _ = rx.await;
    }

    fn push_alert(self, request: AlertRequest) {
        // The queue lives as long as the app state, so it is open.
        let _ = self.alerts.with(|queue| queue.tx.try_send(request));
    }

    /// The next alert to show.
    pub async fn next_alert(self) -> Option<AlertRequest> {
        let rx = self.alerts.with(|queue| queue.rx.clone());
        rx.recv().await.ok()
    }
}
