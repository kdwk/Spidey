#![allow(unused_imports)]
#![allow(unused_variables)]
use ashpd::desktop::open_uri::OpenFileRequest;
use directories;
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::Write,
    path::Path,
    process::Command,
    rc::Rc,
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::Duration,
};

use relm4::actions::{AccelsPlus, ActionName, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::glib::clone;
use relm4::gtk::{prelude::WidgetExt, prelude::*, EventControllerMotion};
use relm4::prelude::*;
use tokio;
use webkit6::{gio::SimpleAction, prelude::*};
use webkit6_sys::webkit_web_view_get_settings;

use crate::document::{
    with, Create, Document, FileSystemEntity,
    Folder::{Project, User},
    Mode,
    Project::{Config, Data},
    User::{Documents, Downloads, Pictures},
};
use crate::{
    config::{APP_ID, PROFILE},
    document::with,
};
use crate::{document::Document, smallwebwindow::*};

pub struct WebWindow {
    pub url: String,
    screenshot_flash_box: gtk::Box,
    can_go_back: bool,
    can_go_forward: bool,
}

#[derive(Debug)]
pub enum WebWindowInput {
    Back,
    CreateSmallWebWindow(webkit6::WebView),
    TitleChanged(String),
    LoadChanged(bool, bool),
    InsecureContentDetected,
    Screenshot,
    BeginScreenshotFlash,
    ScreenshotFlashFinished,
    RetroactivelyLoadUserContentFilter(webkit6::UserContentFilterStore),
    ReturnToMainAppWindow,
    ShowHeaderBar,
    HideHeaderBar,
}

#[derive(Debug)]
pub enum WebWindowOutput {
    LoadChanged((bool, bool)),
    TitleChanged(String),
    ReturnToMainAppWindow,
    Close,
}

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

            #[name(toast_overlay)]
            adw::ToastOverlay {
                #[name(main_overlay)]
                gtk::Overlay {
                    #[name(toolbar_view)]
                    add_overlay = &adw::ToolbarView {
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Start,
                        set_top_bar_style: adw::ToolbarStyle::Raised,
                        set_reveal_top_bars: false,

                        #[name(headerbar)]
                        add_top_bar = &adw::HeaderBar {
                            pack_start = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                add_css_class: "webwindow-headerbar",

                                gtk::Button {
                                    set_icon_name: "left",
                                    set_tooltip_text: Some("Back"),
                                    #[watch]
                                    set_sensitive: model.can_go_back,
                                    connect_clicked => WebWindowInput::Back,
                                },

                                gtk::Button {
                                    set_icon_name: "right",
                                    set_tooltip_text: Some("Forward"),
                                    #[watch]
                                    set_sensitive: model.can_go_forward,
                                    connect_clicked => WebWindowInput::Back,
                                }
                            },
                        },
                    },

                    #[name(show_toolbars_box)]
                    add_overlay = &gtk::Box {
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Start,
                        set_height_request: 10,
                    },

                    #[name(web_view)]
                    webkit6::WebView {
                        set_vexpand: true,
                        load_uri: model.url.as_str(),
                        connect_load_changed[sender] => move |this_webview, _load_event| {
                            sender.input(WebWindowInput::LoadChanged(this_webview.can_go_back(), this_webview.can_go_forward()));
                            sender.output(WebWindowOutput::LoadChanged((this_webview.can_go_back(), this_webview.can_go_forward()))).expect("Could not send output WebWindowOutput::LoadChanged");
                        },
                        connect_title_notify[sender] => move |this_webview| {
                            let title = this_webview.title().map(|title| ToString::to_string(&title));
                            sender.input(WebWindowInput::TitleChanged(match title {
                                Some(text) => text,
                                None => String::from("")
                            }));
                        },
                        connect_insecure_content_detected[sender] => move |_, _| {
                            sender.input(WebWindowInput::InsecureContentDetected);
                        },
                        connect_create[sender] => move |this_webview, _navigation_action| {
                            let new_webview = webkit6::glib::Object::builder::<webkit6::WebView>().property("related-view", this_webview).build();
                            new_webview.set_vexpand(true);
                            new_webview.connect_ready_to_show(clone!(@strong sender, @strong new_webview => move |_| {
                                sender.input(WebWindowInput::CreateSmallWebWindow(new_webview.clone()));
                            }));
                            new_webview.into()

                        },
                    },
                }
            },

            connect_close_request[sender] => move |_| {
                sender.output(WebWindowOutput::Close).expect("Could not send output WebWindowOutput::Close");
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
            can_go_back: false,
            can_go_forward: false,
        };
        let widgets = view_output!();
        // Make the main app be aware of this new window so it doesn't quit when main window is closed
        // relm4::main_adw_application().add_window(&Self::builder().root);
        let show_toolbars_event_controller = EventControllerMotion::new();
        show_toolbars_event_controller.connect_enter(clone!(@strong sender => move |_, _, _| {
            sender.input(WebWindowInput::ShowHeaderBar);
        }));
        widgets
            .show_toolbars_box
            .add_controller(show_toolbars_event_controller);
        let hide_toolbars_event_controller = EventControllerMotion::new();
        hide_toolbars_event_controller.connect_leave(clone!(@strong sender => move |_| {
            sender.input(WebWindowInput::HideHeaderBar);
        }));
        widgets
            .headerbar
            .add_controller(hide_toolbars_event_controller);

        // Set settings for the WebView
        if let Some(web_view_settings) = webkit6::prelude::WebViewExt::settings(&widgets.web_view) {
            web_view_settings.set_media_playback_requires_user_gesture(true);
            web_view_settings.set_enable_back_forward_navigation_gestures(true);
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
            session.connect_download_started(clone!(@strong toast_overlay_widget_clone as toast_overlay => move |this_session, download_object| {
                let download_did_fail = Arc::new(AtomicBool::new(false));
                download_object.connect_failed(clone!(@strong toast_overlay ,@strong download_did_fail => move |this_download_object, error| {
                    (*download_did_fail).store(true, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("{}", error.to_string());
                    toast_overlay
                        .add_toast(adw::Toast::new("Download failed"));
                }));
                download_object.connect_finished(clone!(@strong toast_overlay, @strong download_did_fail => move |this_download_object| {
                    if (*download_did_fail).load(std::sync::atomic::Ordering::Relaxed) {return;}
                    let toast = adw::Toast::new("File saved to Downloads folder");
                    toast.set_button_label(Some("Open"));
                    let downloaded_file_path = match this_download_object.destination() {
                        Some(destination_gstring) => destination_gstring.to_string(),
                        None => String::from(""),
                    };
                    toast.connect_button_clicked(move |_| {
                        let file_result = OpenOptions::new()
                            .read(true)
                            .open(Path::new(&downloaded_file_path));
                        relm4::spawn_local(async move {
                            if let Ok(file) = file_result {
                                let _ = OpenFileRequest::default()
                                    .ask(true)
                                    .send_file(&file)
                                    .await
                                    .is_ok_and(|req| {
                                        let _ = req.response();
                                        true
                                    });
                            }
                        });
                    });
                    toast_overlay.add_toast(toast);
                }));
            }));

            // Enable Intelligent Tracking Prevention
            session.set_itp_enabled(true);

            // Handle persistent cookies
            if let Some(cookie_manager) = session.cookie_manager() {
                if let Some(dir) = directories::ProjectDirs::from("com", "github.kdwk", "Spidey") {
                    create_dir_all(dir.data_dir()).expect("Could not create XDG_DATA_DIR");
                    let cookiesdb_file_path = dir.data_dir().join("cookies.sqlite");
                    cookie_manager.set_persistent_storage(
                        cookiesdb_file_path
                            .into_os_string()
                            .into_string()
                            .expect("Could not build cookiesdb_file_path")
                            .as_str(),
                        webkit6::CookiePersistentStorage::Sqlite,
                    );
                }
            }
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
            WebWindowInput::Back => widgets.web_view.go_back(),
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
                let small_web_window_widget = smallwebwindow.widgets().small_web_window.clone();
                smallwebwindow.model().web_view.connect_title_notify(
                    clone!(@strong small_web_window_widget => move |this_webview| {
                        let title = this_webview
                            .title()
                            .map(|title| ToString::to_string(&title));
                        small_web_window_widget
                            .set_title(Some(title.unwrap_or(String::from("")).as_str()));
                    }),
                );
                smallwebwindow.model().web_view.connect_close(
                    clone!(@strong small_web_window_widget => move |this_webview| {
                        small_web_window_widget.close();
                    }),
                );
            }
            WebWindowInput::RetroactivelyLoadUserContentFilter(user_content_filter_store) => {
                if let Some(user_content_manager) = widgets.web_view.user_content_manager() {
                    user_content_filter_store.load(
                        "adblock",
                        gtk::gio::Cancellable::NONE,
                        move |user_content_filter_result| {
                            if let Ok(user_content_filter) = user_content_filter_result {
                                user_content_manager.add_filter(&user_content_filter);
                            }
                        },
                    )
                }
            }
            WebWindowInput::TitleChanged(title) => {
                widgets.web_window.set_title(Some(title.as_str()));
                sender
                    .output(WebWindowOutput::TitleChanged(title))
                    .expect("Could not send output WebWindowOutput::TitleChanged");
            }
            WebWindowInput::LoadChanged(can_go_back, can_go_forward) => {
                self.can_go_back = can_go_back;
                self.can_go_forward = can_go_forward;
            }
            WebWindowInput::InsecureContentDetected => widgets
                .toast_overlay
                .add_toast(adw::Toast::new("This page is insecure")),
            WebWindowInput::Screenshot => {
                widgets.web_view.snapshot(
                    webkit6::SnapshotRegion::Visible,
                    webkit6::SnapshotOptions::INCLUDE_SELECTION_HIGHLIGHTING,
                    gtk::gio::Cancellable::NONE,
                    clone!(@strong widgets.web_window as web_window, @strong widgets.toast_overlay as toast_overlay => move |snapshot_result| match snapshot_result {
                        Ok(texture) => {
                            // Present the WebWindow to show off the beautiful animation that took an afternoon to figure out
                            web_window.present();
                            // Using async but not threads because WebWindowInput cannot be sent across threads due to one of the variants carrying a WebView
                            let animation_timing_handle = relm4::spawn_local(clone!(@strong sender => async move {
                                // Wait for 300ms for the WebWindow to be in focus
                                tokio::time::sleep(Duration::from_millis(300)).await;
                                // Add the screenshot flash box to the main_overlay of the WebWindow
                                sender.input(WebWindowInput::BeginScreenshotFlash);
                                // Wait for the animation to finish
                                tokio::time::sleep(Duration::from_millis(830)).await;
                                // Remoe the screenshot flash box
                                sender.input(WebWindowInput::ScreenshotFlashFinished);
                                // Wait for another 350ms to prevent whiplash
                                tokio::time::sleep(Duration::from_millis(350)).await;
                                // Return focus back to main app window
                                sender
                                    .output(WebWindowOutput::ReturnToMainAppWindow)
                                    .expect("Could not send output WebWindowOutput::ReturnToMainAppWindow");
                            }));
                            // Function to add an error message to explain what went wrong in case of a failed screenshot save
                            let present_error_toast = |error_message: String| {
                                toast_overlay
                                    .add_toast(adw::Toast::new(&error_message));
                            };
                            with(&[Document::at(User(Pictures(&["Screenshot"])), "Screenshot.png", Create::AutoRenameIfExists)],
                                |mut d| {
                                    d["Screenshot.png"].write(&texture.save_to_png_bytes())?;
                                    let toast = adw::Toast::builder()
                                        .title("Screenshot saved to Pictures → Screenshots")
                                        .button_label("Open")
                                        .build();
                                    toast.connect_button_clicked(clone!(@strong d["Screenshot.png"] as png_document => move |_| {
                                        png_document.launch_with_default_app();
                                        // relm4::spawn_local(clone!(@strong screenshot_save_path_final => async move {
                                        //     let screenshot_file = match OpenOptions::new().read(true).open(Path::new(&screenshot_save_path_final)){
                                        //         Ok(file) => file,
                                        //         Err(_) => {
                                        //             eprintln!("Could not open {} for read", screenshot_save_path_final);
                                        //             return;
                                        //         }
                                        //     };
                                        //     let _ = OpenFileRequest::default()
                                        //         .ask(true)
                                        //         .send_file(&screenshot_file)
                                        //         .await
                                        //         .is_ok_and(|req| {
                                        //             let _ = req.response();
                                        //             true
                                        //         });
                                        // }));
                                    }));
                                    toast_overlay.add_toast(toast);
                                    Ok(())
                                });
                            // if let Some(dir) = directories::UserDirs::new() {
                            //     // Create the ~/Pictures/Screenshots folder if it doesn't exist
                            //     if let Err(_) = create_dir_all(Path::new(
                            //         &dir.picture_dir()
                            //             .expect("Could not find XDG_PICTURES_DIR")
                            //             .join("Screenshots")
                            //             .into_os_string()
                            //             .into_string()
                            //             .expect("Could not build path XDG_PICTURES_DIR/Screenshots"),
                            //     )) {
                            //         present_error_toast(
                            //             "Could not create ~/Pictures/Screenshots".into(),
                            //         );
                            //         return;
                            //     }
                            //     // Function to get the screenshot save path and append the suffix to it
                            //     let screenshot_save_path = |suffix: usize| -> String {
                            //         let suffix_str = suffix.to_string();
                            //         let path = dir
                            //             .picture_dir()
                            //             .expect("Could not find XDG_PICTURE_DIR")
                            //             .join("Screenshots")
                            //             .join(
                            //                 "Screenshot".to_owned()
                            //                     + if suffix != 0 { suffix_str.as_str() } else { "" }
                            //                     + ".png",
                            //             )
                            //             .into_os_string()
                            //             .into_string()
                            //             .expect("Could not build path screenshot_save_path");
                            //         path
                            //     };
                            //     // Increment the suffix until the file doesn't already exist in the folder
                            //     let mut suffix: usize = 0;
                            //     let screenshot_save_path_final = {
                            //         while Path::new(screenshot_save_path(suffix).as_str()).exists() {
                            //             suffix += 1;
                            //         }
                            //         screenshot_save_path(suffix)
                            //     };
                            //     // Create the actual file to save the screenshot to
                            //     if let Err(_) = File::create(Path::new(&screenshot_save_path_final))
                            //     {
                            //         present_error_toast(format!(
                            //             "Could not create {}",
                            //             &screenshot_save_path_final
                            //         ));
                            //         return;
                            //     };
                            //     let mut screenshot_file = match OpenOptions::new()
                            //         .write(true)
                            //         .open(Path::new(&screenshot_save_path_final))
                            //     {
                            //         Ok(file) => file,
                            //         Err(_) => {
                            //             present_error_toast(format!(
                            //                 "Could not open {}",
                            //                 &screenshot_save_path_final
                            //             ));
                            //             return;
                            //         }
                            //     };
                            //     // Actually write the PNG bytes to the file
                            //     if let Err(_) =
                            //         screenshot_file.write_all(&texture.save_to_png_bytes())
                            //     {
                            //         present_error_toast(format!(
                            //             "Failed to write to {}",
                            //             &screenshot_save_path_final
                            //         ));
                            //         return;
                            //     };
                            //     // Add a toast to say that the screenshot is saved and a button to open the screenshot
                            //     let toast = adw::Toast::builder()
                            //         .title("Screenshot saved to Pictures → Screenshots")
                            //         .button_label("Open")
                            //         .build();
                            //     toast.connect_button_clicked(move |_| {
                            //         relm4::spawn_local(clone!(@strong screenshot_save_path_final => async move {
                            //             let screenshot_file = match OpenOptions::new().read(true).open(Path::new(&screenshot_save_path_final)){
                            //                 Ok(file) => file,
                            //                 Err(_) => {
                            //                     eprintln!("Could not open {} for read", screenshot_save_path_final);
                            //                     return;
                            //                 }
                            //             };
                            //             let _ = OpenFileRequest::default()
                            //                 .ask(true)
                            //                 .send_file(&screenshot_file)
                            //                 .await
                            //                 .is_ok_and(|req| {
                            //                     let _ = req.response();
                            //                     true
                            //                 });
                            //         }));
                            //     });
                            //     toast_overlay.add_toast(toast);
                        }
                        Err(error) => {
                            eprintln!("Could not save screenshot: {}", error.to_string());
                            toast_overlay
                                .add_toast(adw::Toast::new("Failed to take screenshot"))
                        }
                    }),
                )
            }
            WebWindowInput::BeginScreenshotFlash => {
                widgets.main_overlay.add_overlay(&self.screenshot_flash_box)
            }
            WebWindowInput::ScreenshotFlashFinished => widgets
                .main_overlay
                .remove_overlay(&self.screenshot_flash_box),
            WebWindowInput::ReturnToMainAppWindow => sender
                .output(WebWindowOutput::ReturnToMainAppWindow)
                .expect("Could not send output WebWindowOutput::ReturnToMainAppWindow"),
            WebWindowInput::ShowHeaderBar => widgets.toolbar_view.set_reveal_top_bars(true),
            WebWindowInput::HideHeaderBar => widgets.toolbar_view.set_reveal_top_bars(false),
        }
    }
}
