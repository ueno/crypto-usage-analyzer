use crate::data::{AuditEvent, TreeNode};
use crate::models::{StatsObject, TreeNodeObject};
use adw::Banner;
use cairo::Context;
use gtk4::prelude::*;
use gtk4::{gio, ColumnView, Label};
use humantime::{format_duration, format_rfc3339};
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};
use sysinfo::System;

#[derive(Debug, Clone)]
struct Segment {
    node: TreeNode,
    start_angle: f64,
    end_angle: f64,
    inner_radius: f64,
    outer_radius: f64,
    depth: usize,
}

impl Segment {
    fn contains_point(&self, x: f64, y: f64, cx: f64, cy: f64) -> bool {
        let dx = x - cx;
        let dy = y - cy;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < self.inner_radius || distance > self.outer_radius {
            return false;
        }

        let mut angle = dy.atan2(dx);
        if angle < 0.0 {
            angle += 2.0 * PI;
        }

        angle >= self.start_angle && angle <= self.end_angle
    }

    fn format_tooltip(&self) -> String {
        let total = self.node.value;
        let children_count = self.node.children.len();

        let mut tooltip = format!("{}\n", self.node.name);
        tooltip.push_str(&format!("Count: {}\n", total));

        if children_count > 0 {
            tooltip.push_str(&format!("Children: {}\n", children_count));

            // Show top 5 children by value
            let mut sorted_children = self.node.children.clone();
            sorted_children.sort_by(|a, b| b.value.cmp(&a.value));

            if !sorted_children.is_empty() {
                tooltip.push_str("\nTop operations:\n");
                for child in sorted_children.iter().take(5) {
                    let percentage = (child.value as f64 / total as f64 * 100.0).round() as u32;
                    tooltip.push_str(&format!("  • {} ({}%)\n", child.name, percentage));
                }
            }
        }

        tooltip
    }
}

pub struct SunburstChart {
    drawing_area: gtk4::DrawingArea,
    data: Rc<RefCell<Option<TreeNode>>>,
    segments: Rc<RefCell<Vec<Segment>>>,
    hover_segment: Rc<RefCell<Option<usize>>>,
    zoom_node: Rc<RefCell<Option<TreeNode>>>,
    banner: Rc<RefCell<Option<Banner>>>,
    tree_store: Rc<RefCell<Option<gio::ListStore>>>,
    selected_path: Rc<RefCell<Vec<String>>>,
    column_view: Rc<RefCell<Option<ColumnView>>>,
    stats_store: Rc<RefCell<Option<gio::ListStore>>>,
    events: Rc<RefCell<Vec<AuditEvent>>>,
    period_start_label: Rc<RefCell<Option<Label>>>,
    period_end_label: Rc<RefCell<Option<Label>>>,
    period_duration_label: Rc<RefCell<Option<Label>>>,
}

impl SunburstChart {
    pub fn new() -> Self {
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_content_width(700);
        drawing_area.set_content_height(700);
        drawing_area.set_vexpand(true);
        drawing_area.set_hexpand(true);
        drawing_area.set_has_tooltip(true);

        let data = Rc::new(RefCell::new(None));
        let segments = Rc::new(RefCell::new(Vec::new()));
        let hover_segment = Rc::new(RefCell::new(None));
        let zoom_node = Rc::new(RefCell::new(None));
        let banner = Rc::new(RefCell::new(None));
        let tree_store = Rc::new(RefCell::new(None));
        let selected_path = Rc::new(RefCell::new(Vec::new()));
        let stats_store = Rc::new(RefCell::new(None));
        let events = Rc::new(RefCell::new(Vec::new()));
        let period_start_label = Rc::new(RefCell::new(None));
        let period_end_label = Rc::new(RefCell::new(None));
        let period_duration_label = Rc::new(RefCell::new(None));

        let column_view = Rc::new(RefCell::new(None));

        let chart = Self {
            drawing_area: drawing_area.clone(),
            data: data.clone(),
            segments: segments.clone(),
            hover_segment: hover_segment.clone(),
            zoom_node: zoom_node.clone(),
            banner: banner.clone(),
            tree_store: tree_store.clone(),
            selected_path: selected_path.clone(),
            column_view: column_view.clone(),
            stats_store: stats_store.clone(),
            events: events.clone(),
            period_start_label: period_start_label.clone(),
            period_end_label: period_end_label.clone(),
            period_duration_label: period_duration_label.clone(),
        };

        // Set up drawing
        let data_clone = data.clone();
        let segments_clone = segments.clone();
        let hover_clone = hover_segment.clone();
        let zoom_clone = zoom_node.clone();
        let selected_path_clone = selected_path.clone();

        drawing_area.set_draw_func(move |_, cr, width, height| {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint().unwrap();

            let data_ref = data_clone.borrow();
            let zoom_ref = zoom_clone.borrow();

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
            Self::draw_node(
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
                &hover_clone,
                &selected_path_clone,
                &Vec::new(),
            );

            *segments_clone.borrow_mut() = new_segments;
        });

        // Set up mouse motion
        let motion_controller = gtk4::EventControllerMotion::new();
        let hover_clone = hover_segment.clone();
        let segments_clone = segments.clone();
        let drawing_area_clone = drawing_area.clone();

        motion_controller.connect_motion(move |_, x, y| {
            let width = drawing_area_clone.width() as f64;
            let height = drawing_area_clone.height() as f64;
            let cx = width / 2.0;
            let cy = height / 2.0;

            let segments_ref = segments_clone.borrow();
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
                drawing_area_clone.set_tooltip_text(Some(&tooltip_text));
            } else {
                drawing_area_clone.set_tooltip_text(None);
            }

            let mut hover_ref = hover_clone.borrow_mut();
            if *hover_ref != found {
                *hover_ref = found;
                drawing_area_clone.queue_draw();
            }
        });

