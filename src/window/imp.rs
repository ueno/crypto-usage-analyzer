use adw::subclass::prelude::*;
use gtk4::{glib, prelude::*, CompositeTemplate};
use crate::sunburst_chart::SunburstChart;

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/gnome/CryptoUsageAnalyzer/ui/window.ui")]
pub struct Window {
    #[template_child]
    pub main_stack: TemplateChild<gtk4::Stack>,
    #[template_child]
    pub empty_open_button: TemplateChild<gtk4::Button>,
    #[template_child]
    pub menu_button: TemplateChild<gtk4::MenuButton>,
    #[template_child]
    pub reload_button: TemplateChild<gtk4::Button>,
    #[template_child]
    pub column_view: TemplateChild<gtk4::ColumnView>,
    #[template_child]
    pub stats_view: TemplateChild<gtk4::ColumnView>,
    #[template_child]
    pub event_stats_view: TemplateChild<gtk4::ColumnView>,
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
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "CryptoUsageAnalyzerWindow";
    type Type = super::Window;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        // Register custom widget types before loading UI
        SunburstChart::ensure_type();

        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for Window {}

impl WidgetImpl for Window {}

impl WindowImpl for Window {}

impl ApplicationWindowImpl for Window {}

impl AdwApplicationWindowImpl for Window {}
