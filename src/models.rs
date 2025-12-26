use glib::Object;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use std::cell::RefCell;

// TreeNodeObject - GObject wrapper for tree node data
mod imp_tree_node {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::TreeNodeObject)]
    pub struct TreeNodeObject {
        #[property(get, set)]
        pub(super) name: RefCell<String>,
        #[property(get, set)]
        pub(super) count: RefCell<String>,
        #[property(get, set)]
        pub(super) value: RefCell<u32>,
        pub(super) children: RefCell<Option<gtk4::gio::ListStore>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TreeNodeObject {
        const NAME: &'static str = "TreeNodeObject";
        type Type = super::TreeNodeObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for TreeNodeObject {}
}

glib::wrapper! {
    pub struct TreeNodeObject(ObjectSubclass<imp_tree_node::TreeNodeObject>);
}

impl TreeNodeObject {
    pub fn new(name: &str, count: &str, value: u32) -> Self {
        Object::builder()
            .property("name", name)
            .property("count", count)
            .property("value", value)
            .build()
    }

    pub fn children(&self) -> Option<gtk4::gio::ListStore> {
        self.imp().children.borrow().clone()
    }

    pub fn set_children(&self, children: Option<gtk4::gio::ListStore>) {
        self.imp().children.replace(children);
    }
}

// StatsObject - GObject wrapper for statistics data
mod imp_stats {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::StatsObject)]
    pub struct StatsObject {
        #[property(get, set)]
        pub(super) algorithm: RefCell<String>,
        #[property(get, set)]
        pub(super) count: RefCell<String>,
        #[property(get, set)]
        pub(super) percentage: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatsObject {
        const NAME: &'static str = "StatsObject";
        type Type = super::StatsObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for StatsObject {}
}

glib::wrapper! {
    pub struct StatsObject(ObjectSubclass<imp_stats::StatsObject>);
}

impl StatsObject {
    pub fn new(algorithm: &str, count: &str, percentage: &str) -> Self {
        Object::builder()
            .property("algorithm", algorithm)
            .property("count", count)
            .property("percentage", percentage)
            .build()
    }
}
