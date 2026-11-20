//! The change stream of the connected camera: open while the link is up and
//! the app is in the foreground, opened again with a growing delay when the
//! socket drops. An event makes the app read what it shows again; the
//! stream carries no data.

use std::time::{Duration, Instant};

use nsh_core::platform::Lifecycle;
use nsh_core::prelude::*;

use super::AppState;
use crate::domain::changes::{ChangeEvent, ChangeKind, FRESH, retry_delay};
use crate::platform::{
    self,
    changes::{ChangeStream, StreamEvent},
};

/// How often the watch looks at the link and the lifecycle.
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

impl AppState {
    /// Keep a change stream open while the camera link is up and the app is
    /// in the foreground. Not in the demo, nor where no camera link exists.
    pub(super) fn watch_changes(self) {
        if self.demo() || !platform::changes::supported() {
            return;
        }
        spawn(async move {
            // The connect generation of the last stream while its link stays
            // up: a stream after it may have missed changes.
            let mut session = None;
            let mut failures = 0;
            loop {
                if !self.stream_wanted() {
                    if !self.link_connected() {
                        session = None;
                    }
                    failures = 0;
                    timer(WATCH_INTERVAL).await;
                    continue;
                }
                let generation = self.connect_generation.get();
                // The connect reads the camera itself; a later stream reads
                // it again after its hello.
                let reconnect = session == Some(generation);
                session = Some(generation);
                let stream = platform::changes::open(&platform::camera_http_base());
                let greeted = self.follow_changes(&stream, generation, reconnect).await;
                drop(stream);
                self.stream_heard.with(|heard| heard.set(None));
                if !self.stream_wanted() || self.connect_generation.get() != generation {
                    continue;
                }
                // The socket dropped while the link is up.
                if greeted {
                    failures = 0;
                }
                let retry_at = Instant::now() + retry_delay(failures);
                failures = failures.saturating_add(1);
                while self.stream_wanted() && self.connect_generation.get() == generation {
                    let Some(remaining) = retry_at.checked_duration_since(Instant::now()) else {
                        break;
                    };
                    timer(remaining.min(WATCH_INTERVAL)).await;
                }
            }
        });
    }

    fn stream_wanted(self) -> bool {
        self.link_connected() && lifecycle() == Lifecycle::Active
    }

    /// Handle the events of one stream until it ends or is no longer
    /// wanted. Returns whether the stream said hello.
    async fn follow_changes(self, stream: &ChangeStream, generation: u64, reconnect: bool) -> bool {
        use futures_util::future::{Either, select};
        let mut greeted = false;
        loop {
            match select(Box::pin(stream.next()), Box::pin(timer(WATCH_INTERVAL))).await {
                Either::Left((None, _)) => return greeted,
                Either::Left((Some(event), _)) => {
                    self.stream_heard
                        .with(|heard| heard.set(Some(Instant::now())));
                    match event {
                        StreamEvent::Change(ChangeEvent::Hello { .. }) => {
                            greeted = true;
                            if reconnect {
                                self.camera_changed(ChangeKind::Looks);
                                self.camera_changed(ChangeKind::Software);
                            }
                        }
                        StreamEvent::Change(ChangeEvent::Changed(kind)) => {
                            self.camera_changed(kind)
                        }
                        StreamEvent::Heartbeat => {}
                    }
                }
                Either::Right(_) => {}
            }
            if !self.stream_wanted() || self.connect_generation.get() != generation {
                return greeted;
            }
        }
    }

    /// Read again what changed, where the app shows it.
    fn camera_changed(self, kind: ChangeKind) {
        app_info!("camera changed: {kind:?}");
        if kind == ChangeKind::Software {
            self.refresh_software();
        }
        // The Looks also show whether a software request blocks them. A
        // change that the app itself makes reads them when it ends.
        if self.looks_snapshot().is_some() && !self.looks_loading() && self.look_busy().is_none() {
            self.refresh_looks();
        }
        // A recipe saved, applied, or moved on the camera.
        if self.recipes_snapshot().is_some() && self.recipe_busy().is_none() {
            self.refresh_recipes();
        }
    }

    /// The change stream got a frame lately, so the camera service answers.
    pub(super) fn change_stream_fresh(self) -> bool {
        self.stream_heard
            .with(|heard| heard.get().is_some_and(|at| at.elapsed() < FRESH))
    }
}