        drawing_area.add_controller(motion_controller);

        // Set up click handler
        let click_controller = gtk4::GestureClick::new();
        let segments_clone = segments.clone();
        let zoom_clone = zoom_node.clone();
        let drawing_area_clone = drawing_area.clone();
        let banner_clone = banner.clone();
        let tree_store_clone = tree_store.clone();
        let data_clone = data.clone();
        let stats_store_clone = stats_store.clone();

        let selected_path_clone = selected_path.clone();

        click_controller.connect_released(move |_, _, x, y| {
            let width = drawing_area_clone.width() as f64;
            let height = drawing_area_clone.height() as f64;
            let cx = width / 2.0;
            let cy = height / 2.0;

            let segments_ref = segments_clone.borrow();

            for seg in segments_ref.iter().rev() {
                if seg.contains_point(x, y, cx, cy) {
                    if seg.depth == 0 {
                        // Reset zoom on root click
                        *zoom_clone.borrow_mut() = None;
                        // Hide banner
                        if let Some(banner) = banner_clone.borrow().as_ref() {
                            banner.set_revealed(false);
                        }
                        // Restore full tree
                        if let Some(data) = data_clone.borrow().as_ref() {
                            if let Some(store) = tree_store_clone.borrow().as_ref() {
                                store.remove_all();
                                SunburstChart::populate_tree_store(store, data);
                            }
                            // Restore full stats
                            if let Some(store) = stats_store_clone.borrow().as_ref() {
                                SunburstChart::populate_stats_store(store, data);
                            }
                        }
                        // Clear selection highlighting
                        *selected_path_clone.borrow_mut() = Vec::new();
                    } else {
                        // Zoom into this segment
                        *zoom_clone.borrow_mut() = Some(seg.node.clone());
                        // Show banner
                        if let Some(banner) = banner_clone.borrow().as_ref() {
                            banner.set_revealed(true);
                        }
                        // Update tree store to show only the zoomed subtree
                        if let Some(store) = tree_store_clone.borrow().as_ref() {
                            store.remove_all();
                            SunburstChart::populate_tree_store(store, &seg.node);
                        }
                        // Update stats store for the zoomed subtree
                        if let Some(store) = stats_store_clone.borrow().as_ref() {
                            SunburstChart::populate_stats_store(store, &seg.node);
                        }
                        // Clear selection highlighting when zooming
                        *selected_path_clone.borrow_mut() = Vec::new();
                    }
                    drawing_area_clone.queue_draw();
                    break;
                }
            }
        });

        drawing_area.add_controller(click_controller);

