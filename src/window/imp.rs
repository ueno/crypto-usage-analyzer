use crate::data::Source;
use crate::models::{StatsObject, TreeNodeObject};
use crate::sunburst_chart::SunburstChart;
use adw::subclass::prelude::*;
use gtk4::{
    gio, glib, prelude::*, Button, CompositeTemplate, Label, ListItem, SignalListItemFactory,
    SingleSelection, TreeListModel, TreeListRow,
};
use std::cell::RefCell;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/gnome/CryptoUsageAnalyzer/ui/window.ui")]
pub struct Window {
    #[template_child]
    pub toast_overlay: TemplateChild<adw::ToastOverlay>,
    #[template_child]
    pub main_stack: TemplateChild<gtk4::Stack>,
    #[template_child]
    pub menu_button: TemplateChild<gtk4::MenuButton>,
    #[template_child]
    pub reload_button: TemplateChild<gtk4::Button>,
    #[template_child]
    pub tree_view: TemplateChild<gtk4::ColumnView>,
    #[template_child]
    pub stats_view: TemplateChild<gtk4::ColumnView>,
    #[template_child]
    pub stats_store: TemplateChild<gio::ListStore>,
    #[template_child]
    pub event_stats_view: TemplateChild<gtk4::ColumnView>,
    #[template_child]
    pub event_stats_store: TemplateChild<gio::ListStore>,
    #[template_child]
    pub sunburst_chart: TemplateChild<crate::sunburst_chart::SunburstChart>,
    #[template_child]
    pub zoom_banner: TemplateChild<adw::Banner>,
    #[template_child]
    pub period_start_label: TemplateChild<gtk4::Label>,
    #[template_child]
    pub period_end_label: TemplateChild<gtk4::Label>,
    #[template_child]
    pub period_duration_label: TemplateChild<gtk4::Label>,

    pub source: RefCell<Option<Source>>,
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "CryptoUsageAnalyzerWindow";
    type Type = super::Window;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        // Register custom widget types before loading UI
        SunburstChart::ensure_type();
        StatsObject::ensure_type();
        TreeNodeObject::ensure_type();

        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[gtk4::template_callbacks]
impl Window {
    #[template_callback]
    fn tree_name_column_setup(_: &SignalListItemFactory, list_item: &ListItem) {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    }

    #[template_callback]
    fn tree_name_column_bind(_: &SignalListItemFactory, list_item: &ListItem) {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let tree_list_row = list_item.item().and_downcast::<TreeListRow>().unwrap();
        let tree_node = tree_list_row
            .item()
            .and_downcast::<TreeNodeObject>()
            .unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();

        // Add indentation based on depth
        let depth = tree_list_row.depth();
        let indent = "  ".repeat(depth as usize);
        label.set_text(&format!("{}{}", indent, tree_node.name()));
    }

    #[template_callback]
    fn name_column_setup(_: &SignalListItemFactory, list_item: &ListItem) {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(4);
        label.set_margin_end(4);
        label.set_wrap(true);
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        label.set_max_width_chars(30);
        list_item.set_child(Some(&label));
    }

    #[template_callback]
    fn tree_count_column_setup(_: &SignalListItemFactory, list_item: &ListItem) {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    }

    #[template_callback]
    fn tree_count_column_bind(_: &SignalListItemFactory, list_item: &ListItem) {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let tree_list_row = list_item.item().and_downcast::<TreeListRow>().unwrap();
        let tree_node = tree_list_row
            .item()
            .and_downcast::<TreeNodeObject>()
            .unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&tree_node.count());
    }

    #[template_callback]
    fn name_column_bind(_: &SignalListItemFactory, list_item: &ListItem) {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.algorithm());
    }

    #[template_callback]
    fn count_column_setup(_: &SignalListItemFactory, list_item: &ListItem) {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    }

    #[template_callback]
    fn count_column_bind(_: &SignalListItemFactory, list_item: &ListItem) {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.count());
    }

    #[template_callback]
    fn percentage_column_setup(_: &SignalListItemFactory, list_item: &ListItem) {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    }

    #[template_callback]
    fn percentage_column_bind(_: &SignalListItemFactory, list_item: &ListItem) {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.percentage());
    }

    #[template_callback]
    fn open_button_clicked(button: &Button) {
        button
            .activate_action("app.open", None)
            .expect("action should exist");
    }

    #[template_callback]
    fn reload_button_clicked(button: &Button) {
        button
            .activate_action("app.reload", None)
            .expect("action should exist");
    }
}

impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();

        // Create tree list model for the tree view
        let root_store = gio::ListStore::new::<TreeNodeObject>();

        let tree_model = TreeListModel::new(
            root_store.clone(),
            false, // passthrough
            true,  // autoexpand
            |item| {
                let tree_node = item.downcast_ref::<TreeNodeObject>().unwrap();
                tree_node.children().map(gio::ListModel::from)
            },
        );

        let selection_model = SingleSelection::new(Some(tree_model));
        self.tree_view.set_model(Some(&selection_model));

        // Setup sunburst chart
        self.sunburst_chart
            .set_zoom_banner(self.zoom_banner.clone());
        self.sunburst_chart.set_tree_store(root_store.clone());
        self.sunburst_chart.set_column_view(self.tree_view.clone());
        self.sunburst_chart
            .set_stats_store(self.stats_store.clone());
        self.sunburst_chart
            .set_event_stats_store(self.event_stats_store.clone());
        self.sunburst_chart.set_period_labels(
            self.period_start_label.clone(),
            self.period_end_label.clone(),
            self.period_duration_label.clone(),
        );

        // Connect tree selection to chart highlighting
        let chart_clone = self.sunburst_chart.clone();
        selection_model.connect_selection_changed(move |selection, _, _| {
            if let Some(selected_item) = selection.selected_item() {
                if let Some(tree_list_row) = selected_item.downcast_ref::<TreeListRow>() {
                    // Build path from root to selected node
                    let mut path = Vec::new();
                    let mut current_row = Some(tree_list_row.clone());

                    while let Some(row) = current_row {
                        if let Some(node) = row.item().and_downcast::<TreeNodeObject>() {
                            path.insert(0, node.name());
                        }
                        current_row = row.parent();
                    }

                    chart_clone.set_selected_path(path);
                }
            } else {
                chart_clone.set_selected_path(Vec::new());
            }
        });
    }
}

impl WidgetImpl for Window {}

impl WindowImpl for Window {}

impl ApplicationWindowImpl for Window {}

impl AdwApplicationWindowImpl for Window {}
