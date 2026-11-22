// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Wakunguma Kalimukwa

//! Test utilities

use super::*;
use i_slint_backend_testing::ElementHandle;
use mukwa_core::create_test_db;
use slint::language::PointerEventButton;

pub trait ElementHandleExt {
    /// Simulates a single click (or touch tap) on the element at its center point with the
    /// left mouse button.
    fn click(&self);
    /// Finds the element with the given accessible label, panics if the element cannot be found.
    fn find_by_accessible_label(&self, label: &str) -> ElementHandle;
    /// Finds the element with the given type id, panics if the element cannot be found.
    fn find_by_id(&self, id: &str) -> ElementHandle;
    /// Finds the element with the given type name, panics if the element cannot be found.
    fn find_by_type_name(&self, type_name: &str) -> ElementHandle;
    /// Finds all the elements with the given type name.
    fn find_all_by_type_name(&self, type_name: &str) -> Vec<ElementHandle>;
}

impl ElementHandleExt for ElementHandle {
    fn click(&self) {
        self.mock_single_click(PointerEventButton::Left)
    }

    #[track_caller]
    fn find_by_accessible_label(&self, label: &str) -> ElementHandle {
        let element = self
            .query_descendants()
            .match_accessible_label(label)
            .find_first();
        assert!(
            element.is_some(),
            "Element with accessible label '{label}' not found"
        );
        element.unwrap()
    }

    #[track_caller]
    fn find_by_id(&self, id: &str) -> ElementHandle {
        let element = self.query_descendants().match_id(id).find_first();
        assert!(element.is_some(), "Element with id '{id}' not found");
        element.unwrap()
    }

    #[track_caller]
    fn find_by_type_name(&self, type_name: &str) -> ElementHandle {
        let element = self
            .query_descendants()
            .match_type_name(type_name)
            .find_first();
        assert!(
            element.is_some(),
            "Element with type name '{type_name}' not found"
        );
        element.unwrap()
    }

    fn find_all_by_type_name(&self, type_name: &str) -> Vec<ElementHandle> {
        self.query_descendants()
            .match_type_name(type_name)
            .find_all()
    }
}

pub fn setup() -> crate::Result<(MainWindow, Service)> {
    i_slint_backend_testing::init_no_event_loop();
    let service = Service::new(create_test_db());
    let app = MainWindow::new()?;
    let settings = SettingsStore::open_in_memory();
    controllers::bind_all(&app, &service, &settings)?;
    Ok((app, service))
}

/// Finds the element with the given accessible label, panics if the element cannot be found.
#[track_caller]
pub fn find_by_accessible_label(element: &ElementHandle, label: &str) -> ElementHandle {
    element.find_by_accessible_label(label)
}

/// Finds the element with the given type id, panics if the element cannot be found.
#[track_caller]
pub fn find_by_id(element: &ElementHandle, id: &str) -> ElementHandle {
    element.find_by_id(id)
}

/// Finds the element with the given type name, panics if the element cannot be found.
#[track_caller]
pub fn find_by_type_name(element: &ElementHandle, type_name: &str) -> ElementHandle {
    element.find_by_type_name(type_name)
}

/// Finds all the elements with the given type name.
#[track_caller]
pub fn find_all_by_type_name(element: &ElementHandle, type_name: &str) -> Vec<ElementHandle> {
    element.find_all_by_type_name(type_name)
}
