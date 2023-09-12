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
use crate::smallwebwindow::*;

pub struct WebWindow {
    pub url: String,
    small_web_window_option: Option<Controller<SmallWebWindow>>,
}

#[derive(Debug)]
pub enum WebWindowInput {
    CreateSmallWebWindow(WebView),
    CloseSmallWebWindow,
}

#[derive(Debug)]
pub enum WebWindowOutput {
    LoadChanged((bool, bool)),
    TitleChanged(String),
    Close,
}

#[relm4::component(pub)]
impl Component for WebWindow {
    type Init = String;
    type Input = WebWindowInput;
    type Output = WebWindowOutput;
    type CommandOutput = ();

    view! {
        #[name(web_window)]
        Window {
            set_default_height: 1000,
            set_default_width: 1000,

            Overlay {
                add_overlay = &WindowControls {
                    set_halign: Align::End,
                    set_valign: Align::Start,
                    set_margin_top: 5,
                    set_margin_end: 5,
                    set_side: PackType::End,
                    add_css_class: "webwindow-close",
                },
                // add_overlay = &HeaderBar {
                //     set_halign: Align::Fill,
                //     set_valign: Align::Start,
                //     set_decoration_layout: Some(":close"),
                //     add_css_class: "webwindow-headerbar",
                // },
                #[name(toast_overlay)]
                ToastOverlay {
                    Box {
                        set_orientation: Orientation::Vertical,

                        #[name(web_view)]
                        WebView {
                            set_vexpand: true,
                            load_uri: model.url.as_str(),
                            connect_load_changed[sender] => move |this_webview, _load_event| {
                                sender.output(WebWindowOutput::LoadChanged((this_webview.can_go_back(), this_webview.can_go_forward())));
                            },
                            connect_title_notify[sender] => move |this_webview| {
                                let title: String = match this_webview.title() {
                                    Some(text) => text.into(),
                                    None => "".into()
                                };
                                sender.output(WebWindowOutput::TitleChanged(title));
                            },
                            connect_create[sender] => move |this_webview, _navigation_action| {
                                let new_webview = glib::Object::builder::<WebView>().property("related-view", this_webview).build();
                                new_webview.set_vexpand(true);
                                // println!("{}", new_webview.uri().unwrap());
                                // let new_webview = WebView::new();
                                // new_webview.set_property("related-view", this_webview);
                                let sender_clone = sender.clone();
                                let new_webview_clone = new_webview.clone();
                                new_webview.connect_ready_to_show(move |_| {
                                    sender_clone.input(WebWindowInput::CreateSmallWebWindow(new_webview_clone.clone()));
                                });
                                new_webview.into()

                            },
                        }
                    }
                }
            },

            connect_close_request[sender] => move |_| {
                sender.output(WebWindowOutput::Close);
                gtk::Inhibit(true)
            } ,

            present: (),
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = WebWindow {
            url: init,
            small_web_window_option: None,
        };
        let widgets = view_output!();
        let web_view_settings: Settings = Settings::new();
        web_view_settings.set_media_playback_requires_user_gesture(true);
        if PROFILE == "Devel" {
            web_view_settings.set_enable_developer_extras(true);
            widgets.web_view.set_settings(&web_view_settings);
        } else {
            widgets.web_view.set_settings(&web_view_settings);
        }
        ComponentParts {
            model: model,
            widgets: widgets,
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            WebWindowInput::CreateSmallWebWindow(new_webview) => {
                self.small_web_window_option = Some(
                    SmallWebWindow::builder()
                        // .transient_for(root)
                        .launch(new_webview)
                        .forward(sender.input_sender(), |message| match message {
                            SmallWebWindowOutput::Close => WebWindowInput::CloseSmallWebWindow,
                        }),
                );
            }
            WebWindowInput::CloseSmallWebWindow => self.small_web_window_option = None,
        }
    }
}
