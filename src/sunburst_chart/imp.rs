use crate::data::{AuditEvent, TreeNode};
use crate::models::{StatsObject, TreeNodeObject};
use adw::Banner;
use cairo::Context;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gio, glib, ColumnView, Label};
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct Segment {
    pub node: TreeNode,
    pub start_angle: f64,
    pub end_angle: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub depth: usize,
}

impl Segment {
    pub fn contains_point(&self, x: f64, y: f64, cx: f64, cy: f64) -> bool {
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

    pub fn format_tooltip(&self) -> String {
        self.node.name.to_string()
    }
}

#[derive(Debug, Default)]
pub struct SunburstChart {
    pub data: RefCell<Option<TreeNode>>,
    pub original_data: RefCell<Option<TreeNode>>,
    pub detailed_view: RefCell<bool>,
    pub segments: RefCell<Vec<Segment>>,
    pub hover_segment: RefCell<Option<usize>>,
    pub zoom_node: RefCell<Option<TreeNode>>,
    pub banner: RefCell<Option<Banner>>,
    pub tree_store: RefCell<Option<gio::ListStore>>,
    pub selected_path: RefCell<Vec<String>>,
    pub column_view: RefCell<Option<ColumnView>>,
    pub stats_store: RefCell<Option<gio::ListStore>>,
    pub event_stats_store: RefCell<Option<gio::ListStore>>,
    pub events: RefCell<Vec<AuditEvent>>,
    pub period_start_label: RefCell<Option<Label>>,
    pub period_end_label: RefCell<Option<Label>>,
    pub period_duration_label: RefCell<Option<Label>>,
}

#[glib::object_subclass]
impl ObjectSubclass for SunburstChart {
    const NAME: &'static str = "SunburstChart";
    type Type = super::SunburstChart;
    type ParentType = gtk4::DrawingArea;
}

impl ObjectImpl for SunburstChart {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        obj.set_content_width(300);
        obj.set_content_height(300);
        obj.set_vexpand(true);
        obj.set_hexpand(true);
        obj.set_has_tooltip(true);

        // Set up drawing
        obj.set_draw_func(glib::clone!(
            #[weak(rename_to = chart)]
            obj,
            move |_, cr, width, height| {
                chart.draw_chart(cr, width, height);
            }
        ));

        // Set up mouse motion
        let motion_controller = gtk4::EventControllerMotion::new();
        motion_controller.connect_motion(glib::clone!(
            #[weak(rename_to = chart)]
            obj,
            move |_, x, y| {
                chart.handle_motion(x, y);
            }
        ));
        obj.add_controller(motion_controller);

        // Set up click handler
        let click_controller = gtk4::GestureClick::new();
        click_controller.connect_released(glib::clone!(
            #[weak(rename_to = chart)]
            obj,
            move |_, _, x, y| {
                chart.handle_click(x, y);
            }
        ));
        obj.add_controller(click_controller);
    }
}

impl WidgetImpl for SunburstChart {}

impl DrawingAreaImpl for SunburstChart {}

