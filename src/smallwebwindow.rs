#![allow(unused_imports)]
#![allow(unused_variables)]
use pango::EllipsizeMode;
use relm4::adw::{
    prelude::*, HeaderBar, MessageDialog, StatusPage, Toast, ToastOverlay, ViewStack, Window,
};
use relm4::gtk::{
    prelude::*, Align, Box, Button, Entry, EntryBuffer, InputHints, InputPurpose, Label,
    Orientation, Overlay, PackType, ScrolledWindow, WindowControls,
};
use relm4::{factory::FactoryVecDeque, prelude::*};
use url::Url;
use webkit6::{glib, prelude::*, NavigationAction, Settings, WebView};
use webkit6_sys::webkit_web_view_get_settings;

use crate::config::{APP_ID, PROFILE};

pub struct SmallWebWindow {
    web_view: WebView,
}

#[derive(Debug)]
pub enum SmallWebWindowOutput {
    Close,
}

#[relm4::component(pub)]
impl SimpleComponent for SmallWebWindow {
    type Init = WebView;
    type Input = ();
    type Output = SmallWebWindowOutput;

    view! {
        Window {
            set_default_height: 400,
            set_default_width: 400,

            Box {
                set_orientation: Orientation::Vertical,

                HeaderBar {
                    set_decoration_layout: Some(":close"),
                    add_css_class: "raised",
                },

                model.web_view.clone(),
            },

            connect_close_request[sender] => move |_| {
                sender.output(SmallWebWindowOutput::Close);
                gtk::Inhibit(true)
            }
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SmallWebWindow { web_view: init };
        let widgets = view_output!();
        ComponentParts {
            model: model,
            widgets: widgets,
        }
    }
}
