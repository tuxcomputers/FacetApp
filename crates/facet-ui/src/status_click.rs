//! A left click on the status item. It is Pause's accelerator. While a connected cube is followed, a second click
//! within the double-click interval makes the pair Lock's accelerator instead, so the pause waits out that interval
//! before it is sent.

use std::rc::{Rc, Weak};
use std::time::Duration;

use facet_core::debug_log::{Record, Tag, Trace};

use crate::device::Device;
use crate::faces::Faces;

/// What a left click does, for the composition root to call on each one. `interval` is read on every click and is the
/// system's double-click interval.
pub fn gesture(
    faces: Weak<Faces>,
    device: Weak<Device>,
    log: Rc<Trace>,
    interval: impl Fn() -> Duration + 'static,
) -> impl Fn() + 'static {
    let pending = Rc::new(slint::Timer::default());
    move || {
        let Some(faces) = faces.upgrade() else { return };
        if !faces.is_following_cube() {
            faces.toggle_pause();
            return;
        }
        let Some(cube) = device.upgrade() else { return };
        if !cube.is_cube_connected() {
            cube.toggle_cube_pause();
            return;
        }
        if pending.running() {
            pending.stop();
            log.record(Tag::Tray, || "Status item double clicked, so the cube lock is toggled".to_string());
            cube.toggle_cube_lock();
            return;
        }
        let waiting = device.clone();
        pending.start(slint::TimerMode::SingleShot, interval(), move || {
            if let Some(cube) = waiting.upgrade() {
                cube.toggle_cube_pause();
            }
        });
    }
}
