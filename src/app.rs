#![allow(unused_imports)]
#![allow(unused_variables)]
use pango::EllipsizeMode;
use relm4::adw::{
    prelude::*, HeaderBar, MessageDialog, StatusPage, Toast, ToastOverlay, ViewStack, Window,
};
use relm4::gtk::{
    prelude::*, Align, Box, Button, Entry, EntryBuffer, InputHints, InputPurpose, Label,
    Orientation, ScrolledWindow, Video,
};
use relm4::{
    factory::FactoryVecDeque,
    prelude::{FactoryComponent, *},
};
use relm4_macros::*;
use url::Url;
use webkit6::{prelude::*, NavigationAction, Settings, WebView};
use webkit6_sys::webkit_web_view_get_settings;

use crate::config::{APP_ID, PROFILE};

// pub struct SmallWebWindow {
//     pub web_view_option: Option<WebView>,
// }

// pub enum SmallWebWindowOutput {
//     CloseSmallWebWindow
// }

// #[relm4::component(pub)]
// impl SimpleComponent for SmallWebWindow {
//     type Init = Option<WebView>;
//     type Input = ();
//     type Output = ();

//     view! {
//         #[name(small_web_window)]
//         Window {
//             set_default_height: 500,
//             set_default_width: 300,

//             Box {
//                 set_orientation: Orientation::Vertical,

//                 HeaderBar {
//                     set_decoration_layout: Some(":close"),
//                     add_css_class: "raised"
//                 },

//                 model.web_view_option.unwrap_or("Cannot find WebView").widgets(),
//             }
//         }
//     }

//     fn init(
//             init: Self::Init,
//             root: &Self::Root,
//             sender: ComponentSender<Self>,
//         ) -> ComponentParts<Self> {
//         let model = SmallWebWindow { web_view_option: init };
//         let widgets = view_output!();
//         ComponentParts { model: model, widgets: widgets }
//     }

//     fn shutdown(&mut self, widgets: &mut Self::Widgets, output: relm4::Sender<Self::Output>) {
//         widgets.small_web_window.destroy();
//     }
// }

pub struct WebWindow {
    pub url: String,
    // small_web_window_option: Option<SmallWebWindow>
}

#[derive(Debug)]
pub enum WebWindowInput {
    CloseSmallWebWindow
}

#[derive(Debug)]
pub enum WebWindowOutput {
    Close,
}

#[relm4::component(pub)]
impl SimpleComponent for WebWindow {
    type Init = String;
    type Input = WebWindowInput;
    type Output = WebWindowOutput;

