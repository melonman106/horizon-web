//! What the game asks of the loop that runs it.
//!
//! A window's game runs inside winit's event loop, and its methods used to take winit's
//! `ActiveEventLoop` only to stop the loop or to say when to wake next. A headless client
//! (#4951) has no display, so it cannot have that loop: it runs its own, and hands the same
//! methods this trait instead. winit's loop implements it, so a window's game is unchanged.

use horizon_os::time::{Duration, Instant};
use winit::event_loop::{ActiveEventLoop, ControlFlow};

/// How long the page's loop sleeps after `about_to_wait`. With a redraw
/// asked, its animation frame wakes the loop; the deadline only keeps the
/// loop turning while the tab is hidden and gets no frames. Without one,
/// nothing may wake the loop, so it looks again in 4 ms.
#[cfg(target_arch = "wasm32")]
pub(super) fn web_wait(redraw_asked: bool) -> ControlFlow {
    let after = if redraw_asked { 100 } else { 4 };
    ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(after))
}

/// A successful frame only permits polling until the next idle iteration. If
/// the compositor stops delivering redraws, the old result must not keep the
/// loop spinning indefinitely. Offscreen frames are driven by the loop itself.
/// The page waits on its animation frames instead (`web_wait`).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) fn redraw_flow(frame_drawn: &mut bool, offscreen: bool, now: Instant) -> ControlFlow {
    let drew_frame = std::mem::take(frame_drawn);
    if offscreen || drew_frame {
        ControlFlow::Poll
    } else {
        ControlFlow::WaitUntil(now + Duration::from_millis(16))
    }
}

pub(super) trait EventLoopCtl {
    /// Ends the loop after the current iteration (`ActiveEventLoop::exit`).
    fn exit(&self);
    /// When the loop wakes next (`ActiveEventLoop::set_control_flow`).
    fn set_control_flow(&self, flow: ControlFlow);
}

impl EventLoopCtl for ActiveEventLoop {
    fn exit(&self) {
        ActiveEventLoop::exit(self);
    }
    fn set_control_flow(&self, flow: ControlFlow) {
        ActiveEventLoop::set_control_flow(self, flow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_redraw_waits_and_a_new_frame_resumes_polling() {
        let now = Instant::now();
        let mut frame_drawn = true;
        assert_eq!(redraw_flow(&mut frame_drawn, false, now), ControlFlow::Poll);
        assert_eq!(
            redraw_flow(&mut frame_drawn, false, now),
            ControlFlow::WaitUntil(now + Duration::from_millis(16))
        );
        frame_drawn = true;
        assert_eq!(redraw_flow(&mut frame_drawn, false, now), ControlFlow::Poll);
    }

    #[test]
    fn offscreen_keeps_polling_without_a_presented_frame() {
        let mut frame_drawn = false;
        assert_eq!(
            redraw_flow(&mut frame_drawn, true, Instant::now()),
            ControlFlow::Poll
        );
    }
}
