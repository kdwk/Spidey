#![allow(unused_imports)]
#![allow(unused_variables)]
use std::{
    error::Error,
    process::Command,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use relm4::{
    actions::{AccelsPlus, ActionName, RelmAction, RelmActionGroup},
    adw::prelude::*,
    gtk::{
        glib::clone,
        prelude::{WidgetExt, *},
        EventControllerMotion,
    },
    prelude::*,
};
use webkit6::{gio::SimpleAction, glib::GString, prelude::*};
use webkit6_sys::webkit_web_view_get_settings;

use crate::smallwebwindow::*;
use crate::{
    app::process_url,
    config::{APP_ID, PROFILE},
};
use crate::{
    document::{
        with, Create, Document, FileSystemEntity,
        Folder::{self, Project, User},
        LinesBufReaderFileExt, Map, Mode,
        Project::{Config, Data},
        ResultDocumentBoxErrorExt,
        User::{Documents, Downloads, Pictures},
    },
    recipe::{Discard, Log, Pass, Pipe, Recipe, Runnable, Step},
    whoops::{attempt, Catch, IntoWhoops, Whoops},
};

#[tracker::track]
pub struct WebWindow {
    pub url: String,
    title: String,
    screenshot_flash_box: gtk::Box,
    can_go_back: bool,
    can_go_forward: bool,
    go_back_now: bool,
    go_forward_now: bool,
    refresh_now: bool,
    in_title_edit_mode: bool,
    title_edit_textbuffer: gtk::EntryBuffer,
    show_headerbar: bool,
}

#[derive(Debug)]
pub enum WebWindowInput {
    Back,
    Forward,
    Refresh,
    CreateSmallWebWindow(webkit6::WebView),
    TitleChanged(String),
    LoadChanged(bool, bool),
    InsecureContentDetected,
    Screenshot(bool),
    BeginScreenshotFlash,
    ScreenshotFlashFinished,
    RetroactivelyLoadUserContentFilter(webkit6::UserContentFilterStore),
    ReturnToMainAppWindow,
    EnterTitleEditMode,
    LeaveTitleEditMode,
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
                        set_top_bar_style: adw::ToolbarStyle::RaisedBorder,
                        #[track = "model.changed(WebWindow::show_headerbar())"]
                        set_reveal_top_bars: model.show_headerbar,

                        #[name(headerbar)]
                        add_top_bar = &adw::HeaderBar {
                            add_css_class: "undershoot-top",
                            pack_start = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,

                                #[name(back_btn)]
                                gtk::Button {
                                    set_icon_name: "left",
                                    set_tooltip_text: Some("Back"),
                                    #[track = "model.changed(WebWindow::can_go_back())"]
                                    set_sensitive: model.can_go_back,
                                    connect_clicked => WebWindowInput::Back,
                                },

                                #[name(forward_btn)]
                                gtk::Button {
                                    set_icon_name: "right",
                                    set_tooltip_text: Some("Forward"),
                                    #[track = "model.changed(WebWindow::can_go_forward())"]
                                    set_sensitive: model.can_go_forward,
                                    connect_clicked => WebWindowInput::Forward,
                                },

                                #[name(refresh_btn)]
                                gtk::Button {
                                    set_icon_name: "refresh",
                                    set_tooltip_text: Some("Refresh"),
                                    connect_clicked => WebWindowInput::Refresh,
                                },

                                #[name(screenshot_btn)]
                                adw::SplitButton {
                                    set_icon_name: "screenshooter",
                                    set_tooltip_text: Some("Take a screenshot"),
                                    connect_clicked => WebWindowInput::Screenshot(false),
                                    #[wrap(Some)]
                                    set_popover = &gtk::Popover {
                                        set_tooltip_text: Some("Select screenshot area"),

                                    }
                                }
                            },
                            #[wrap(Some)]
                            set_title_widget = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,

                                if model.in_title_edit_mode {
                                    gtk::Entry {
                                        set_width_request: 100,
                                        #[track = "model.changed(WebWindow::in_title_edit_mode())"]
                                        grab_focus: (),
                                        #[track = "model.changed(WebWindow::title_edit_textbuffer())"]
                                        set_buffer: &model.title_edit_textbuffer,
                                        set_placeholder_text: Some("Search the web or enter a link"),
                                        set_input_purpose: gtk::InputPurpose::Url,
                                        set_input_hints: gtk::InputHints::NO_SPELLCHECK,
                                        set_icon_from_icon_name: (gtk::EntryIconPosition::Secondary, Some("arrow3-right-symbolic")),
                                        set_icon_tooltip_text: (gtk::EntryIconPosition::Secondary, Some("Go")),
                                        connect_activate => WebWindowInput::LeaveTitleEditMode,
                                        connect_icon_press[sender] => move |_this_entry, icon_position| {
                                            if let gtk::EntryIconPosition::Secondary = icon_position {
                                                sender.input(WebWindowInput::LeaveTitleEditMode);
                                            }
                                        },
                                    }
                                } else {
                                    gtk::Button {
                                        set_width_request: 100,
                                        set_can_shrink: true,
                                        #[track = "model.changed(WebWindow::title())"]
                                        set_label: model.title.as_str(),
                                        connect_clicked => WebWindowInput::EnterTitleEditMode,
                                    }
                                },
                            }

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
                        #[track = "model.changed(WebWindow::url())"]
                        load_uri: model.url.as_str(),
                        #[track = "model.changed(WebWindow::go_back_now())"]
                        go_back: (),
                        #[track = "model.changed(WebWindow::go_forward_now())"]
                        go_forward: (),
                        #[track = "model.changed(WebWindow::refresh_now())"]
                        reload: (),
                        connect_load_changed[sender] => move |this_webview, _load_event| {
                            sender.input(WebWindowInput::LoadChanged(this_webview.can_go_back(), this_webview.can_go_forward()));
                            sender.output(WebWindowOutput::LoadChanged((this_webview.can_go_back(), this_webview.can_go_forward()))).expect("Could not send output WebWindowOutput::LoadChanged");
                        },
                        connect_title_notify[sender] => move |this_webview| {
                            let title = this_webview.title().map(|title| ToString::to_string(&title));
                            sender.input(WebWindowInput::TitleChanged(match title.clone() {
                                Some(text) => text,
                                None => String::from("")
                            }));
                            sender.output(WebWindowOutput::TitleChanged(match title {
                                Some(text) => text,
                                None => String::from("")
                            })).expect("Could not send WebWindowOutput::TitleChanged")
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
        root: Self::Root,
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
            url: init.0.clone(),
            screenshot_flash_box,
            can_go_back: false,
            can_go_forward: false,
            go_back_now: false,
            go_forward_now: false,
            refresh_now: false,
            show_headerbar: false,
            title: init.0,
            in_title_edit_mode: false,
            title_edit_textbuffer: gtk::EntryBuffer::new(Some("")),
            tracker: 0,
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
                let did_download_fail = Arc::new(AtomicBool::new(false));
                download_object.connect_decide_destination(|this_download_object, suggested_filename| {
                    this_download_object.set_destination(Document::at(User(Downloads(&[])), suggested_filename, Create::No).suggest_rename().as_str());
                    true
                });
                download_object.connect_created_destination(clone!(@strong toast_overlay, @strong did_download_fail => move |this_download_object, destination| {
                    let destination_string = destination.to_string();
                    this_download_object.connect_finished(clone!(@strong toast_overlay, @strong did_download_fail, @strong destination_string => move |this_download_object| {
                        if (*did_download_fail).load(std::sync::atomic::Ordering::Relaxed) {return;}
                        let toast = adw::Toast::new("File saved to Downloads folder");
                        toast.set_button_label(Some("Open"));
                        let toast_overlay_clone = toast_overlay.clone();
                        let destination_string_clone = destination_string.clone();
                        toast.connect_button_clicked(move |_| {
                            match Document::from_path(destination_string_clone.clone(), "download_file", Create::No) {
                                Ok(document) => if let Err(error) = document.launch_with_default_app() {
                                    toast_overlay_clone.add_toast(adw::Toast::new(format!("{}", error).as_str()));
                                }
                                Err(error) => toast_overlay_clone.add_toast(adw::Toast::new(format!("{}", error).as_str()))
                            };
                        });
                        toast_overlay.add_toast(toast);
                    }));
                }));
                download_object.connect_failed(clone!(@strong toast_overlay ,@strong did_download_fail => move |this_download_object, error| {
                    (*did_download_fail).store(true, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("{}", error.to_string());
                    toast_overlay
                        .add_toast(adw::Toast::new("Download failed"));
                }));
            }));

            // Enable Intelligent Tracking Prevention
            session.set_itp_enabled(true);

            // Handle persistent cookies
            if let Some(cookie_manager) = session.cookie_manager() {
                with(
                    &[Document::at(
                        Project(Data(&[]).with_id("com", "github.kdwk", "Spidey")),
                        "cookies.sqlite",
                        Create::No,
                    )],
                    |d| {
                        cookie_manager.set_persistent_storage(
                            d["cookies.sqlite"].path().as_str(),
                            webkit6::CookiePersistentStorage::Sqlite,
                        );
                    },
                );
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
        self.reset();
        let sender_clone = sender.clone();
        match message {
            WebWindowInput::Back => self.set_go_back_now(!self.go_back_now),
            WebWindowInput::Forward => self.set_go_forward_now(!self.go_forward_now),
            WebWindowInput::Refresh => self.set_refresh_now(!self.refresh_now),
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
                self.set_title(title.clone());
                sender
                    .output(WebWindowOutput::TitleChanged(title))
                    .expect("Could not send output WebWindowOutput::TitleChanged");
            }
            WebWindowInput::EnterTitleEditMode => {
                self.set_title_edit_textbuffer(gtk::EntryBuffer::new(Some(self.url.clone())));
                self.set_in_title_edit_mode(true);
            }
            WebWindowInput::LeaveTitleEditMode => {
                let input = self.title_edit_textbuffer.text().to_string();
                self.title_edit_textbuffer.set_text("");
                self.set_in_title_edit_mode(false);
                let url = match process_url(input.clone()) {
                    Some(url) => url,
                    None => self.url.clone(),
                };
                if url != self.url {
                    self.set_title(input);
                    self.set_url(url)
                }
            }
            WebWindowInput::LoadChanged(can_go_back, can_go_forward) => {
                self.set_can_go_back(can_go_back);
                self.set_can_go_forward(can_go_forward);
            }
            WebWindowInput::InsecureContentDetected => widgets
                .toast_overlay
                .add_toast(adw::Toast::new("This page is insecure")),
            WebWindowInput::Screenshot(need_return_main_app) => {
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
                                tokio::time::sleep(Duration::from_millis(300)).await; // Wait for 300ms for the WebWindow to be in focus
                                sender.input(WebWindowInput::BeginScreenshotFlash); // Add the screenshot flash box to the main_overlay of the WebWindow
                                tokio::time::sleep(Duration::from_millis(830)).await; // Wait for the animation to finish
                                sender.input(WebWindowInput::ScreenshotFlashFinished); // Remove the screenshot flash box
                                if need_return_main_app {
                                    tokio::time::sleep(Duration::from_millis(350)).await; // Wait for another 350ms to prevent whiplash
                                    sender // Return focus back to main app window
                                        .output(WebWindowOutput::ReturnToMainAppWindow)
                                        .expect("Could not send output WebWindowOutput::ReturnToMainAppWindow");
                                }
                            }));
                            with(&[Document::at(User(Pictures(&["Screenshots"])), "Screenshot.png", Create::AutoRenameIfExists)],
                                |mut d| {
                                    d["Screenshot.png"].replace_with(&texture.save_to_png_bytes())?;
                                    let toast = adw::Toast::builder()
                                        .title("Screenshot saved to Pictures → Screenshots")
                                        .button_label("Open")
                                        .build();
                                    let png_document = d["Screenshot.png"].clone();
                                    let toast_overlay_clone = toast_overlay.clone();
                                    toast.connect_button_clicked(move |_| {
                                        match png_document.launch_with_default_app() {
                                            Ok(_) => {}
                                            Err(error) => toast_overlay_clone.add_toast(adw::Toast::new(format!("{}", error).as_str()))
                                        }
                                    });
                                    toast_overlay.add_toast(toast);
                                    Ok(())
                                });
                        }
                        Err(error) => {
                            eprintln!("Could not save screenshot: {}", error);
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
            WebWindowInput::ShowHeaderBar => self.set_show_headerbar(true),
            WebWindowInput::HideHeaderBar => self.set_show_headerbar(false),
        }
        self.update_view(widgets, sender_clone);
    }
}
