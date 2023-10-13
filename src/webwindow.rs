#![allow(unused_imports)]
#![allow(unused_variables)]
use directories;
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::Write,
    path::Path,
    thread,
    time::Duration,
};

use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use tokio;
use webkit6::prelude::*;
use webkit6_sys::webkit_web_view_get_settings;

use crate::config::{APP_ID, PROFILE};
use crate::smallwebwindow::*;

pub struct WebWindow {
    url: String,
    screenshot_flash_box: gtk::Box,
}

#[derive(Debug)]
pub enum WebWindowInput {
    CreateSmallWebWindow(webkit6::WebView),
    TitleChanged(String),
    InsecureContentDetected,
    Screenshot,
    BeginScreenshotFlash,
    ScreenshotFlashFinished,
}

#[derive(Debug)]
pub enum WebWindowOutput {
    LoadChanged((bool, bool)),
    TitleChanged(String),
    ReturnToMainAppWindow,
    Close,
}

relm4::new_action_group!(WebWindowActionGroup, "win");
relm4::new_stateless_action!(GoBack, WebWindowActionGroup, "go_back");
#[relm4::component(pub)]
impl Component for WebWindow {
    type Init = (String, Option<webkit6::UserContentFilterStore>);
    type Input = WebWindowInput;
    type Output = WebWindowOutput;
    type CommandOutput = ();

