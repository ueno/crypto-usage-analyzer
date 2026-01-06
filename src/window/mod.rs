mod imp;

use crate::data::{AuditEvent, TreeNode};
use anyhow::Result;
use gtk4::{gio, glib, prelude::*, subclass::prelude::*};
use std::fs;
use std::path::Path;
use std::process::Command;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub fn set_path(&self, path: impl AsRef<Path>) {
        *self.imp().path.borrow_mut() = Some(path.as_ref().to_path_buf());
        self.imp().reload_button.set_sensitive(true);
    }

    pub fn reload(&self) -> Result<()> {
        let events: Vec<AuditEvent> =
            if let Ok(content) = Command::new("crau-query").output() {
                serde_json::from_slice(&content.stdout)?
            } else if let Some(ref path) = *self.imp().path.borrow() {
                let content = fs::read_to_string(path)?;
                serde_json::from_str(&content)?
            } else {
                return Ok(())
            };

        let tree = TreeNode::from_events(&events);
        self.imp().sunburst_chart.set_data(tree, events);
        self.imp().main_stack.set_visible_child_name("content");

        Ok(())
    }

    pub fn show_toast(&self, message: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(message));
    }
}
