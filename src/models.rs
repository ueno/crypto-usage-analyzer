use glib::Object;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{Ordering, Sorter};
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
        pub(super) count: RefCell<u64>,
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
    pub fn new(name: &str, count: u64) -> Self {
        Object::builder()
            .property("name", name)
            .property("count", count)
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
        pub(super) name: RefCell<String>,
        #[property(get, set)]
        pub(super) count: RefCell<u64>,
        #[property(get, set)]
        pub(super) total: RefCell<u64>,
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
    pub fn new(name: &str, count: u64, total: u64) -> Self {
        Object::builder()
            .property("name", name)
            .property("count", count)
            .property("total", total)
            .build()
    }

    pub fn percentage(&self) -> f64 {
        if self.total() > 0 {
            self.count() as f64 / self.total() as f64 * 100.0
        } else {
            0f64
        }
    }
}

// StatsSorter - GObject wrapper for sorting statistics data
mod imp_stats_sorter {
    use super::*;

    #[derive(Default)]
    pub struct StatsSorter {}

    #[glib::object_subclass]
    impl ObjectSubclass for StatsSorter {
        const NAME: &'static str = "StatsSorter";
        type Type = super::StatsSorter;
        type ParentType = Sorter;
    }

    impl ObjectImpl for StatsSorter {}

    impl SorterImpl for StatsSorter {
        fn compare(&self, item1: &Object, item2: &Object) -> Ordering {
            let item1 = item1.downcast_ref::<StatsObject>().unwrap();
            let item2 = item2.downcast_ref::<StatsObject>().unwrap();
            let ordering: Ordering = item2.count().cmp(&item1.count()).into();
            if ordering == Ordering::Equal {
                return item2.name().cmp(&item1.name()).into();
            }
            ordering
        }
    }
}

glib::wrapper! {
    pub struct StatsSorter(ObjectSubclass<imp_stats_sorter::StatsSorter>);
}

impl StatsSorter {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