    view! {
        #[name(web_window)]
        adw::Window {
            set_default_height: 1000,
            set_default_width: 1000,

            #[name(main_overlay)]
            gtk::Overlay {
                add_overlay = &gtk::WindowHandle {
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Start,
                    set_height_request: 20,
                },
                add_overlay = &gtk::WindowControls {
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::Start,
                    set_margin_top: 5,
                    set_margin_end: 5,
                    set_side: gtk::PackType::End,
                    add_css_class: "webwindow-close",
                },
                #[name(toast_overlay)]
                adw::ToastOverlay {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[name(web_view)]
                        webkit6::WebView {
                            set_vexpand: true,
                            load_uri: model.url.as_str(),
                            connect_load_changed[sender] => move |this_webview, _load_event| {
                                sender.output(WebWindowOutput::LoadChanged((this_webview.can_go_back(), this_webview.can_go_forward()))).unwrap();
                            },
                            connect_title_notify[sender] => move |this_webview| {
                                let title = this_webview.title().map(|title| ToString::to_string(&title));
                                sender.input(WebWindowInput::TitleChanged(match title {
                                    Some(text) => text,
                                    None => "".into()
                                }));
                            },
                            connect_insecure_content_detected[sender] => move |_, _| {
                                sender.input(WebWindowInput::InsecureContentDetected);
                            },
                            connect_create[sender] => move |this_webview, _navigation_action| {
                                let new_webview = webkit6::glib::Object::builder::<webkit6::WebView>().property("related-view", this_webview).build();
                                new_webview.set_vexpand(true);
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
                sender.output(WebWindowOutput::Close).unwrap();
                gtk::glib::Propagation::Stop
            } ,

            present: (),
        }
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Standard component initialization
        let screenshot_flash_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::Fill)
            .build();
        screenshot_flash_box.add_css_class("screenshot-in-progress");
        let model = WebWindow {
            url: init.0,
            screenshot_flash_box,
        };
        let widgets = view_output!();

        // Set settings for the WebView
        if let Some(web_view_settings) = webkit6::prelude::WebViewExt::settings(&widgets.web_view) {
            web_view_settings.set_media_playback_requires_user_gesture(true);
            if PROFILE == "Devel" {
                web_view_settings.set_enable_developer_extras(true);
            }
        }

        // Set up adblock
        if let Some(user_content_manager) = widgets.web_view.user_content_manager() {
            if let Some(user_content_filter_store) = init.1 {
                user_content_filter_store.load(
                    "adblock",
                    gtk::gio::Cancellable::NONE,
                    move |user_content_filter_result| {
                        if let Ok(user_content_filter) = user_content_filter_result {
                            user_content_manager.add_filter(&user_content_filter);
                        }
                    },
                );
            }
        }

        // Handle things related to the Network Session
        let toast_overlay_widget_clone = widgets.toast_overlay.clone();
        if let Some(session) = widgets.web_view.network_session() {
            // Handle downloads
            session.connect_download_started(move |this_session, download_object| {
                let toast_overlay_widget_clone_clone_1 = toast_overlay_widget_clone.clone();
                let toast_overlay_widget_clone_clone_2 = toast_overlay_widget_clone.clone();
                download_object.connect_failed(move |this_download_object, error| {
                    eprintln!("{}", error.to_string());
                    toast_overlay_widget_clone_clone_1
                        .add_toast(adw::Toast::new("Download failed"));
                });
                download_object.connect_finished(move |this_download_object| {
                    toast_overlay_widget_clone_clone_2
                        .add_toast(adw::Toast::new("File saved to Downloads folder"));
                    //TODO: add button to open file
                });
            });

            // Enable Intelligent Tracking Prevention
            session.set_itp_enabled(true);

            // Handle persistent cookies
            if let Some(cookie_manager) = session.cookie_manager() {
                if let Some(dir) = directories::ProjectDirs::from("com", "github.kdwk", "Spidey") {
                    create_dir_all(dir.data_dir()).unwrap();
                    let cookiesdb_file_path = dir.data_dir().join("cookies.sqlite");
                    cookie_manager.set_persistent_storage(
                        &cookiesdb_file_path.into_os_string().into_string().unwrap()[..],
                        webkit6::CookiePersistentStorage::Sqlite,
                    );
                }
            }
        }

        let app = relm4::main_adw_application();
        let mut action_group = RelmActionGroup::<WebWindowActionGroup>::new();
        let web_view_widget_clone = widgets.web_view.clone();
        let go_back: RelmAction<GoBack> = RelmAction::new_stateless(move |_| {
            web_view_widget_clone.go_back();
        });
        app.set_accelerators_for_action::<GoBack>(&["<Ctrl>leftarrow"]);
        action_group.add_action(go_back);
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
            WebWindowInput::CreateSmallWebWindow(new_webview) => {
                let height_over_width =
                    widgets.web_window.height() as f32 / widgets.web_window.width() as f32;
                let smallwebwindow_width = widgets.web_window.width() / 2;
                let smallwebwindow_height =
                    (smallwebwindow_width as f32 * height_over_width + 100.0) as i32;
                let smallwebwindow = SmallWebWindow::builder()
                    .transient_for(root)
                    .launch((new_webview, (smallwebwindow_width, smallwebwindow_height)))
                    .detach();
                let small_web_window_widget_clone =
                    smallwebwindow.widgets().small_web_window.clone();
                smallwebwindow
                    .model()
                    .web_view
                    .connect_title_notify(move |this_webview| {
                        let title = this_webview
                            .title()
                            .map(|title| ToString::to_string(&title));
                        small_web_window_widget_clone
                            .set_title(Some(&title.unwrap_or(String::from(""))[..]));
                    });
                let small_web_window_widget_clone =
                    smallwebwindow.widgets().small_web_window.clone();
                smallwebwindow
                    .model()
                    .web_view
                    .connect_close(move |this_webview| {
                        small_web_window_widget_clone.close();
                    });
            }
            WebWindowInput::TitleChanged(title) => {
                widgets.web_window.set_title(Some(title.as_str()));
                sender.output(WebWindowOutput::TitleChanged(title)).unwrap();
            }
            WebWindowInput::InsecureContentDetected => widgets
                .toast_overlay
                .add_toast(adw::Toast::new("This page is insecure")),
            WebWindowInput::Screenshot => {
                let web_window_widget_clone = widgets.web_window.clone();
                let toast_overlay_widget_clone = widgets.toast_overlay.clone();
                widgets.web_view.snapshot(
                    webkit6::SnapshotRegion::FullDocument,
                    webkit6::SnapshotOptions::INCLUDE_SELECTION_HIGHLIGHTING,
                    gtk::gio::Cancellable::NONE,
                    move |snapshot_result| match snapshot_result {
                        Ok(texture) => {
                            // Present the WebWindow to show off the beautiful animation that took an afternoon to figure out
                            web_window_widget_clone.present();
                            let sender_clone = sender.clone();
                            // Using async but not threads because WebWindowInput cannot be sent across threads due to one of the variants carrying a WebView
                            let animation_timing_handle = relm4::spawn_local(async move {
                                // Wait for 300ms for the WebWindow to be in focus
                                tokio::time::sleep(Duration::from_millis(300)).await;
                                // Add the screenshot flash box to the main_overlay of the WebWindow
                                sender_clone.input(WebWindowInput::BeginScreenshotFlash);
                                // Wait for the animation to finish
                                tokio::time::sleep(Duration::from_millis(830)).await;
                                // Remoe the screenshot flash box
                                sender_clone.input(WebWindowInput::ScreenshotFlashFinished);
                                // Wait for another 350ms to prevent whiplash
                                tokio::time::sleep(Duration::from_millis(350)).await;
                                // Return focus back to main app window
                                sender_clone.output(WebWindowOutput::ReturnToMainAppWindow);
                            });
                            // Function to add an error message to explain what went wrong in case of a failed screenshot save
                            let present_error_toast = |error_message: String| {
                                toast_overlay_widget_clone
                                    .add_toast(adw::Toast::new(&error_message));
                            };
                            if let Some(dir) = directories::UserDirs::new() {
                                // Create the ~/Pictures/Screenshots folder if it doesn't exist
                                if let Err(_) = create_dir_all(Path::new(
                                    &dir.picture_dir()
                                        .unwrap()
                                        .join("Screenshots")
                                        .into_os_string()
                                        .into_string()
                                        .unwrap(),
                                )) {
                                    present_error_toast(
                                        "Could not create ~/Pictures/Screenshots".into(),
                                    );
                                    return;
                                }
                                // Function to get the screenshot save path and append the suffix to it
                                let screenshot_save_path = |suffix: usize| -> String {
                                    let suffix_str = suffix.to_string();
                                    let path = dir
                                        .picture_dir()
                                        .unwrap()
                                        .join("Screenshots")
                                        .join(
                                            "Screenshot".to_owned()
                                                + if suffix != 0 { &suffix_str[..] } else { "" }
                                                + ".png",
                                        )
                                        .into_os_string()
                                        .into_string()
                                        .unwrap();
                                    path
                                };
                                // Increment the suffix until the file doesn't already exist in the folder
                                let mut suffix: usize = 0;
                                let screenshot_save_path_final = {
                                    while Path::new(&screenshot_save_path(suffix)[..]).exists() {
                                        suffix += 1;
                                    }
                                    screenshot_save_path(suffix)
                                };
                                // Create the actual file to save the screenshot to
                                if let Err(_) = File::create(Path::new(&screenshot_save_path_final))
                                {
                                    present_error_toast(format!(
                                        "Could not create {}",
                                        &screenshot_save_path_final
                                    ));
                                    return;
                                };
                                let mut screenshot_file = match OpenOptions::new()
                                    .write(true)
                                    .open(Path::new(&screenshot_save_path_final))
                                {
                                    Ok(file) => file,
                                    Err(_) => {
                                        present_error_toast(format!(
                                            "Could not open {}",
                                            &screenshot_save_path_final
                                        ));
                                        return;
                                    }
                                };
                                // Actually write the PNG bytes to the file
                                if let Err(_) =
                                    screenshot_file.write_all(&texture.save_to_png_bytes())
                                {
                                    present_error_toast(format!(
                                        "Failed to write to {}",
                                        &screenshot_save_path_final
                                    ));
                                    return;
                                };
                                toast_overlay_widget_clone.add_toast(adw::Toast::new(
                                    "Screenshot saved to Pictures→Screenshots",
                                ));
                            }
                        }
                        Err(error) => {
                            eprintln!("Could not save screenshot: {}", error.to_string());
                            toast_overlay_widget_clone
                                .add_toast(adw::Toast::new("Failed to take screenshot"))
                        }
                    },
                )
            }
            WebWindowInput::BeginScreenshotFlash => {
                widgets.main_overlay.add_overlay(&self.screenshot_flash_box)
            }
            WebWindowInput::ScreenshotFlashFinished => widgets
                .main_overlay
                .remove_overlay(&self.screenshot_flash_box),
        }
    }
}
