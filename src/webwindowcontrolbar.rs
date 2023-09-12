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

use crate::app::AppInput;
use crate::config::{APP_ID, PROFILE};
use crate::webwindow::*;

pub struct WebWindowControlBar {
    id: DynamicIndex,
    url: String,
    label: String,
    webwindow: Controller<WebWindow>,
    web_view_can_go_back: bool,
    web_view_can_go_forward: bool,
}

pub type WebWindowControlBarInit = String;

#[derive(Debug)]
pub enum WebWindowControlBarInput {
    Back,
    Forward,
    Close,
    Refresh,
    Focus,
    WebViewLoadChanged((bool, bool)),
    WebViewTitleChanged(String),
}

#[derive(Debug)]
pub enum WebWindowControlBarOutput {
    Remove(DynamicIndex), // pass the id
}

#[relm4::factory(pub)]
impl FactoryComponent for WebWindowControlBar {
    type Init = WebWindowControlBarInit;
    type Input = WebWindowControlBarInput;
    type Output = WebWindowControlBarOutput;
    type CommandOutput = ();
    type Widgets = WebWindowControlBarWidgets;
    type ParentInput = AppInput;
    type ParentWidget = Box;

    view! {
        Box {
            set_orientation: Orientation::Horizontal,
            set_spacing: 0,
            set_margin_all: 5,

            #[name(back_btn)]
            Button {
                add_css_class: "circular",
                add_css_class: "flat",
                set_icon_name: "left",
                set_tooltip_text: Some("Back"),
                #[watch]
                set_sensitive: self.web_view_can_go_back,
                connect_clicked => WebWindowControlBarInput::Back,
            },

            #[name(forward_btn)]
            Button {
                add_css_class: "circular",
                add_css_class: "flat",
                set_icon_name: "right",
                set_tooltip_text: Some("Forward"),
                #[watch]
                set_sensitive: self.web_view_can_go_forward,
                connect_clicked => WebWindowControlBarInput::Forward,
            },

            #[name(refresh_btn)]
            Button {
                add_css_class: "circular",
                add_css_class: "flat",
                set_icon_name: "refresh",
                set_tooltip_text: Some("Refresh"),
                connect_clicked => WebWindowControlBarInput::Refresh,
            },

            #[name(label)]
            Label {
                set_hexpand: true,
                set_halign: Align::Start,
                set_margin_start: 5,
                set_margin_end: 5,
                set_ellipsize: EllipsizeMode::End,
                #[watch]
                set_label: &self.label,
            },

            #[name(focus_btn)]
            Button {
                add_css_class: "circular",
                add_css_class: "flat",
                add_css_class: "toolbar-button",
                set_icon_name: "multitasking-windows",
                set_tooltip_text: Some("Focus"),
                connect_clicked => WebWindowControlBarInput::Focus,
            },

            #[name(close_btn)]
            Button {
                add_css_class: "circular",
                add_css_class: "flat",
                add_css_class: "toolbar-button",
                set_icon_name: "cross",
                set_tooltip_text: Some("Close"),
                connect_clicked => WebWindowControlBarInput::Close,
            }
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            WebWindowControlBarInput::Close => {
                self.webwindow.widgets().web_window.destroy();
                sender.output(WebWindowControlBarOutput::Remove(self.id.clone()));
            }
            WebWindowControlBarInput::Back => self.webwindow.widgets().web_view.go_back(),
            WebWindowControlBarInput::Forward => self.webwindow.widgets().web_view.go_forward(),
            WebWindowControlBarInput::Refresh => self.webwindow.widgets().web_view.reload(),
            WebWindowControlBarInput::Focus => self.webwindow.widgets().web_window.present(),
            WebWindowControlBarInput::WebViewLoadChanged((can_go_back, can_go_forward)) => {
                self.web_view_can_go_back = can_go_back;
                self.web_view_can_go_forward = can_go_forward;
            }
            WebWindowControlBarInput::WebViewTitleChanged(title) => self.label = title,
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let new_webwindow =
            WebWindow::builder()
                .launch(init.clone())
                .forward(sender.input_sender(), |message| match message {
                    WebWindowOutput::LoadChanged((can_go_back, can_go_forward)) => {
                        WebWindowControlBarInput::WebViewLoadChanged((can_go_back, can_go_forward))
                    }
                    WebWindowOutput::TitleChanged(title) => {
                        WebWindowControlBarInput::WebViewTitleChanged(title)
                    }
                    WebWindowOutput::Close => WebWindowControlBarInput::Close,
                });
        Self {
            id: index.clone(),
            url: init.clone(),
            label: init,
            webwindow: new_webwindow,
            web_view_can_go_back: false,
            web_view_can_go_forward: false,
        }
    }

    fn forward_to_parent(_output: Self::Output) -> Option<Self::ParentInput> {
        Some(match _output {
            WebWindowControlBarOutput::Remove(id) => AppInput::RemoveWebWindowControlBar(id),
        })
    }
}
