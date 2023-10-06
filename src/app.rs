#![allow(unused_imports)]
#![allow(unused_variables)]
use curl::easy::Easy;
use directories;
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::io::Write;
use std::{
    fs::{create_dir_all, File, OpenOptions},
    thread,
};
use url::Url;

use crate::config::{APP_ID, PROFILE};
use crate::webwindowcontrolbar::*;

pub(super) struct App {
    url_entry_buffer: gtk::EntryBuffer,
    webwindowcontrolbars: relm4::factory::FactoryVecDeque<WebWindowControlBar>,
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
        adw::Window {
            set_default_height: 510,
            set_default_width: 370,
            set_title: Some("Spidey"),
            add_css_class?: if PROFILE == "Devel" {
                Some("devel")
            } else {
                None
            },

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_decoration_layout: Some(":close"),
                    add_css_class: "flat",

                    pack_start = &gtk::Button {
                        set_icon_name: "about",
                        connect_clicked => AppInput::ShowAboutWindow,
                    }
                },

                add_top_bar = &gtk::Box {
                    add_css_class: "toolbar",
                    set_margin_top: 5,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_hexpand: true,
                    set_halign: gtk::Align::Fill,

                    #[name(url_entry)]
                    gtk::Entry {
                        set_hexpand: true,
                        set_halign: gtk::Align::Fill,
                        set_margin_start: 5,
                        set_margin_end: 0,
                        #[watch]
                        set_buffer: &model.url_entry_buffer,
                        set_placeholder_text: Some("Search the web or enter a link"),
                        set_input_purpose: gtk::InputPurpose::Url,
                        set_input_hints: gtk::InputHints::NO_SPELLCHECK,
                        connect_activate => AppInput::NewWebWindow,
                    },

                    #[name(add_btn)]
                    gtk::Button {
                        set_margin_start: 5,
                        set_margin_end: 5,
                        set_halign: gtk::Align::End,
                        set_icon_name: "plus",
                        set_tooltip_text: Some("New Web Window"),
                        add_css_class: "raised",
                        connect_clicked => AppInput::NewWebWindow,
                    }
                },

                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_vexpand: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        set_halign: gtk::Align::Fill,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            #[local_ref]
                            webwindowcontrolbar_box -> gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 0,
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
        thread::spawn(|| {
            if let Some(dir) = directories::ProjectDirs::from("com", "github.kdwk", "Spidey") {
                create_dir_all(dir.data_dir()).unwrap();
                let adblock_json_file_path = dir
                    .data_dir()
                    .join("block.json")
                    .into_os_string()
                    .into_string()
                    .unwrap();
                File::create(&adblock_json_file_path.clone()[..]);
                let mut adblock_json_file = OpenOptions::new()
                    .write(true)
                    .open(&adblock_json_file_path[..])
                    .unwrap();
                let mut download_blocklist_operation = Easy::new();
                download_blocklist_operation.url(
                    "https://easylist-downloads.adblockplus.org/easylist_min_content_blocker.json",
                )
                .unwrap();
                download_blocklist_operation
                    .write_function(move |data| {
                        adblock_json_file.write_all(data).unwrap();
                        Ok(data.len())
                    })
                    .unwrap();
                download_blocklist_operation.perform().unwrap();
            }
        });
        let webwindowcontrolbars = relm4::factory::FactoryVecDeque::builder(gtk::Box::default())
            .launch()
            .forward(sender.input_sender(), |output| match output {
                WebWindowControlBarOutput::Remove(index) => {
                    AppInput::RemoveWebWindowControlBar(index)
                }
            });
        let model = App {
            webwindowcontrolbars: webwindowcontrolbars,
            url_entry_buffer: gtk::EntryBuffer::default(),
        };
        let webwindowcontrolbar_box = model.webwindowcontrolbars.widget();
        let widgets = view_output!();
        let app = relm4::main_adw_application();
        let mut action_group = RelmActionGroup::<AppWindowActionGroup>::new();
        let show_about: RelmAction<ShowAbout> = RelmAction::new_stateless(move |_| {
            adw::AboutWindow::builder()
                .application_icon("application-x-executable")
                .developer_name("Kdwk")
                .version("1.0")
                .comments("World Wide Web-crawler")
                .website("https://github.com/kdwk/Spidey")
                .issue_url("https://github.com/kdwk/Spidey/issues")
                .copyright("© 2023 Kendrew Leung")
                .build()
                .present();
        });
        app.set_accels_for_action("show_about", &["<Alt>A"]);
        action_group.add_action(show_about);
        action_group.register_for_widget(root);
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
                adw::AboutWindow::builder()
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