    view! {
        #[name(web_window)]
        Window {
            set_default_height: 1000,
            set_default_width: 1000,

            #[name(toast_overlay)]
            ToastOverlay {
                Box {
                    set_orientation: Orientation::Vertical,

                    #[name(web_view)]
                    WebView {
                        set_vexpand: true,
                        load_uri: model.url.as_str(),
                        connect_create[sender] => |a, b| {
                            println!("{:?}", a);
                            println!("{:?}", b);
                            let c: WebView = WebView::new();
                            c.into()
                            // let model.small_web_window_option = Some(SmallWebWindow::builder()
                            //                                                         .transient_for(root)
                            //                                                         .launch(Some(a_web_view))
                            //                                                         .forward(sender.input_sender(), |message| match message {
                            //                                                             SmallWebWindow::CloseSmallWebWindow => WebWindow::CloseSmallWebWindow,
                            //                                                         }));
                            // model.small_web_window_option.unwrap_or("Could not create SmallWebWindow").into()
                        },
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
        let model = WebWindow { url: init };
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

    // fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
    //     match message {
    //         WebWindowInput::CloseSmallWebWindow => self.small_web_window_option = None,
    //     }
    // }

    // fn shutdown(&mut self, widgets: &mut Self::Widgets, output: relm4::Sender<Self::Output>) {
    //     widgets.web_window.destroy();
    // }
}

pub struct WebWindowControlBar {
    id: DynamicIndex,
    url: String,
    webwindow: Controller<WebWindow>,
}

pub type WebWindowControlBarInit = String;

#[derive(Debug)]
pub enum WebWindowControlBarInput {
    Back,
    Forward,
    Close,
    Refresh,
    Focus,
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
                connect_clicked => WebWindowControlBarInput::Back,
            },

            #[name(forward_btn)]
            Button {
                add_css_class: "circular",
                add_css_class: "flat",
                set_icon_name: "right",
                set_tooltip_text: Some("Forward"),
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

            Label {
                set_hexpand: true,
                set_halign: Align::Start,
                set_margin_start: 5,
                set_margin_end: 5,
                set_ellipsize: EllipsizeMode::End,
                set_label: &self.url,
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
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let new_webwindow =
            WebWindow::builder()
                .launch(init.clone())
                .forward(sender.input_sender(), |message| match message {
                    WebWindowOutput::Close => WebWindowControlBarInput::Close,
                });
        Self {
            id: index.clone(),
            url: init,
            webwindow: new_webwindow,
        }
    }

    fn forward_to_parent(_output: Self::Output) -> Option<Self::ParentInput> {
        Some(match _output {
            WebWindowControlBarOutput::Remove(id) => AppInput::RemoveWebWindowControlBar(id),
        })
    }
}

pub(super) struct App {
    url_entry_buffer: EntryBuffer,
    webwindowcontrolbars: FactoryVecDeque<WebWindowControlBar>,
    entry_is_valid: Option<bool>,
}

#[derive(Debug)]
pub enum AppInput {
    EntryChanged,
    NewWebWindow, // Also handles adding a WebWindowControlBar
    RemoveWebWindowControlBar(DynamicIndex),
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppInput;
    type Output = ();

    view! {
        Window {
            set_default_height: 500,
            set_default_width: 400,
            set_title: Some("Spidey"),
            add_css_class?: if PROFILE == "Devel" {
                Some("devel")
            } else {
                None
            },

            Box {
                set_orientation: Orientation::Vertical,

                HeaderBar {
                    set_decoration_layout: Some(":close"),
                    add_css_class: "flat",
                },

                Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 3,
                    set_margin_all: 5,

                    Box {
                        set_orientation: Orientation::Horizontal,
                        set_hexpand: true,
                        set_margin_all: 5,
                        set_halign: Align::Fill,

                        #[name(url_entry)]
                        Entry {
                            set_hexpand: true,
                            set_halign: Align::Fill,
                            set_margin_all: 5,
                            #[watch]
                            set_buffer: &model.url_entry_buffer,
                            #[watch]
                            set_css_classes: match model.entry_is_valid {
                                Some(is_valid) => if is_valid {&["success"]} else {&["error"]},
                                None => &[""],
                            },
                            set_placeholder_text: Some("Search the web or enter a link"),
                            set_input_purpose: InputPurpose::Url,
                            set_input_hints: InputHints::NO_SPELLCHECK,
                            // connect_changed => AppInput::EntryChanged,
                        },

                        #[name(add_btn)]
                        Button {
                            set_margin_all: 5,
                            set_halign: Align::End,
                            set_icon_name: "plus",
                            set_tooltip_text: Some("New Window"),
                            connect_clicked => AppInput::NewWebWindow,
                        }
                    },

                    ScrolledWindow {
                        set_vexpand: true,

                        Box {
                            set_orientation: Orientation::Horizontal,
                            set_hexpand: true,
                            set_halign: Align::Fill,

                            Box {
                                set_orientation: Orientation::Vertical,

                                #[local_ref]
                                webwindowcontrolbar_box -> Box {
                                    set_orientation: Orientation::Vertical,
                                    set_spacing: 0,
                                }
                            }

                        }
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let webwindowcontrolbars = FactoryVecDeque::new(gtk::Box::default(), sender.input_sender());
        let model = App {
            webwindowcontrolbars: webwindowcontrolbars,
            url_entry_buffer: EntryBuffer::default(),
            entry_is_valid: None,
        };
        let webwindowcontrolbar_box = model.webwindowcontrolbars.widget();
        let widgets = view_output!();
        ComponentParts {
            model: model,
            widgets: widgets,
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppInput::EntryChanged => {
                if !self.url_entry_buffer.text().is_empty() {
                    let url_processed_result =
                        process_url(String::from(self.url_entry_buffer.text()));
                    match url_processed_result {
                        Ok(_) => self.entry_is_valid = Some(true),
                        Err(_) => self.entry_is_valid = Some(false),
                    }
                } else {
                    self.entry_is_valid = None;
                }
            }
            AppInput::NewWebWindow => {
                let url_processed_result = process_url(String::from(self.url_entry_buffer.text()));
                let final_url_option = url_processed_result.ok();
                match final_url_option {
                    Some(final_url) => {
                        self.webwindowcontrolbars.guard().push_back(final_url);
                        self.url_entry_buffer = EntryBuffer::default();
                        self.entry_is_valid = None;
                    }
                    None => {}
                }
            }

            AppInput::RemoveWebWindowControlBar(id) => {
                self.webwindowcontrolbars.guard().remove(id.current_index());
            }
        }
    }
}

fn process_url(mut url: String) -> Result<String, ()> {
    if url.contains(" ") || !url.contains(".") {
        url = String::from(url.trim());
        url = url.replace(" ", "+");
        let mut search = String::from("https://duckduckgo.com/?q=");
        search.push_str(url.as_str());
        url = search;
    } else if !(url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("webkit://"))
    {
        url = String::from("https://") + url.as_str();
    }
    let result = Url::parse(url.as_str());
    match result {
        Ok(final_url) => Ok(final_url.into_string()),
        Err(error) => Err(()),
    }
}
