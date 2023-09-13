#![allow(unused_imports)]
#![allow(unused_variables)]
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

#[relm4::component(pub)]
impl SimpleComponent for SmallWebWindow {
    type Init = WebView;
    type Input = ();
    type Output = ();

    view! {
        Window {
            set_default_height: 550,
            set_default_width: 450,
            set_modal: true,

            Box {
                set_orientation: Orientation::Vertical,

                HeaderBar {
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
        let model = SmallWebWindow { web_view: init };
        let widgets = view_output!();
        ComponentParts {
            model: model,
            widgets: widgets,
        }
    }
}
