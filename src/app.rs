#![allow(unused_imports)]
#![allow(unused_variables)]
use chrono::offset::Utc;
use directories;
use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::{glib::clone, prelude::*};
use relm4::{prelude::*, ComponentController};
use reqwest;
use std::error::Error;
use std::io::Write;
use std::{
    fs::{create_dir_all, File, OpenOptions},
    path::Path,
    thread,
};
use url::Url;
use webkit6::prelude::WebViewExt;

use crate::config::{APP_ID, PROFILE, VERSION};
use crate::document::FileSystemEntity;
use crate::document::{
    with, Create, Document,
    Folder::{Project, User},
    Mode,
    Project::{Config, Data},
    User::{Documents, Downloads, Pictures},
};
use crate::{webwindowcontrolbar::*, AppActionGroup, PresentMainWindow};

pub(super) struct App {
    url_entry_buffer: gtk::EntryBuffer,
    webwindowcontrolbars: relm4::factory::FactoryVecDeque<WebWindowControlBar>,
    user_content_filter_store_option: Option<webkit6::UserContentFilterStore>,
}

relm4::new_action_group!(AppWindowActionGroup, "win");
relm4::new_stateless_action!(ShowAboutWindow, AppWindowActionGroup, "show_about");
relm4::new_stateless_action!(
    ShowKeyboardShortcutsWindow,
    AppWindowActionGroup,
    "show_shortcuts"
);
#[derive(Debug)]
pub enum AppInput {
    NewWebWindow, // Also handles adding a WebWindowControlBar
    RemoveWebWindowControlBar(DynamicIndex),
    ShowAboutWindow,
    ShowKeyboardShortcutsWindow,
    SetUpUserContentFilterStore,
    PresentWindow,
    SaveUrls,
    RestoreUrls,
    FocusUrlEntry,
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[name(app_window)]
        adw::Window {
            set_default_height: 530,
            set_default_width: 400,
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
            },
            connect_is_active_notify => AppInput::FocusUrlEntry
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Set up adblock filters in another thread
        // let sender_clone = sender.clone();
        thread::spawn(clone!(@strong sender => move || {
            println!("Successfully entered adblock json download thread");
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
                // Set up the UserContentFilterStore no matter before it has been downloaded from the Internet so WebWindows launched now can still have adblock from the old adblock.json
                // This is safe because the update function for this message variant will check if the file exists so we don't need to provide a guarantee here
                sender.input(AppInput::SetUpUserContentFilterStore);
                println!("XDG_DATA_DIR/adblock.json is older than 7 days. Downloading from the Internet...");

                with(&[Document::at(Project(Data(&[]).with_id("com", "github.kdwk", "Spidey")), "adblock.json", Create::OnlyIfNotExists)],
                    |mut d| {
                        reqwest::blocking::get("https://easylist-downloads.adblockplus.org/easylist_min_content_blocker.json")?
                            .copy_to(&mut d["adblock.json"].file(Mode::Replace)?)?;
                        // Update the last updated time of adblock.json
                        gsettings
                            .set_int64("adblock-json-last-updated", Utc::now().timestamp())
                            .expect("Could not update GSettings value 'adblock-json-last-updated'");
                        // Set up UserContentFilterStore again with new adblock.json
                        sender.input(AppInput::SetUpUserContentFilterStore);
                        Ok(())
                    });
            } else {
                println!("XDG_DATA_DIR/adblock.json is less than 7 days old. No need to re-download from the Internet.");
                // Set up UserContentFilterStore with either old adblock or no adblock
                sender.input(AppInput::SetUpUserContentFilterStore);
            }
            println!("Done with adblock json download thread");
        }));

        // Set up WebWindowControlBars
        let webwindowcontrolbars = relm4::factory::FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |output| match output {
                WebWindowControlBarOutput::Remove(index) => {
                    AppInput::RemoveWebWindowControlBar(index)
                }
                WebWindowControlBarOutput::ReturnToMainAppWindow => AppInput::PresentWindow,
            });

        sender.input(AppInput::RestoreUrls);

        // Standard component initialization procedures
        let model = App {
            webwindowcontrolbars: webwindowcontrolbars,
            url_entry_buffer: gtk::EntryBuffer::default(),
            user_content_filter_store_option: None,
        };
        let webwindowcontrolbar_box = model.webwindowcontrolbars.widget();
        let widgets = view_output!();
        let app = relm4::main_adw_application();
        let mut app_window_action_group = RelmActionGroup::<AppWindowActionGroup>::new();
        // let sender_clone = sender.clone();
        let show_about_window: RelmAction<ShowAboutWindow> =
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(AppInput::ShowAboutWindow);
            }));
        let show_keyboard_shortcuts_window: RelmAction<ShowKeyboardShortcutsWindow> =
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(AppInput::ShowKeyboardShortcutsWindow);
            }));
        app.set_accelerators_for_action::<ShowAboutWindow>(&["<Alt>A"]);
        app.set_accelerators_for_action::<ShowKeyboardShortcutsWindow>(&["<Ctrl>question"]);
        app_window_action_group.add_action(show_about_window);
        app_window_action_group.add_action(show_keyboard_shortcuts_window);
        app_window_action_group.register_for_widget(root);
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
                if let Some(final_url) = url_processed_result.ok() {
                    self.webwindowcontrolbars
                        .guard()
                        .push_back((final_url, self.user_content_filter_store_option.clone()));
                    self.url_entry_buffer.set_text("");
                    sender.input(AppInput::SaveUrls);
                }
            }

            AppInput::RemoveWebWindowControlBar(id) => {
                self.webwindowcontrolbars.guard().remove(id.current_index());
                sender.input(AppInput::SaveUrls);
            }

            AppInput::ShowAboutWindow => {
                adw::AboutWindow::builder()
                    .transient_for(root)
                    .application_icon(if PROFILE == "Devel" {
                        "com.github.kdwk.Spidey.Devel"
                    } else {
                        "com.github.kdwk.Spidey"
                    })
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
                let user_content_filter_store_folder = Project(
                    Data(&["UserContentFilterStore"]).with_id("com", "github.kdwk", "Spidey"),
                );

                if !user_content_filter_store_folder.exists() {
                    create_dir_all(user_content_filter_store_folder.path())
                        .expect("Could not create Project(Data(&[\"UserContentFilterStore\"]).with_id(\"com\", \"github.kdwk\", \"Spidey\")");
                }

                self.user_content_filter_store_option = Some(webkit6::UserContentFilterStore::new(
                    user_content_filter_store_folder.path().as_str(),
                ));

                if let Some(user_content_filter_store) = &self.user_content_filter_store_option {
                    with(
                        &[Document::at(
                            Project(Data(&[]).with_id("com", "github.kdwk", "Spidey")),
                            "adblock.json",
                            Create::No,
                        )],
                        |d| {
                            user_content_filter_store.save_from_file(
                                "adblock",
                                &webkit6::gio::File::for_path(d["adblock.json"].path()),
                                webkit6::gio::Cancellable::NONE,
                                |_| println!("Successfully saved adblock.json into UserContentFilterStore")
                            );
                            Ok(())
                        },
                    );
                }
            }

            AppInput::PresentWindow => root.present(),

            AppInput::ShowKeyboardShortcutsWindow => {
                // let shortcuts_window = gtk::ShortcutsWindow::builder()
                //     .transient_for(root)
                //     .modal(true)
                //     .child(&gtk::ShortcutsSection::builder()
                //             .section_name("app")
                //             .title("App shortcuts")
                //             .build())
                //     .build();
                // shortcuts_window.present();
            }

            AppInput::RestoreUrls => {
                let gschema_id = if PROFILE == "Devel" {
                    "com.github.kdwk.Spidey.Devel"
                } else {
                    "com.github.kdwk.Spidey"
                };
                let gsettings = gtk::gio::Settings::new(gschema_id);
                let urls = gsettings.string("urls").to_string();
                let url_vec = if urls.len() > 0 {
                    urls.split(" ")
                        .map(|url| url.to_string())
                        .collect::<Vec<String>>()
                } else {
                    vec![]
                };
                for url in url_vec {
                    self.webwindowcontrolbars
                        .guard()
                        .push_back((url, self.user_content_filter_store_option.clone()));
                }
            }

            AppInput::SaveUrls => {
                let gschema_id = if PROFILE == "Devel" {
                    "com.github.kdwk.Spidey.Devel"
                } else {
                    "com.github.kdwk.Spidey"
                };
                let gsettings = gtk::gio::Settings::new(gschema_id);
                let urls = self
                    .webwindowcontrolbars
                    .guard()
                    .iter()
                    .map(|webwindowcontrolbar| {
                        if let Some(uri) = webwindowcontrolbar.webwindow.widgets().web_view.uri() {
                            uri.to_string()
                        } else {
                            String::from("")
                        }
                    })
                    .collect::<Vec<String>>();
                let mut urls_string = String::from("");
                for url in urls {
                    if url.len() > 0 {
                        urls_string.push_str(&url);
                        urls_string.push_str(" ");
                    }
                }
                if urls_string.ends_with(" ") {
                    urls_string.pop();
                }
                let _ = gsettings.set_string("urls", urls_string.as_str());
            }

            AppInput::FocusUrlEntry => {
                println!("focus");
                widgets.url_entry.grab_focus();
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
