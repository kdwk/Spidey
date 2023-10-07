#![allow(unused_imports)]
#![allow(unused_variables)]
use chrono::offset::Utc;
use curl::easy::Easy;
use directories;
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::io::Write;
use std::{
    fs::{create_dir_all, File, OpenOptions},
    path::Path,
    thread,
};
use url::Url;

use crate::config::{APP_ID, PROFILE};
use crate::webwindowcontrolbar::*;

pub(super) struct App {
    url_entry_buffer: gtk::EntryBuffer,
    webwindowcontrolbars: relm4::factory::FactoryVecDeque<WebWindowControlBar>,
    user_content_filter_store_option: Option<webkit6::UserContentFilterStore>,
}

relm4::new_action_group!(AppWindowActionGroup, "win");
relm4::new_stateless_action!(ShowAbout, AppWindowActionGroup, "show_about");
#[derive(Debug)]
pub enum AppInput {
    NewWebWindow, // Also handles adding a WebWindowControlBar
    RemoveWebWindowControlBar(DynamicIndex),
    ShowAboutWindow,
    SetUpUserContentFilterStore,
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
        // Set up adblock filters in another thread
        let sender_clone = sender.clone();
        thread::spawn(move || {
            let gschema_id = if PROFILE == "Devel" {
                "com.github.kdwk.Spidey.Devel"
            } else {
                "com.github.kdwk.Spidey"
            };
            // Get the GSettings from GSchema file
            let gsettings = gtk::gio::Settings::new(gschema_id);
            // Get when the XDG_DATA_DIR/adblock.json file has been last updated
            let adblock_json_last_updated_timestamp = gsettings.int64("adblock-json-last-updated");
            // Only download the file from the Internet again if the file has not been updated in the last 7 days
            if Utc::now().timestamp() > adblock_json_last_updated_timestamp + 7 * 24 * 60 * 60 {
                if let Some(dir) = directories::ProjectDirs::from("com", "github.kdwk", "Spidey") {
                    create_dir_all(dir.data_dir()).unwrap();
                    let adblock_json_file_path = dir
                        .data_dir()
                        .join("adblock.json")
                        .into_os_string()
                        .into_string()
                        .unwrap();
                    File::create(&adblock_json_file_path.clone()[..]).unwrap();
                    let mut adblock_json_file = OpenOptions::new()
                        .write(true)
                        .open(&adblock_json_file_path[..])
                        .unwrap();
                    // Set up and perform curl Easy operation to download the adblock.json file from the Internet
                    let mut download_blocklist_operation = Easy::new();
                    download_blocklist_operation.url(
                        "https://easylist-downloads.adblockplus.org/easylist_min_content_blocker.json",
                    )
                    .unwrap();
                    download_blocklist_operation
                        .write_function(move |data| {
                            // Write the downloaded data to adblock.json file
                            adblock_json_file.write_all(data).unwrap();
                            // Return the data length as required by curl
                            Ok(data.len())
                        })
                        .unwrap();
                    download_blocklist_operation.perform().unwrap();
                    // Update the last updated time of adblock.json
                    gsettings
                        .set_int64("adblock-json-last-updated", Utc::now().timestamp())
                        .unwrap();
                }
            } else {
                println!("XDG_DATA_DIR/adblock.json is less than 7 days old. No need to re-download from the Internet.")
            }
            // Set up the UserContentFilterStore no matter if it has been freshly downloaded from the Internet or not
            // This is safe because the update function for this message variant will check if the file exists so we don't need to provide a guarantee here
            sender_clone.input(AppInput::SetUpUserContentFilterStore);
        });

        // Set up WebWindowControlBars
        let webwindowcontrolbars = relm4::factory::FactoryVecDeque::builder(gtk::Box::default())
            .launch()
            .forward(sender.input_sender(), |output| match output {
                WebWindowControlBarOutput::Remove(index) => {
                    AppInput::RemoveWebWindowControlBar(index)
                }
            });

        // Standard component initialization procedures
        let model = App {
            webwindowcontrolbars: webwindowcontrolbars,
            url_entry_buffer: gtk::EntryBuffer::default(),
            user_content_filter_store_option: None,
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
                        self.webwindowcontrolbars
                            .guard()
                            .push_back((final_url, self.user_content_filter_store_option.clone()));
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

            AppInput::SetUpUserContentFilterStore => {
                if let Some(dir) = directories::ProjectDirs::from("com", "github.kdwk", "Spidey") {
                    let user_content_filter_store_path = &dir
                        .data_dir()
                        .join("UserContentFilterStore")
                        .into_os_string()
                        .into_string()
                        .unwrap()[..];

                    // Create the UserContentFilterStore storage location if it doesn't exist
                    create_dir_all(user_content_filter_store_path).unwrap();

                    // Create a new UserContentFilterStore and save to the corresponding field in the struct of self
                    self.user_content_filter_store_option = Some(
                        webkit6::UserContentFilterStore::new(user_content_filter_store_path),
                    );

                    // Save XDG_DATA_DIR/adblock.json into the UserContentFilterStore as a UserContentFilter
                    if let Some(user_content_filter_store) = &self.user_content_filter_store_option
                    {
                        let adblock_json_file_path = &dir
                            .data_dir()
                            .join("adblock.json")
                            .into_os_string()
                            .into_string()
                            .unwrap()[..];
                        match Path::try_exists(Path::new(adblock_json_file_path)) {
                            // Ok(true): path points to existing entity; Ok(false): path is broken
                            Ok(path_is_broken) => {
                                if !path_is_broken {
                                    user_content_filter_store.save_from_file(
                                        "adblock",
                                        &webkit6::gio::File::for_path(adblock_json_file_path),
                                        webkit6::gio::Cancellable::NONE,
                                        |_| {println!("Successfully saved adblock.json into UserContentFilterStore")},
                                    )
                                } else {
                                    eprintln!("XDG_DATA_DIR/adblock.json is a broken path");
                                }
                            }
                            // Err(_): path cannot be verified to exist or otherwise
                            Err(_) => {
                                eprintln!("Cannot verify whether XDG_DATA_DIR/adblock.json exists")
                            }
                        }
                    }
                }
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