impl SunburstChart {
    pub fn draw_node(
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
        hover_segment: &RefCell<Option<usize>>,
        selected_path: &RefCell<Vec<String>>,
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

        // Draw border (only the arcs, not the radial lines)
        if is_selected {
            // Thicker, more visible border for selected segment
            cr.set_source_rgb(0.0, 0.4, 0.8);
            cr.set_line_width(3.0);
        } else {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.set_line_width(1.0);
        }

        // Only draw the outer and inner arcs, skip radial lines
        // This prevents the line from center to east at angle 0
        cr.new_path();
        cr.arc(cx, cy, current_outer, start_angle, end_angle);
        cr.stroke().unwrap();

        cr.new_path();
        cr.arc(cx, cy, current_inner, start_angle, end_angle);
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

    pub fn get_color(name: &str, depth: usize) -> (f64, f64, f64) {
        // Simple hash-based color generation
        // Use only the name for hashing to ensure consistent colors when zooming
        let mut hash: u32 = 0;
        for byte in name.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }

        let hue = (hash % 360) as f64 / 360.0;
        let saturation = 0.6 + ((hash / 360) % 20) as f64 / 100.0;
        // Use depth for value variation to distinguish between rings visually
        let value = 0.5 + (depth as f64 * 0.08).min(0.4);

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

    pub fn populate_tree_store(store: &gio::ListStore, node: &TreeNode) {
        for child in &node.children {
            let child_obj = Self::tree_node_to_object(child);
            store.append(&child_obj);
        }
    }

    pub fn tree_node_to_object(node: &TreeNode) -> TreeNodeObject {
        let obj = TreeNodeObject::new(&node.name, node.value as u64);

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

    pub fn populate_stats_store(store: &gio::ListStore, node: &TreeNode) {
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
        for (name, count) in stats_vec {
            let stats_obj = StatsObject::new(&name, count as u64, total as u64);
            store.append(&stats_obj);
        }
    }

    pub fn populate_event_stats_store(store: &gio::ListStore, node: &TreeNode) {
        store.remove_all();

        let mut stats: HashMap<String, usize> = HashMap::new();
        node.extract_event_stats(&mut stats);

        if stats.is_empty() {
            return;
        }

        // Calculate total for percentages
        let total: usize = stats.values().sum();

        // Sort by count (descending)
        let mut stats_vec: Vec<_> = stats.into_iter().collect();
        stats_vec.sort_by(|a, b| b.1.cmp(&a.1));

        for (event_name, count) in stats_vec {
            let stats_obj = StatsObject::new(&event_name, count as u64, total as u64);
            store.append(&stats_obj);
        }
    }

    pub fn draw_child_captions(
        cr: &Context,
        segments: &[Segment],
        hover_idx: usize,
        cx: f64,
        cy: f64,
    ) {
        let hovered_segment = &segments[hover_idx];
        let hovered_depth = hovered_segment.depth;

        // Collect direct children of the hovered segment
        let mut child_segments: Vec<&Segment> = Vec::new();
        for seg in segments.iter() {
            if seg.depth == hovered_depth + 1 {
                // Check if this segment is within the hovered segment's angle range
                if seg.start_angle >= hovered_segment.start_angle
                    && seg.end_angle <= hovered_segment.end_angle
                {
                    child_segments.push(seg);
                }
            }
        }

        if child_segments.is_empty() {
            return;
        }

        // Draw captions for each child
        for child_seg in child_segments {
            let middle_angle = (child_seg.start_angle + child_seg.end_angle) / 2.0;
            let middle_radius = (child_seg.inner_radius + child_seg.outer_radius) / 2.0;

            // Calculate the center point of the segment
            let seg_center_x = cx + middle_radius * middle_angle.cos();
            let seg_center_y = cy + middle_radius * middle_angle.sin();

            // Calculate label position - extend outward from the segment
            let label_distance = child_seg.outer_radius + 30.0;
            let label_x = cx + label_distance * middle_angle.cos();
            let label_y = cy + label_distance * middle_angle.sin();

            // Draw line from segment center to label position
            cr.save().unwrap();
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.8);
            cr.set_line_width(1.0);
            cr.move_to(seg_center_x, seg_center_y);
            cr.line_to(label_x, label_y);
            cr.stroke().unwrap();

            // Draw small circle at segment center
            cr.arc(seg_center_x, seg_center_y, 2.0, 0.0, 2.0 * PI);
            cr.fill().unwrap();

            // Draw label text
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(10.0);

            let text = &child_seg.node.name;
            let text_extents = cr.text_extents(text).unwrap();

            // Position text based on which quadrant the label is in
            let text_x = if label_x > cx {
                label_x + 5.0
            } else {
                label_x - text_extents.width() - 5.0
            };
            let text_y = label_y + text_extents.height() / 2.0;

            // Draw text background
            let padding = 2.0;
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
            cr.rectangle(
                text_x - padding,
                text_y - text_extents.height() - padding,
                text_extents.width() + 2.0 * padding,
                text_extents.height() + 2.0 * padding,
            );
            cr.fill().unwrap();

            // Draw text
            cr.set_source_rgb(0.0, 0.0, 0.0);
            cr.move_to(text_x, text_y);
            cr.show_text(text).unwrap();

            cr.restore().unwrap();
        }
    }
}
