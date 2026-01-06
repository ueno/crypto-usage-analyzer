mod imp;

use crate::data::{AuditEvent, TreeNode};
use adw::Banner;
use cairo::Context;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gio, glib, ColumnView, Label};
use std::f64::consts::PI;
use std::time::{Duration, UNIX_EPOCH};
use sysinfo::System;

glib::wrapper! {
    pub struct SunburstChart(ObjectSubclass<imp::SunburstChart>)
        @extends gtk4::DrawingArea, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Default for SunburstChart {
    fn default() -> Self {
        Self::new()
    }
}

impl SunburstChart {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_data(&self, data: TreeNode, events: Vec<AuditEvent>) {
        let imp = self.imp();

        // Store original data
        *imp.original_data.borrow_mut() = Some(data.clone());

        // Apply simple view if enabled
        let display_data = if *imp.simple_view.borrow() {
            data.create_simple_view()
        } else {
            data.clone()
        };

        *imp.data.borrow_mut() = Some(display_data.clone());
        *imp.events.borrow_mut() = events;
        *imp.zoom_node.borrow_mut() = None;

        // Hide banner when loading new data
        if let Some(banner) = imp.banner.borrow().as_ref() {
            banner.set_revealed(false);
        }

        // Populate tree store
        if let Some(store) = imp.tree_store.borrow().as_ref() {
            store.remove_all();
            imp::SunburstChart::populate_tree_store(store, &display_data);
        }

        // Populate stats store
        if let Some(store) = imp.stats_store.borrow().as_ref() {
            imp::SunburstChart::populate_stats_store(store, &display_data);
        }

        // Populate event stats store
        if let Some(store) = imp.event_stats_store.borrow().as_ref() {
            imp::SunburstChart::populate_event_stats_store(store, &display_data);
        }

        // Update period labels
        self.update_period_labels();

        self.queue_draw();
    }

    pub fn toggle_simple_view(&self) {
        let imp = self.imp();

        // Toggle the flag
        let new_value = !*imp.simple_view.borrow();
        *imp.simple_view.borrow_mut() = new_value;

        // Recompute the data view
        if let Some(original) = imp.original_data.borrow().as_ref() {
            let display_data = if new_value {
                original.create_simple_view()
            } else {
                original.clone()
            };

            *imp.data.borrow_mut() = Some(display_data.clone());

            // Reset zoom when toggling view
            *imp.zoom_node.borrow_mut() = None;

            // Hide banner
            if let Some(banner) = imp.banner.borrow().as_ref() {
                banner.set_revealed(false);
            }

            // Update tree store
            if let Some(store) = imp.tree_store.borrow().as_ref() {
                store.remove_all();
                imp::SunburstChart::populate_tree_store(store, &display_data);
            }

            // Update stats store
            if let Some(store) = imp.stats_store.borrow().as_ref() {
                imp::SunburstChart::populate_stats_store(store, &display_data);
            }

            // Update event stats store
            if let Some(store) = imp.event_stats_store.borrow().as_ref() {
                imp::SunburstChart::populate_event_stats_store(store, &display_data);
            }

            // Clear selection
            *imp.selected_path.borrow_mut() = Vec::new();

            self.queue_draw();
        }
    }

    pub fn is_simple_view(&self) -> bool {
        *self.imp().simple_view.borrow()
    }

    pub fn set_tree_store(&self, tree_store: gio::ListStore) {
        *self.imp().tree_store.borrow_mut() = Some(tree_store);
    }

    pub fn set_column_view(&self, column_view: ColumnView) {
        *self.imp().column_view.borrow_mut() = Some(column_view);
    }

    pub fn set_stats_store(&self, stats_store: gio::ListStore) {
        *self.imp().stats_store.borrow_mut() = Some(stats_store);
    }

    pub fn set_event_stats_store(&self, event_stats_store: gio::ListStore) {
        *self.imp().event_stats_store.borrow_mut() = Some(event_stats_store);
    }

    pub fn set_period_labels(&self, start_label: Label, end_label: Label, duration_label: Label) {
        let imp = self.imp();
        *imp.period_start_label.borrow_mut() = Some(start_label);
        *imp.period_end_label.borrow_mut() = Some(end_label);
        *imp.period_duration_label.borrow_mut() = Some(duration_label);
    }

    fn update_period_labels(&self) {
        let imp = self.imp();
        let events = imp.events.borrow();

        if let Some((start_ns, end_ns)) = AuditEvent::get_time_range(&events) {
            // Format as human-readable dates
            let boot_time = UNIX_EPOCH + Duration::from_secs(System::boot_time());

            let start_time = boot_time + Duration::from_nanos(start_ns);
            let end_time = boot_time + Duration::from_nanos(end_ns);

            let start_time: jiff::Timestamp = start_time.try_into().unwrap();
            let end_time: jiff::Timestamp = end_time.try_into().unwrap();
            let start_text = format!("Start: {}", start_time.strftime("%c"));
            let end_text = format!("End: {}", end_time.strftime("%c"));

            // Calculate duration
            let duration = end_time.duration_since(start_time);
            if let Some(label) = imp.period_start_label.borrow().as_ref() {
                label.set_text(&start_text);
            }
            if let Some(label) = imp.period_end_label.borrow().as_ref() {
                label.set_text(&end_text);
            }
            if let Some(label) = imp.period_duration_label.borrow().as_ref() {
                label.set_text(&format!("Duration: {duration:#}"));
            }
        }
    }

    pub fn set_selected_path(&self, path: Vec<String>) {
        *self.imp().selected_path.borrow_mut() = path;
        self.queue_draw();
    }

    pub fn set_zoom_banner(&self, banner: Banner) {
        let imp = self.imp();

        // Set up banner button to reset zoom
        banner.connect_button_clicked(glib::clone!(
            #[weak(rename_to = chart)]
            self,
            move |_| {
                let imp = chart.imp();
                *imp.zoom_node.borrow_mut() = None;

                if let Some(banner) = imp.banner.borrow().as_ref() {
                    banner.set_revealed(false);
                }

                // Restore full tree
                if let Some(data) = imp.data.borrow().as_ref() {
                    if let Some(store) = imp.tree_store.borrow().as_ref() {
                        store.remove_all();
                        imp::SunburstChart::populate_tree_store(store, data);
                    }
                    // Restore full stats
                    if let Some(store) = imp.stats_store.borrow().as_ref() {
                        imp::SunburstChart::populate_stats_store(store, data);
                    }
                    // Restore full event stats
                    if let Some(store) = imp.event_stats_store.borrow().as_ref() {
                        imp::SunburstChart::populate_event_stats_store(store, data);
                    }
                }

                // Clear selection highlighting
                *imp.selected_path.borrow_mut() = Vec::new();

                chart.queue_draw();
            }
        ));

        *imp.banner.borrow_mut() = Some(banner);
    }

    pub(crate) fn draw_chart(&self, cr: &Context, width: i32, height: i32) {
        let imp = self.imp();

        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint().unwrap();

        let data_ref = imp.data.borrow();
        let zoom_ref = imp.zoom_node.borrow();

        let root = if let Some(zoom) = zoom_ref.as_ref() {
            zoom
        } else if let Some(data) = data_ref.as_ref() {
            data
        } else {
            return;
        };

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let max_radius = cx.min(cy) - 20.0;

        let mut new_segments = Vec::new();
        imp::SunburstChart::draw_node(
            cr,
            root,
            0.0,
            2.0 * PI,
            0.0,
            max_radius,
            0,
            &mut new_segments,
            cx,
            cy,
            &imp.hover_segment,
            &imp.selected_path,
            &Vec::new(),
        );

        *imp.segments.borrow_mut() = new_segments.clone();

        // Draw child captions if hovering over a segment
        if let Some(hover_idx) = *imp.hover_segment.borrow() {
            imp::SunburstChart::draw_child_captions(
                cr,
                &new_segments,
                hover_idx,
                cx,
                cy,
                width as f64,
                height as f64,
            );
        }
    }

    pub(crate) fn handle_motion(&self, x: f64, y: f64) {
        let imp = self.imp();

        let width = self.width() as f64;
        let height = self.height() as f64;
        let cx = width / 2.0;
        let cy = height / 2.0;

        let segments_ref = imp.segments.borrow();
        let mut found = None;

        for (i, seg) in segments_ref.iter().enumerate().rev() {
            if seg.contains_point(x, y, cx, cy) {
                found = Some(i);
                break;
            }
        }

        // Update tooltip
        if let Some(idx) = found {
            let tooltip_text = segments_ref[idx].format_tooltip();
            self.set_tooltip_text(Some(&tooltip_text));
        } else {
            self.set_tooltip_text(None);
        }

        let mut hover_ref = imp.hover_segment.borrow_mut();
        if *hover_ref != found {
            *hover_ref = found;
            self.queue_draw();
        }
    }

    pub(crate) fn handle_click(&self, x: f64, y: f64) {
        let imp = self.imp();

        let width = self.width() as f64;
        let height = self.height() as f64;
        let cx = width / 2.0;
        let cy = height / 2.0;

        let segments_ref = imp.segments.borrow();

        for seg in segments_ref.iter().rev() {
            if seg.contains_point(x, y, cx, cy) {
                if seg.depth == 0 {
                    // Reset zoom on root click
                    *imp.zoom_node.borrow_mut() = None;
                    // Hide banner
                    if let Some(banner) = imp.banner.borrow().as_ref() {
                        banner.set_revealed(false);
                    }
                    // Restore full tree
                    if let Some(data) = imp.data.borrow().as_ref() {
                        if let Some(store) = imp.tree_store.borrow().as_ref() {
                            store.remove_all();
                            imp::SunburstChart::populate_tree_store(store, data);
                        }
                        // Restore full stats
                        if let Some(store) = imp.stats_store.borrow().as_ref() {
                            imp::SunburstChart::populate_stats_store(store, data);
                        }
                        // Restore full event stats
                        if let Some(store) = imp.event_stats_store.borrow().as_ref() {
                            imp::SunburstChart::populate_event_stats_store(store, data);
                        }
                    }
                    // Clear selection highlighting
                    *imp.selected_path.borrow_mut() = Vec::new();
                } else {
                    // Zoom into this segment
                    *imp.zoom_node.borrow_mut() = Some(seg.node.clone());
                    // Show banner
                    if let Some(banner) = imp.banner.borrow().as_ref() {
                        banner.set_revealed(true);
                    }
                    // Update tree store to show only the zoomed subtree
                    if let Some(store) = imp.tree_store.borrow().as_ref() {
                        store.remove_all();
                        imp::SunburstChart::populate_tree_store(store, &seg.node);
                    }
                    // Update stats store for the zoomed subtree
                    if let Some(store) = imp.stats_store.borrow().as_ref() {
                        imp::SunburstChart::populate_stats_store(store, &seg.node);
                    }
                    // Update event stats store for the zoomed subtree
                    if let Some(store) = imp.event_stats_store.borrow().as_ref() {
                        imp::SunburstChart::populate_event_stats_store(store, &seg.node);
                    }
                    // Clear selection highlighting when zooming
                    *imp.selected_path.borrow_mut() = Vec::new();
                }
                self.queue_draw();
                break;
            }
        }
    }
}
