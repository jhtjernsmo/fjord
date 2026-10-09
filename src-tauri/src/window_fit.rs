//! Keeps the main window inside the screen it opens on. The configured size
//! (1320x840) is bigger than some laptop screens, and with a second monitor
//! attached the window could open taller or wider than the screen (#65).

use tauri::{Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow};

/// Leave a little room around the window so it never touches the screen edges.
const FILL: f64 = 0.92;

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Where the window should go so it fits inside `area`, or None if it already does.
pub fn fitted(window: Rect, area: Rect) -> Option<Rect> {
    let inside = window.x >= area.x
        && window.y >= area.y
        && window.x + window.width as i32 <= area.x + area.width as i32
        && window.y + window.height as i32 <= area.y + area.height as i32;
    if inside {
        return None;
    }
    let max_w = (f64::from(area.width) * FILL) as u32;
    let max_h = (f64::from(area.height) * FILL) as u32;
    let width = window.width.min(max_w);
    let height = window.height.min(max_h);
    Some(Rect {
        x: area.x + (area.width - width) as i32 / 2,
        y: area.y + (area.height - height) as i32 / 2,
        width,
        height,
    })
}

/// Shrinks and centres the window on its screen when it doesn't fit. Never fails:
/// if the screen can't be read, the window is left as the OS placed it.
pub fn fit_to_screen<R: Runtime>(window: &WebviewWindow<R>) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(pos), Ok(size)) =
        (monitor, window.outer_position(), window.outer_size())
    else {
        return;
    };
    let work = monitor.work_area();
    let area = Rect {
        x: work.position.x,
        y: work.position.y,
        width: work.size.width,
        height: work.size.height,
    };
    let current = Rect {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    };
    if let Some(r) = fitted(current, area) {
        let _ = window.set_size(PhysicalSize::new(r.width, r.height));
        let _ = window.set_position(PhysicalPosition::new(r.x, r.y));
    }
}

/// Runs [`fit_to_screen`] on the main window.
pub fn fit_main_window<R: Runtime, M: Manager<R>>(app: &M) {
    if let Some(window) = app.get_webview_window("main") {
        fit_to_screen(&window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: Rect = Rect {
        x: 0,
        y: 25,
        width: 1440,
        height: 875,
    };

    #[test]
    fn leaves_a_window_that_fits_alone() {
        let w = Rect {
            x: 60,
            y: 40,
            width: 1320,
            height: 840,
        };
        assert_eq!(fitted(w, LAPTOP), None);
    }

    #[test]
    fn shrinks_and_centres_a_window_taller_than_the_screen() {
        let w = Rect {
            x: 60,
            y: 25,
            width: 1320,
            height: 900,
        };
        let r = fitted(w, LAPTOP).unwrap();
        assert!(r.height <= LAPTOP.height && r.width == 1320, "{r:?}");
        assert!(r.y >= LAPTOP.y && r.y + r.height as i32 <= LAPTOP.y + LAPTOP.height as i32);
        assert_eq!(r.x, (1440 - 1320) / 2);
    }

    #[test]
    fn brings_back_a_window_that_opened_partly_off_screen() {
        // e.g. placed for a second monitor to the right of the laptop screen
        let w = Rect {
            x: 900,
            y: 25,
            width: 1320,
            height: 840,
        };
        let r = fitted(w, LAPTOP).unwrap();
        assert!(r.x >= 0 && r.x + r.width as i32 <= 1440, "{r:?}");
    }

    #[test]
    fn works_on_a_monitor_left_of_the_main_one() {
        let left = Rect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let w = Rect {
            x: -100,
            y: 0,
            width: 1320,
            height: 840,
        };
        let r = fitted(w, left).unwrap();
        assert!(r.x >= -1920 && r.x + r.width as i32 <= 0, "{r:?}");
    }
}
