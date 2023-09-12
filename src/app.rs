#![allow(unused_imports)]
#![allow(unused_variables)]
use relm4::adw::{
    prelude::*, HeaderBar, MessageDialog, StatusPage, Toast, ToastOverlay, ViewStack, Window,
};
use relm4::gtk::{
    pango::EllipsizeMode, prelude::*, Align, Box, Button, Entry, EntryBuffer, InputHints,
    InputPurpose, Label, Orientation, Overlay, PackType, ScrolledWindow, WindowControls,
};
use relm4::{factory::FactoryVecDeque, prelude::*};
use url::Url;
use webkit6::{glib, prelude::*, NavigationAction, Settings, WebView};
use webkit6_sys::webkit_web_view_get_settings;

use crate::config::{APP_ID, PROFILE};
use crate::webwindowcontrolbar::*;

pub(super) struct App {
    url_entry_buffer: EntryBuffer,
    webwindowcontrolbars: FactoryVecDeque<WebWindowControlBar>,
}

#[derive(Debug)]
pub enum AppInput {
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
                    set_orientation: Orientation::Vertical,
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
            AppInput::NewWebWindow => {
                let url_processed_result = process_url(String::from(self.url_entry_buffer.text()));
                let final_url_option = url_processed_result.ok();
                match final_url_option {
                    Some(final_url) => {
                        self.webwindowcontrolbars.guard().push_back(final_url);
                        self.url_entry_buffer = EntryBuffer::default();
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
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("webkit://") {
    } else if url.contains(" ") || !url.contains(".") {
        url = String::from(url.trim());
        url = url.replace(" ", "+");
        let mut search = String::from("https://duckduckgo.com/?q=");
        search.push_str(url.as_str());
        url = search;
    } else {
        url = String::from("https://") + url.as_str();
    }
    let result = Url::parse(url.as_str());
    match result {
        Ok(final_url) => Ok(String::from(url)),
        Err(error) => Err(()),
    }
}
