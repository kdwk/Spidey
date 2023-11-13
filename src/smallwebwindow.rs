#![allow(unused_imports)]
#![allow(unused_variables)]
use relm4::adw::prelude::*;
use relm4::gtk::{glib::clone, prelude::*};
use relm4::prelude::*;
use webkit6::prelude::*;

use crate::config::{APP_ID, PROFILE};

pub struct SmallWebWindow {
    pub web_view: webkit6::WebView,
    width_height: (i32, i32),
}

#[relm4::component(pub)]
impl SimpleComponent for SmallWebWindow {
    type Init = (webkit6::WebView, (i32, i32));
    type Input = ();
    type Output = ();

    view! {
        #[name(small_web_window)]
        adw::Window {
            set_default_width: model.width_height.0,
            set_default_height: model.width_height.1,
            set_modal: true,
            set_title: Some(""),

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                adw::HeaderBar {
                    set_decoration_layout: Some(":close"),
                    add_css_class: "raised",
                },

                model.web_view.clone(),
            },

            present: ()
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SmallWebWindow {
            web_view: init.0,
            width_height: init.1,
        };
        let widgets = view_output!();
        ComponentParts {
            model: model,
            widgets: widgets,
        }
    }
}
