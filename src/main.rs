mod data;
mod models;
mod sunburst_chart;
mod window;

use adw::prelude::*;
use adw::{glib, AboutWindow, Application};
use gtk4::{gio, subclass::prelude::*};
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

    // Set up "open" action
    let window_clone = window.clone();

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

        let window_clone = window_clone.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        window_clone.set_path(path);
                        if let Err(e) = window_clone.reload() {
                            window_clone.show_toast(&e.to_string());
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
    let detailed_view_action =
        gio::SimpleAction::new_stateful("detailed-view", None, &initial_state.to_variant());

    detailed_view_action.connect_change_state(move |action, state| {
        if let Some(state) = state {
            let show_detailed: bool = state.get().unwrap();
            action.set_state(state);

            // show_detailed is the inverse of simple_view
            // If show_detailed is true, we want simple_view to be false, and vice versa
            let should_be_simple = !show_detailed;
            if chart_clone.is_simple_view() != should_be_simple {
                chart_clone.toggle_simple_view();
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

    let reload_action = gio::SimpleAction::new("reload", None);
    let window_clone = window.clone();
    reload_action.connect_activate(move |_, _| {
        if let Err(e) = window_clone.reload() {
            window_clone.show_toast(&e.to_string());
        }
    });
    app.add_action(&reload_action);

    window.present();
}
