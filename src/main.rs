mod data;
mod models;
mod sunburst_chart;
mod window;

use adw::prelude::*;
use adw::{glib, AboutWindow, Application, Banner};
use anyhow::Result;
use data::{AuditEvent, TreeNode};
use gtk4::{
    gio, subclass::prelude::*, Button, ColumnView, ColumnViewColumn, Label, ListItem, SignalListItemFactory,
    SingleSelection, Stack, TreeListModel, TreeListRow,
};
use models::{StatsObject, TreeNodeObject};
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use sunburst_chart::SunburstChart;
use window::Window;

const APP_ID: &str = "org.gnome.CryptoUsageAnalyzer";

fn main() -> glib::ExitCode {
    // Load resources
    let resources_bytes = include_bytes!(concat!(
        env!("MESON_BUILD_ROOT"),
        "/data/org.gnome.CryptoUsageAnalyzer.gresource"
    ));
    let resource_data = glib::Bytes::from_static(resources_bytes);
    let resources = gio::Resource::from_data(&resource_data).expect("Failed to load resources");
    gio::resources_register(&resources);

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    // Create the window
    let window = Window::new(app);

    // Load menu from resources
    let menu_builder = gtk4::Builder::from_resource("/org/gnome/CryptoUsageAnalyzer/ui/menu.ui");
    let menu: gio::Menu = menu_builder.object("primary_menu").expect("Failed to get primary_menu");

    // Set menu model on menu button
    window.imp().menu_button.set_menu_model(Some(&menu));

    // Create tree list model for the tree view
    let root_store = gio::ListStore::new::<TreeNodeObject>();

    let tree_model = TreeListModel::new(
        root_store.clone(),
        false, // passthrough
        true,  // autoexpand
        |item| {
            let tree_node = item.downcast_ref::<TreeNodeObject>().unwrap();
            tree_node
                .children()
                .map(gio::ListModel::from)
        },
    );

    let selection_model = SingleSelection::new(Some(tree_model));
    window.imp().column_view.set_model(Some(&selection_model));

    // Create "Operation" column
    let name_factory = SignalListItemFactory::new();
    name_factory.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    });
    name_factory.connect_bind(|_, list_item| {
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
    });

    let name_column = ColumnViewColumn::new(Some("Operation"), Some(name_factory));
    name_column.set_expand(true);
    window.imp().column_view.append_column(&name_column);

    // Create "Count" column
    let count_factory = SignalListItemFactory::new();
    count_factory.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    });
    count_factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let tree_list_row = list_item.item().and_downcast::<TreeListRow>().unwrap();
        let tree_node = tree_list_row
            .item()
            .and_downcast::<TreeNodeObject>()
            .unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&tree_node.count());
    });

    let count_column = ColumnViewColumn::new(Some("Count"), Some(count_factory));
    window.imp().column_view.append_column(&count_column);

    // Create list stores for statistics views
    let stats_store = gio::ListStore::new::<StatsObject>();
    let stats_selection = SingleSelection::new(Some(stats_store.clone()));
    window.imp().stats_view.set_model(Some(&stats_selection));

    let event_stats_store = gio::ListStore::new::<StatsObject>();
    let event_stats_selection = SingleSelection::new(Some(event_stats_store.clone()));
    window.imp().event_stats_view.set_model(Some(&event_stats_selection));

    // Setup columns for event statistics view (event_stats_view)
    // "Event" column for event stats
    let event_factory = SignalListItemFactory::new();
    event_factory.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(4);
        label.set_margin_end(4);
        label.set_wrap(true);
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        label.set_max_width_chars(30);
        list_item.set_child(Some(&label));
    });
    event_factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.algorithm());
    });
    let event_column = ColumnViewColumn::new(Some("Event"), Some(event_factory));
    event_column.set_expand(true);
    window.imp().event_stats_view.append_column(&event_column);

    // Create "Count" column for event stats
    let event_count_factory = SignalListItemFactory::new();
    event_count_factory.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    });
    event_count_factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.count());
    });
    let event_count_column = ColumnViewColumn::new(Some("Count"), Some(event_count_factory));
    window.imp().event_stats_view.append_column(&event_count_column);

    // Create "Percentage" column for event stats
    let event_percent_factory = SignalListItemFactory::new();
    event_percent_factory.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    });
    event_percent_factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.percentage());
    });
    let event_percent_column = ColumnViewColumn::new(Some("%"), Some(event_percent_factory));
    window.imp().event_stats_view.append_column(&event_percent_column);

    // Setup columns for algorithms statistics view (stats_view)
    // "Algorithm" column
    let algo_factory = SignalListItemFactory::new();
    algo_factory.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    });
    algo_factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.algorithm());
    });
    let algo_column = ColumnViewColumn::new(Some("Algorithm"), Some(algo_factory));
    algo_column.set_expand(true);
    window.imp().stats_view.append_column(&algo_column);

    // Create "Count" column
    let count_factory_stats = SignalListItemFactory::new();
    count_factory_stats.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    });
    count_factory_stats.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.count());
    });
    let count_column_stats = ColumnViewColumn::new(Some("Count"), Some(count_factory_stats));
    window.imp().stats_view.append_column(&count_column_stats);

    // Create "Percentage" column
    let percent_factory = SignalListItemFactory::new();
    percent_factory.connect_setup(|_, list_item| {
        let label = Label::new(None);
        label.set_halign(gtk4::Align::End);
        label.set_margin_start(4);
        label.set_margin_end(4);
        list_item.set_child(Some(&label));
    });
    percent_factory.connect_bind(|_, list_item| {
        let list_item = list_item.downcast_ref::<ListItem>().unwrap();
        let stats_obj = list_item.item().and_downcast::<StatsObject>().unwrap();
        let label = list_item.child().and_downcast::<Label>().unwrap();
        label.set_text(&stats_obj.percentage());
    });
    let percent_column = ColumnViewColumn::new(Some("%"), Some(percent_factory));
    window.imp().stats_view.append_column(&percent_column);

    // Setup sunburst chart
    window.imp().sunburst_chart.set_zoom_banner(window.imp().zoom_banner.clone());
    window.imp().sunburst_chart.set_tree_store(root_store.clone());
    window.imp().sunburst_chart.set_column_view(window.imp().column_view.clone());
    window.imp().sunburst_chart.set_stats_store(stats_store.clone());
    window.imp().sunburst_chart.set_event_stats_store(event_stats_store.clone());
    window.imp().sunburst_chart.set_period_labels(
        window.imp().period_start_label.clone(),
        window.imp().period_end_label.clone(),
        window.imp().period_duration_label.clone(),
    );

    // Connect tree selection to chart highlighting
    let chart_clone = window.imp().sunburst_chart.clone();
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

    // Set initial page to empty state
    window.imp().main_stack.set_visible_child_name("empty");

    // Track current file path for reload functionality
    let current_file_path = Rc::new(RefCell::new(Option::<String>::None));

    // Set up "open" action
    let window_clone = window.clone();
    let chart_clone = window.imp().sunburst_chart.clone();
    let main_stack_clone = window.imp().main_stack.clone();
    let current_file_path_clone = current_file_path.clone();
    let reload_button_clone = window.imp().reload_button.clone();

    let open_action = gio::SimpleAction::new("open", None);
    open_action.connect_activate(move |_, _| {
        let dialog = gtk4::FileChooserDialog::new(
            Some("Open Audit File"),
            Some(&window_clone),
            gtk4::FileChooserAction::Open,
            &[
                ("Cancel", gtk4::ResponseType::Cancel),
                ("Open", gtk4::ResponseType::Accept),
            ],
        );

        // Add file filter
        let filter = gtk4::FileFilter::new();
        filter.set_name(Some("JSON Files"));
        filter.add_pattern("*.json");
        dialog.add_filter(&filter);

        let all_filter = gtk4::FileFilter::new();
        all_filter.set_name(Some("All Files"));
        all_filter.add_pattern("*");
        dialog.add_filter(&all_filter);

        let chart = chart_clone.clone();
        let main_stack = main_stack_clone.clone();
        let current_file_path = current_file_path_clone.clone();
        let reload_button = reload_button_clone.clone();

        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        if load_and_display(&path_str, &chart).is_ok() {
                            main_stack.set_visible_child_name("content");
                            *current_file_path.borrow_mut() = Some(path_str);
                            reload_button.set_sensitive(true);
                        }
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    });
    app.add_action(&open_action);

    // Set up "detailed-view" stateful action (toggle menu item)
    // Note: The action state is inverted - checked means detailed view (simple_view = false)
    let chart_clone = window.imp().sunburst_chart.clone();
    let initial_state = !window.imp().sunburst_chart.is_simple_view(); // Inverse: checked = detailed, unchecked = simple
    let detailed_view_action = gio::SimpleAction::new_stateful(
        "detailed-view",
        None,
        &initial_state.to_variant(),
    );

    detailed_view_action.connect_change_state({
        let chart = chart_clone.clone();
        move |action, state| {
            if let Some(state) = state {
                let show_detailed: bool = state.get().unwrap();
                action.set_state(state);

                // show_detailed is the inverse of simple_view
                // If show_detailed is true, we want simple_view to be false, and vice versa
                let should_be_simple = !show_detailed;
                if chart.is_simple_view() != should_be_simple {
                    chart.toggle_simple_view();
                }
            }
        }
    });

    app.add_action(&detailed_view_action);

    // Set up "about" action
    let window_clone = window.clone();
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate(move |_, _| {
        let about = AboutWindow::builder()
            .application_name("Crypto Usage Analyzer")
            .application_icon("org.gnome.CryptoUsageAnalyzer")
            .developer_name("Crypto Auditing Project")
            .version("0.1.0")
            .comments("Visualize cryptographic operations with interactive sunburst charts")
            .website("https://github.com/latchset/crypto-auditing")
            .issue_url("https://github.com/latchset/crypto-auditing/issues")
            .license_type(gtk4::License::Gpl30)
            .build();

        about.set_transient_for(Some(&window_clone));
        about.present();
    });
    app.add_action(&about_action);

    // Connect empty state button to open action
    let app_clone = app.clone();
    window.imp().empty_open_button.connect_clicked(move |_| {
        app_clone.activate_action("open", None);
    });

    // Set up reload button action
    let chart_reload = window.imp().sunburst_chart.clone();
    let current_file_path_reload = current_file_path.clone();
    window.imp().reload_button.connect_clicked(move |_| {
        if let Some(path) = current_file_path_reload.borrow().as_ref() {
            let _ = load_and_display(path, &chart_reload);
        }
    });

    // Try to load default file if it exists
    let default_path = "audit.json";
    if std::path::Path::new(default_path).exists()
        && load_and_display(default_path, &window.imp().sunburst_chart).is_ok() {
            window.imp().main_stack.set_visible_child_name("content");
            *current_file_path.borrow_mut() = Some(default_path.to_string());
            window.imp().reload_button.set_sensitive(true);
        }

    window.present();
}

fn load_and_display(path: &str, chart: &SunburstChart) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let events: Vec<AuditEvent> = serde_json::from_str(&content)?;

    let tree = TreeNode::from_events(&events);
    chart.set_data(tree, events);

    Ok(())
}
