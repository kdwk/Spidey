#![allow(unused_imports)]
#![allow(unused_variables)]
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::{
    prelude::*, AboutWindow, HeaderBar, MessageDialog, StatusPage, Toast, ToastOverlay,
    ToolbarView, ViewStack, Window,
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

relm4::new_action_group!(AppWindowActionGroup, "win");
relm4::new_stateless_action!(ShowAbout, AppWindowActionGroup, "show_about");
#[derive(Debug)]
pub enum AppInput {
    NewWebWindow, // Also handles adding a WebWindowControlBar
    RemoveWebWindowControlBar(DynamicIndex),
    ShowAboutWindow,
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ();

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

                    pack_start = &Button {
                        set_icon_name: "about",
                        connect_clicked => AppInput::ShowAboutWindow,
                    }
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
                        add_css_class: "raised",
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
        // let app = relm4::main_adw_application();
        // let mut action_group = RelmActionGroup::<AppWindowActionGroup>::new();
        // let show_about: RelmAction<ShowAbout> = RelmAction::new_stateless(move |_| {
        //     AboutWindow::builder()
        //     .application_icon("application-x-executable")
        //     .developer_name("Kdwk")
        //     .version("1.0")
        //     .comments("World Wide Web-crawler")
        //     .website("https://github.com/kdwk/Spidey")
        //     .issue_url("https://github.com/kdwk/Spidey/issues")
        //     .copyright("© 2023 Kendrew Leung")
        //     .build()
        //     .present();
        // });
        // app.set_accels_for_action("show_about", &["<Primary>A"]);
        // action_group.add_action(show_about);
        // action_group.register_for_widget(root);
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
            AppInput::NewWebWindow => {
                let url_processed_result = process_url(String::from(self.url_entry_buffer.text()));
                let final_url_option = url_processed_result.ok();
                match final_url_option {
                    Some(final_url) => {
                        self.webwindowcontrolbars.guard().push_back(final_url);
                        self.url_entry_buffer.set_text("");
                    }
                    None => {}
                }
            }

            AppInput::RemoveWebWindowControlBar(id) => {
                self.webwindowcontrolbars.guard().remove(id.current_index());
            }

            AppInput::ShowAboutWindow => {
                AboutWindow::builder()
                    .transient_for(root)
                    .application_icon("application-x-executable")
                    .developer_name("Kdwk")
                    .version("1.0")
                    .comments("World Wide Web-crawler")
                    .website("https://github.com/kdwk/Spidey")
                    .issue_url("https://github.com/kdwk/Spidey/issues")
                    .copyright("© 2023 Kendrew Leung")
                    .build()
                    .present();
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
