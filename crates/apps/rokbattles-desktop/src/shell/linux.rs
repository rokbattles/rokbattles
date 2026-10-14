//! Tauri 2.12 puts Linux child webviews in a vertical GtkBox, which ignores their
//! requested position. A GtkOverlay keeps the shell full-size and allocates the
//! content inside fixed margins, including when shrinking or changing DPI.

use gtk::prelude::*;

pub(super) fn attach(webview: &tauri::Webview, shell: bool) -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    webview
        .with_webview(move |platform| {
            let result = (|| {
                let inner = platform.inner();
                let container = inner
                    .parent()
                    .and_then(|parent| parent.downcast::<gtk::Box>().ok())
                    .ok_or("Expected the Tauri webview container")?;

                if shell {
                    attach_resize_handler(inner.upcast_ref::<gtk::Widget>());

                    let overlay = gtk::Overlay::new();
                    container.remove(&inner);
                    overlay.add(&inner);
                    container.pack_start(&overlay, true, true, 0);
                    overlay.show_all();
                } else {
                    let overlay = container
                        .children()
                        .into_iter()
                        .find_map(|child| child.downcast::<gtk::Overlay>().ok())
                        .ok_or("Launcher overlay is unavailable")?;

                    container.remove(&inner);
                    inner.set_halign(gtk::Align::Fill);
                    inner.set_valign(gtk::Align::Fill);
                    inner.set_margin_start((super::EDGE + super::BORDER) as i32);
                    inner.set_margin_end((super::EDGE + super::BORDER) as i32);
                    inner.set_margin_top((super::TITLE_HEIGHT + super::BORDER) as i32);
                    inner.set_margin_bottom((super::FOOTER_HEIGHT + super::BORDER) as i32);

                    overlay.add_overlay(&inner);
                    inner.show();
                }

                Ok(())
            })();

            if sender.send(result).is_err() {
                eprintln!("[rokbattles] webview layout receiver was dropped");
            }
        })
        .map_err(|error| error.to_string())?;

    receiver.recv().map_err(|error| error.to_string())?
}

fn resize_edge(x: f64, y: f64, width: i32, height: i32) -> Option<gtk::gdk::WindowEdge> {
    use gtk::gdk::WindowEdge;

    let left = x < 6.0;
    let right = x >= f64::from(width) - 6.0;
    let top = y < 6.0;
    let bottom = y >= f64::from(height) - 6.0;

    match (left, right, top, bottom) {
        (true, _, true, _) => Some(WindowEdge::NorthWest),
        (_, true, true, _) => Some(WindowEdge::NorthEast),
        (true, _, _, true) => Some(WindowEdge::SouthWest),
        (_, true, _, true) => Some(WindowEdge::SouthEast),
        (true, _, _, _) => Some(WindowEdge::West),
        (_, true, _, _) => Some(WindowEdge::East),
        (_, _, true, _) => Some(WindowEdge::North),
        (_, _, _, true) => Some(WindowEdge::South),
        _ => None,
    }
}

fn resizable_window(widget: &gtk::Widget) -> Option<gtk::Window> {
    let window = widget.toplevel()?.downcast::<gtk::Window>().ok()?;

    (window.is_resizable() && !window.is_maximized()).then_some(window)
}

fn attach_resize_handler(widget: &gtk::Widget) {
    use gtk::{gdk, glib::Propagation};

    // Tauri's Linux handler assumes webview -> box -> window. The overlay adds
    // one level, so use the actual toplevel instead of a fixed parent chain.
    widget.add_events(gdk::EventMask::POINTER_MOTION_MASK | gdk::EventMask::BUTTON_PRESS_MASK);

    widget.connect_motion_notify_event(|widget, event| {
        if resizable_window(widget).is_none() {
            return Propagation::Proceed;
        }

        let (x, y) = event.position();
        let Some(edge) = resize_edge(x, y, widget.allocated_width(), widget.allocated_height())
        else {
            return Propagation::Proceed;
        };

        let name = match edge {
            gdk::WindowEdge::NorthWest | gdk::WindowEdge::SouthEast => "nwse-resize",
            gdk::WindowEdge::NorthEast | gdk::WindowEdge::SouthWest => "nesw-resize",
            gdk::WindowEdge::North | gdk::WindowEdge::South => "ns-resize",
            _ => "ew-resize",
        };
        if let Some(window) = widget.window() {
            window.set_cursor(gdk::Cursor::from_name(&window.display(), name).as_ref());
        }

        Propagation::Stop
    });

    widget.connect_button_press_event(|widget, event| {
        if event.button() != 1 {
            return Propagation::Proceed;
        }

        let Some(window) = resizable_window(widget) else {
            return Propagation::Proceed;
        };

        let (x, y) = event.position();
        let Some(edge) = resize_edge(x, y, widget.allocated_width(), widget.allocated_height())
        else {
            return Propagation::Proceed;
        };

        let (root_x, root_y) = event.root();
        window.begin_resize_drag(edge, 1, root_x as i32, root_y as i32, event.time());

        Propagation::Stop
    });
}

#[cfg(test)]
mod tests {
    use gtk::gdk::WindowEdge;

    use super::*;

    #[test]
    fn all_edges_and_corners_resize_without_stealing_content_clicks() {
        for (x, y, edge) in [
            (1.0, 1.0, WindowEdge::NorthWest),
            (799.0, 1.0, WindowEdge::NorthEast),
            (1.0, 599.0, WindowEdge::SouthWest),
            (799.0, 599.0, WindowEdge::SouthEast),
            (1.0, 300.0, WindowEdge::West),
            (799.0, 300.0, WindowEdge::East),
            (400.0, 1.0, WindowEdge::North),
            (400.0, 599.0, WindowEdge::South),
        ] {
            assert_eq!(resize_edge(x, y, 800, 600), Some(edge));
        }

        assert_eq!(resize_edge(9.0, 49.0, 800, 600), None);
        assert_eq!(resize_edge(400.0, 580.0, 800, 600), None);
    }
}
