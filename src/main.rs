mod data;
mod models;
mod sunburst;

use adw::prelude::*;
use adw::{
    glib, AboutWindow, Application, ApplicationWindow, Banner, HeaderBar, NavigationPage,
    NavigationSplitView, StatusPage, ToolbarView, ViewStack, ViewSwitcherBar,
};
use anyhow::Result;
use data::{AuditEvent, TreeNode};
use gtk4::{
    gio, Button, ColumnView, ColumnViewColumn, Label, ListItem, Orientation, ScrolledWindow,
    SignalListItemFactory, SingleSelection, Stack, TreeListModel, TreeListRow,
};
use models::{StatsObject, TreeNodeObject};
use std::fs;
use std::rc::Rc;
use sunburst::SunburstChart;

const APP_ID: &str = "org.gnome.CryptoUsageAnalyzer";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    // Create header bar
    let header_bar = HeaderBar::new();

    // Create hamburger menu
    let menu = gio::Menu::new();
    menu.append(Some("Open File"), Some("app.open"));
    menu.append(Some("About Crypto Usage Analyzer"), Some("app.about"));

    let menu_button = gtk4::MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_menu_model(Some(&menu));
    header_bar.pack_end(&menu_button);

    // Create stack for switching between empty state and split view
    let stack = Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);

    // Create empty state with status page
    let status_page = StatusPage::builder()
        .icon_name("document-open-symbolic")
        .title("No Data Loaded")
        .description("Open an audit.json file to visualize crypto usage")
        .build();

    let empty_button = Button::with_label("Open File");
    empty_button.add_css_class("pill");
    empty_button.add_css_class("suggested-action");
    status_page.set_child(Some(&empty_button));

    stack.add_named(&status_page, Some("empty"));

    // Create tree list model for sidebar
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
    let column_view = ColumnView::new(Some(selection_model.clone()));
    column_view.add_css_class("data-table");

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
    column_view.append_column(&name_column);

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
    column_view.append_column(&count_column);

    // Wrap column view in scrolled window
    let tree_scroll = ScrolledWindow::new();
    tree_scroll.set_child(Some(&column_view));
    tree_scroll.set_min_content_width(300);

    // Create sampling period section
    let sampling_period_box = gtk4::Box::new(Orientation::Vertical, 6);
    sampling_period_box.set_margin_start(12);
    sampling_period_box.set_margin_end(12);
    sampling_period_box.set_margin_top(12);
    sampling_period_box.set_margin_bottom(12);

    let period_title = gtk4::Label::new(Some("Sampling Period"));
    period_title.set_halign(gtk4::Align::Start);
    period_title.add_css_class("title-4");
    sampling_period_box.append(&period_title);

    let period_start_label = gtk4::Label::new(Some("Start: Not loaded"));
    period_start_label.set_halign(gtk4::Align::Start);
    period_start_label.add_css_class("dim-label");
    sampling_period_box.append(&period_start_label);

    let period_end_label = gtk4::Label::new(Some("End: Not loaded"));
    period_end_label.set_halign(gtk4::Align::Start);
    period_end_label.add_css_class("dim-label");
    sampling_period_box.append(&period_end_label);

    let period_duration_label = gtk4::Label::new(Some("Duration: Not loaded"));
    period_duration_label.set_halign(gtk4::Align::Start);
    period_duration_label.add_css_class("dim-label");
    sampling_period_box.append(&period_duration_label);

    let separator = gtk4::Separator::new(Orientation::Horizontal);
    separator.set_margin_top(6);
    sampling_period_box.append(&separator);

    // Create algorithms section
    let algorithms_box = gtk4::Box::new(Orientation::Vertical, 6);
    algorithms_box.set_margin_start(12);
    algorithms_box.set_margin_end(12);
    algorithms_box.set_margin_top(12);
    algorithms_box.set_margin_bottom(12);

    let algorithms_title = gtk4::Label::new(Some("Most Used Algorithms"));
    algorithms_title.set_halign(gtk4::Align::Start);
    algorithms_title.add_css_class("title-4");
    algorithms_box.append(&algorithms_title);

    // Create statistics view
    let stats_store = gio::ListStore::new::<StatsObject>();
    let stats_selection = SingleSelection::new(Some(stats_store.clone()));
    let stats_view = ColumnView::new(Some(stats_selection));
    stats_view.add_css_class("data-table");

    // Create "Algorithm" column
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
    stats_view.append_column(&algo_column);

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
    stats_view.append_column(&count_column_stats);

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
    let percent_column = ColumnViewColumn::new(Some("Percentage"), Some(percent_factory));
    stats_view.append_column(&percent_column);

    // Wrap statistics view in scrolled window
    let stats_scroll = ScrolledWindow::new();
    stats_scroll.set_child(Some(&stats_view));
    stats_scroll.set_min_content_width(300);
    stats_scroll.set_min_content_height(200);
    stats_scroll.set_max_content_height(400);

    // Add the scrolled window to algorithms box
    algorithms_box.append(&stats_scroll);

    // Create stats container with period section and algorithms section
    let stats_container = gtk4::Box::new(Orientation::Vertical, 0);
    stats_container.append(&sampling_period_box);
    stats_container.append(&algorithms_box);

    // Create sidebar page with statistics only
    let sidebar_page = NavigationPage::builder()
        .title("Statistics")
        .child(&stats_container)
        .build();

    // Create banner for zoom notification
    let banner = Banner::new("");
    banner.set_title("Click to reset the zoom");
    banner.set_button_label(Some("Reset"));
    banner.set_revealed(false);

    // Create sunburst chart
    let chart = Rc::new(SunburstChart::new());
    chart.set_zoom_banner(banner.clone());
    chart.set_tree_store(root_store.clone());
    chart.set_column_view(column_view.clone());
    chart.set_stats_store(stats_store.clone());
    chart.set_period_labels(
        period_start_label.clone(),
        period_end_label.clone(),
        period_duration_label.clone(),
    );

    // Create sunburst view container (banner + chart)
    let sunburst_box = gtk4::Box::new(Orientation::Vertical, 0);
    sunburst_box.append(&banner);
    sunburst_box.append(chart.widget());

    // Connect tree selection to chart highlighting
    let chart_clone = chart.clone();
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

    // Create view stack for content area
    let content_view_stack = ViewStack::new();
    content_view_stack.set_vexpand(true);

    let sunburst_page = content_view_stack.add_titled(&sunburst_box, Some("sunburst"), "Sunburst");
    sunburst_page.set_icon_name(Some("view-paged-symbolic"));

    let tree_page = content_view_stack.add_titled(&tree_scroll, Some("tree"), "Event Tree");
    tree_page.set_icon_name(Some("view-list-symbolic"));

    // Create view switcher bar for content
    let content_view_switcher = ViewSwitcherBar::new();
    content_view_switcher.set_stack(Some(&content_view_stack));
    content_view_switcher.set_reveal(true);

    // Create content container with view stack and switcher
    let content_container = gtk4::Box::new(Orientation::Vertical, 0);
    content_container.append(&content_view_stack);
    content_container.append(&content_view_switcher);

    // Create content page
    let content_page = NavigationPage::builder()
        .title("Crypto Usage")
        .child(&content_container)
        .build();

    // Create navigation split view
    let split_view = NavigationSplitView::new();
    split_view.set_sidebar(Some(&sidebar_page));
    split_view.set_content(Some(&content_page));
    split_view.set_show_content(true);
    split_view.set_min_sidebar_width(250.0);
    split_view.set_max_sidebar_width(500.0);

    stack.add_named(&split_view, Some("content"));

    // Set initial page
    stack.set_visible_child_name("empty");

    // Create toolbar view (modern Adwaita pattern)
    let toolbar_view = ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&stack));

    // Create window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Crypto Usage Analyzer")
        .default_width(1100)
        .default_height(800)
        .content(&toolbar_view)
        .build();

    // Set up "open" action
    let window_clone = window.clone();
    let chart_clone = chart.clone();
    let stack_clone = stack.clone();

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
        let stack = stack_clone.clone();

        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        if load_and_display(&path.to_string_lossy(), &chart).is_ok() {
                            stack.set_visible_child_name("content");
                        }
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    });
    app.add_action(&open_action);

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
            .website("https://github.com/simo5/crypto-auditing")
            .issue_url("https://github.com/simo5/crypto-auditing/issues")
            .license_type(gtk4::License::Gpl30)
            .build();

        about.set_transient_for(Some(&window_clone));
        about.present();
    });
    app.add_action(&about_action);

    // Connect empty state button to open action
    let app_clone = app.clone();
    empty_button.connect_clicked(move |_| {
        app_clone.activate_action("open", None);
    });

    // Try to load default file if it exists
    let default_path = "audit.json";
    if std::path::Path::new(default_path).exists()
        && load_and_display(default_path, &chart).is_ok() {
            stack.set_visible_child_name("content");
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