        chart
    }

    fn draw_node(
        cr: &Context,
        node: &TreeNode,
        start_angle: f64,
        end_angle: f64,
        inner_radius: f64,
        outer_radius: f64,
        depth: usize,
        segments: &mut Vec<Segment>,
        cx: f64,
        cy: f64,
        hover_segment: &Rc<RefCell<Option<usize>>>,
        selected_path: &Rc<RefCell<Vec<String>>>,
        current_path: &Vec<String>,
    ) {
        if node.value == 0 {
            return;
        }

        let ring_thickness = (outer_radius - inner_radius) / 6.0;
        let current_inner = inner_radius + (depth as f64 * ring_thickness);
        let current_outer = current_inner + ring_thickness;

        if current_outer > outer_radius || current_outer <= current_inner {
            return;
        }

        // Generate color based on node name
        let (r, g, b) = Self::get_color(&node.name, depth);

        let segment_idx = segments.len();
        let is_hovered = *hover_segment.borrow() == Some(segment_idx);

        // Check if this segment is selected via tree view
        let mut path_with_current = current_path.clone();
        path_with_current.push(node.name.clone());
        let is_selected = {
            let selected = selected_path.borrow();
            !selected.is_empty() && *selected == path_with_current
        };

        segments.push(Segment {
            node: node.clone(),
            start_angle,
            end_angle,
            inner_radius: current_inner,
            outer_radius: current_outer,
            depth,
        });

        // Draw the arc
        cr.save().unwrap();

        if is_selected {
            // Highlight selected segment with a bright blue border
            cr.set_source_rgb(r, g, b);
        } else if is_hovered {
            cr.set_source_rgb(r * 1.2, g * 1.2, b * 1.2);
        } else {
            cr.set_source_rgb(r, g, b);
        }

        cr.arc(cx, cy, current_outer, start_angle, end_angle);
        cr.arc_negative(cx, cy, current_inner, end_angle, start_angle);
        cr.close_path();
        cr.fill().unwrap();

        // Draw border
        if is_selected {
            // Thicker, more visible border for selected segment
            cr.set_source_rgb(0.0, 0.4, 0.8);
            cr.set_line_width(3.0);
        } else {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.set_line_width(1.0);
        }
        cr.arc(cx, cy, current_outer, start_angle, end_angle);
        cr.arc_negative(cx, cy, current_inner, end_angle, start_angle);
        cr.close_path();
        cr.stroke().unwrap();

        cr.restore().unwrap();

        // Draw children
        if !node.children.is_empty() && depth < 5 {
            let angle_span = end_angle - start_angle;
            let mut current_angle = start_angle;

            for child in &node.children {
                let child_angle_span = angle_span * (child.value as f64 / node.value as f64);
                let child_end_angle = current_angle + child_angle_span;

                Self::draw_node(
                    cr,
                    child,
                    current_angle,
                    child_end_angle,
                    inner_radius,
                    outer_radius,
                    depth + 1,
                    segments,
                    cx,
                    cy,
                    hover_segment,
                    selected_path,
                    &path_with_current,
                );

                current_angle = child_end_angle;
            }
        }
    }

    fn get_color(name: &str, depth: usize) -> (f64, f64, f64) {
        // Simple hash-based color generation
        let mut hash: u32 = depth as u32 * 100;
        for byte in name.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }

        let hue = (hash % 360) as f64 / 360.0;
        let saturation = 0.6 + ((hash / 360) % 20) as f64 / 100.0;
        let value = 0.7 + ((hash / 7200) % 20) as f64 / 100.0;

        // Convert HSV to RGB
        let h = hue * 6.0;
        let i = h.floor();
        let f = h - i;
        let p = value * (1.0 - saturation);
        let q = value * (1.0 - saturation * f);
        let t = value * (1.0 - saturation * (1.0 - f));

        match i as i32 % 6 {
            0 => (value, t, p),
            1 => (q, value, p),
            2 => (p, value, t),
            3 => (p, q, value),
            4 => (t, p, value),
            _ => (value, p, q),
        }
    }

    pub fn set_data(&self, data: TreeNode, events: Vec<AuditEvent>) {
        *self.data.borrow_mut() = Some(data.clone());
        *self.events.borrow_mut() = events;
        *self.zoom_node.borrow_mut() = None;

        // Hide banner when loading new data
        if let Some(banner) = self.banner.borrow().as_ref() {
            banner.set_revealed(false);
        }

        // Populate tree store
        if let Some(store) = self.tree_store.borrow().as_ref() {
            store.remove_all();
            Self::populate_tree_store(store, &data);
        }

        // Populate stats store
        if let Some(store) = self.stats_store.borrow().as_ref() {
            Self::populate_stats_store(store, &data);
        }

        // Update period labels
        self.update_period_labels();

        self.drawing_area.queue_draw();
    }

    fn populate_tree_store(store: &gio::ListStore, node: &TreeNode) {
        for child in &node.children {
            let child_obj = Self::tree_node_to_object(child);
            store.append(&child_obj);
        }
    }

    fn tree_node_to_object(node: &TreeNode) -> TreeNodeObject {
        let obj = TreeNodeObject::new(&node.name, &node.value.to_string(), node.value as u32);

        if !node.children.is_empty() {
            let children_store = gio::ListStore::new::<TreeNodeObject>();
            for child in &node.children {
                let child_obj = Self::tree_node_to_object(child);
                children_store.append(&child_obj);
            }
            obj.set_children(Some(children_store));
        }

        obj
    }

    pub fn set_tree_store(&self, tree_store: gio::ListStore) {
        *self.tree_store.borrow_mut() = Some(tree_store);
    }

    pub fn set_column_view(&self, column_view: ColumnView) {
        *self.column_view.borrow_mut() = Some(column_view);
    }

    pub fn set_stats_store(&self, stats_store: gio::ListStore) {
        *self.stats_store.borrow_mut() = Some(stats_store);
    }

    pub fn set_period_labels(&self, start_label: Label, end_label: Label, duration_label: Label) {
        *self.period_start_label.borrow_mut() = Some(start_label);
        *self.period_end_label.borrow_mut() = Some(end_label);
        *self.period_duration_label.borrow_mut() = Some(duration_label);
    }

    fn update_period_labels(&self) {
        let events = self.events.borrow();

        if let Some((start_ns, end_ns)) = AuditEvent::get_time_range(&events) {
            // Format as human-readable dates
            let boot_time = UNIX_EPOCH + Duration::from_secs(System::boot_time());

            let start_time = boot_time + Duration::from_nanos(start_ns);
            let end_time = boot_time + Duration::from_nanos(end_ns);

            // Format using chrono-like formatting (we'll use simple formatting here)
            let start_text = format!("Start: {}", format_rfc3339(start_time));
            let end_text = format!("End: {}", format_rfc3339(end_time));

            // Calculate duration
            if let Ok(duration) = end_time.duration_since(start_time) {
                let duration_text = format_duration(duration);

                if let Some(label) = self.period_start_label.borrow().as_ref() {
                    label.set_text(&start_text);
                }
                if let Some(label) = self.period_end_label.borrow().as_ref() {
                    label.set_text(&end_text);
                }
                if let Some(label) = self.period_duration_label.borrow().as_ref() {
                    label.set_text(&format!("Duration: {}", duration_text));
                }
            }
        }
    }

    fn populate_stats_store(store: &gio::ListStore, node: &TreeNode) {
        store.remove_all();

        let mut stats: HashMap<String, usize> = HashMap::new();
        node.extract_algorithm_stats(&mut stats);

        if stats.is_empty() {
            return;
        }

        // Calculate total for percentages
        let total: usize = stats.values().sum();

        // Sort by count (descending)
        let mut stats_vec: Vec<_> = stats.into_iter().collect();
        stats_vec.sort_by(|a, b| b.1.cmp(&a.1));

        // Populate store
        for (algorithm, count) in stats_vec {
            let percentage = if total > 0 {
                (count as f64 / total as f64 * 100.0).round() as u32
            } else {
                0
            };

            let stats_obj =
                StatsObject::new(&algorithm, &count.to_string(), &format!("{}%", percentage));
            store.append(&stats_obj);
        }
    }

    pub fn set_selected_path(&self, path: Vec<String>) {
        *self.selected_path.borrow_mut() = path;
        self.drawing_area.queue_draw();
    }

    pub fn set_zoom_banner(&self, banner: Banner) {
        // Set up banner button to reset zoom
        let zoom_clone = self.zoom_node.clone();
        let drawing_area_clone = self.drawing_area.clone();
        let banner_clone = banner.clone();
        let data_clone = self.data.clone();
        let tree_store_clone = self.tree_store.clone();
        let stats_store_clone = self.stats_store.clone();
        let selected_path_clone = self.selected_path.clone();

        banner.connect_button_clicked(move |_| {
            *zoom_clone.borrow_mut() = None;
            banner_clone.set_revealed(false);

            // Restore full tree
            if let Some(data) = data_clone.borrow().as_ref() {
                if let Some(store) = tree_store_clone.borrow().as_ref() {
                    store.remove_all();
                    SunburstChart::populate_tree_store(store, data);
                }
                // Restore full stats
                if let Some(store) = stats_store_clone.borrow().as_ref() {
                    SunburstChart::populate_stats_store(store, data);
                }
            }

            // Clear selection highlighting
            *selected_path_clone.borrow_mut() = Vec::new();

            drawing_area_clone.queue_draw();
        });

        *self.banner.borrow_mut() = Some(banner);
    }

    pub fn widget(&self) -> &gtk4::DrawingArea {
        &self.drawing_area
    }
}
