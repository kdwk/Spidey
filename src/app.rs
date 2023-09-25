#![allow(unused_imports)]
#![allow(unused_variables)]
use relm4::actions::AccelsPlus;
use relm4::adw::{
    prelude::*, HeaderBar, MessageDialog, StatusPage, Toast, ToastOverlay, ToolbarView, ViewStack,
    Window,
};
use relm4::gtk::{
    prelude::*, Align, Box, Button, Entry, EntryBuffer, EntryIconPosition, InputHints,
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
            set_default_height: 510,
            set_default_width: 370,
            set_title: Some("Spidey"),
            add_css_class?: if PROFILE == "Devel" {
                Some("devel")
            } else {
                None
            },

            ToolbarView {
                add_top_bar = &HeaderBar {
                    set_decoration_layout: Some(":close"),
                    add_css_class: "flat",
                },

                add_top_bar = &Box {
                    add_css_class: "toolbar",
                    set_margin_top: 5,
                    set_orientation: Orientation::Horizontal,
                    set_hexpand: true,
                    set_halign: Align::Fill,

                    #[name(url_entry)]
                    Entry {
                        set_hexpand: true,
                        set_halign: Align::Fill,
                        set_margin_start: 5,
                        set_margin_end: 0,
                        #[watch]
                        set_buffer: &model.url_entry_buffer,
                        set_placeholder_text: Some("Search the web or enter a link"),
                        set_input_purpose: InputPurpose::Url,
                        set_input_hints: InputHints::NO_SPELLCHECK,
                        connect_activate => AppInput::NewWebWindow,
                    },

                    #[name(add_btn)]
                    Button {
                        set_margin_start: 5,
                        set_margin_end: 5,
                        set_halign: Align::End,
                        set_icon_name: "plus",
                        set_tooltip_text: Some("New Web Window"),
                        connect_clicked => AppInput::NewWebWindow,
                    }
                },

                #[wrap(Some)]
                set_content = &ScrolledWindow {
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
        let webwindowcontrolbars = FactoryVecDeque::builder(Box::default()).launch().forward(
            sender.input_sender(),
            |output| match output {
                WebWindowControlBarOutput::Remove(index) => {
                    AppInput::RemoveWebWindowControlBar(index)
                }
            },
        );
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
