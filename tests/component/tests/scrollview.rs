// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Wakunguma Kalimukwa

use slint::{LogicalPosition, platform::WindowEvent};

include!(concat!(env!("OUT_DIR"), "/scrollview.rs"));

#[test]
fn scroll() {
    i_slint_backend_testing::init_no_event_loop();

    let instance = TestCase::new().unwrap();
    assert_eq!(instance.get_content_x(), 0.0);
    assert_eq!(instance.get_content_y(), 0.0);

    // Scroll vertically by pressing on the handle
    let start = LogicalPosition::new(96.0, 1.0);
    let end = LogicalPosition::new(96.0, 10.0);
    let button = slint::platform::PointerEventButton::Left;
    let window = instance.window();
    window.dispatch_event(WindowEvent::PointerMoved { position: start });
    window.dispatch_event(WindowEvent::PointerPressed {
        position: start,
        button,
    });
    window.dispatch_event(WindowEvent::PointerMoved { position: end });
    window.dispatch_event(WindowEvent::PointerReleased {
        position: end,
        button,
    });

    assert!(instance.get_content_y() < 0.0);
}
