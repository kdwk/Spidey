#![allow(unused_imports)]
#![allow(unused_variables)]
use core::fmt::Display;
use documents::prelude::*;
use std::{
    error::Error,
    process::Command,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use adw::{Toast, ToastOverlay};
use relm4::{
    actions::{AccelsPlus, ActionName, RelmAction, RelmActionGroup},
    adw::prelude::*,
    gtk::{
        gdk::ContentProvider,
        glib::clone,
        prelude::{WidgetExt, *},
        EventControllerMotion,
    },
    menu,
    prelude::*,
};
use tracing_subscriber::fmt::format::Full;
use webkit6::{
    gdk::RGBA,
    gio::SimpleAction,
    glib::{GString, Variant, VariantTy},
    prelude::*,
    ContextMenuItem, WebView,
};
use webkit6_sys::webkit_web_view_get_settings;

use crate::smallwebwindow::*;
use crate::{
    app::process_url,
    config::{APP_ID, PROFILE},
};
use crate::{
    recipe::{Discard, Log, Pass, Pipe, Recipe, Runnable, Step},
    whoops::{attempt, Catch, IntoWhoops, Whoops},
};

fn match_style_with_rgb(main_app: adw::Application) -> RGBA {
    match main_app.style_manager().color_scheme() {
        adw::ColorScheme::Default
        | adw::ColorScheme::ForceLight
        | adw::ColorScheme::PreferLight => RGBA::new(0.949019608, 0.949019608, 0.949019608, 1.0),
        adw::ColorScheme::ForceDark | adw::ColorScheme::PreferDark => {
            RGBA::new(0.164705882, 0.164705882, 0.164705882, 1.0)
        }
        _ => RGBA::new(0.949019608, 0.949019608, 0.949019608, 1.0),
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct Inhibited {
    no_of_inhibitions: u32,
}

impl Inhibited {
    fn new() -> Self {
        Self {
            no_of_inhibitions: 0,
        }
    }
    fn inhibit(&mut self) {
        self.no_of_inhibitions += 1;
    }
    fn release(&mut self) {
        if let Some(result) = self.no_of_inhibitions.checked_sub(1) {
            self.no_of_inhibitions = result;
        }
    }
    fn is_clear(&self) -> bool {
        if self.no_of_inhibitions == 0 {
            true
        } else {
            false
        }
    }
}

#[tracker::track]
#[derive(Clone)]
pub struct WebWindow {
    url: String,
    title: String,
    screenshot_flash_box: gtk::Box,
    can_go_back: bool,
    can_go_forward: bool,
    fullscreen: bool,
    in_title_edit_mode: bool,
    can_hide_headerbar: Inhibited,
    pin_headerbar: bool,
    show_headerbar: bool,
    web_view: Option<WebView>,
    toast_overlay: Option<ToastOverlay>,
}

#[derive(Debug)]
pub enum WebWindowInput {
    Back,
    Forward,
    Refresh,
    CopyLink,
    LoadUrl(String),
    CreateSmallWebWindow(webkit6::WebView),
    TitleChanged(String),
    UrlChanged(String),
    NavigationHistoryChanged(bool, bool),
    InsecureContentDetected,
    Screenshot(bool, webkit6::SnapshotRegion),
    BeginScreenshotFlash,
    ScreenshotFlashFinished,
    RetroactivelyLoadUserContentFilter(webkit6::UserContentFilterStore),
    ReturnToMainAppWindow,
    EnterTitleEditMode,
    LeaveTitleEditMode,
    ShowHeaderBar,
    BeginHideHeaderBarTimeout,
    HideHeaderBar,
    ToggleFullscreen,
    TogglePinHeaderBar,
    InhibitHideHeaderBar,
    ReleaseHideHeaderBar,
    Peek(String),
    ShowToast(String),
}

#[derive(Debug)]
pub enum WebWindowOutput {
    LoadChanged(bool, bool),
    UrlChanged(String),
    TitleChanged(String),
    ReturnToMainAppWindow,
    Close,
}

relm4::new_action_group!(WebWindowActionGroup, "webwindow");
relm4::new_stateless_action!(
    FullPageScreenshotAction,
    WebWindowActionGroup,
    "fullpage-screenshot"
);
relm4::new_stateful_action!(PeekAction, WebWindowActionGroup, "peek", String, ());
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
                        #[track = "model.changed(WebWindow::show_headerbar()) || model.changed(WebWindow::can_hide_headerbar())"]
                        set_reveal_top_bars: if model.can_hide_headerbar.is_clear() {
                            model.show_headerbar
                        } else {true},

                        #[name(headerbar)]
                        add_top_bar = &adw::HeaderBar {
                            #[track = "model.changed(WebWindow::fullscreen())"]
                            set_decoration_layout: if model.fullscreen {
                                Some(":")
                            } else {
                                Some(":close")
                            },
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
                                    set_icon_name: "arrow-circular-top-right",
                                    set_tooltip_text: Some("Refresh"),
                                    connect_clicked => WebWindowInput::Refresh,
                                },

                                #[name(screenshot_btn)]
                                adw::SplitButton {
                                    set_icon_name: "screenshooter",
                                    set_tooltip_text: Some("Take a screenshot"),
                                    connect_clicked => WebWindowInput::Screenshot(false, webkit6::SnapshotRegion::Visible),
                                    #[wrap(Some)]
                                    set_popover = &gtk::PopoverMenu::from_model(Some(&screenshot_menu)) {
                                        connect_show => WebWindowInput::InhibitHideHeaderBar,
                                        connect_closed => WebWindowInput::ReleaseHideHeaderBar,
                                    },
                                }
                            },
                            #[wrap(Some)]
                            set_title_widget = &adw::Clamp {
                                set_maximum_size: 350,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,

                                    if model.in_title_edit_mode {
                                        #[name(title_edit_entry)]
                                        gtk::Entry {
                                            set_margin_start: 24,
                                            // set_width_request: 350,
                                            #[track = "model.changed(WebWindow::in_title_edit_mode())"]
                                            grab_focus: (),
                                            #[name(title_edit_entry_buffer)]
                                            set_buffer = &gtk::EntryBuffer {
                                                #[track = "model.changed(WebWindow::url())"]
                                                set_text: model.url.clone(),
                                            },
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
                                            set_margin_start: 24,
                                            // set_width_request: 350,
                                            set_can_shrink: true,
                                            set_tooltip_text: Some("Click to enter link or search"),

                                            #[wrap(Some)]
                                            set_child = &gtk::Box {
                                                set_orientation: gtk::Orientation::Horizontal,
                                                set_halign: gtk::Align::Center,
                                                #[name(padlock_image)]
                                                gtk::Image {
                                                    #[track = "model.changed(WebWindow::url())"]
                                                    set_from_icon_name: if model.url.starts_with("https://") || model.url.starts_with("webkit://") {
                                                        Some("padlock2")
                                                    } else if model.url.starts_with("http://") {
                                                        Some("padlock2-open")
                                                    } else {None},
                                                },

                                                gtk::Label {
                                                    set_margin_start: 7,
                                                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                                                    #[track = "model.changed(WebWindow::title())"]
                                                    set_label: model.title.as_str()
                                                }
                                            },

                                            connect_clicked => WebWindowInput::EnterTitleEditMode,
                                        }
                                    },

                                    gtk::Button {
                                        set_icon_name: "copy",
                                        set_tooltip_text: Some("Copy link"),
                                        add_css_class: "flat",
                                        connect_clicked => WebWindowInput::CopyLink,
                                    }
                                },
                            },

                            pack_end = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,

                                gtk::Button {
                                    set_icon_name: "pin",
                                    #[track = "model.changed(WebWindow::pin_headerbar())"]
                                    set_tooltip_text: if model.pin_headerbar {
                                        Some("Unpin header bar")
                                    } else {
                                        Some("Pin header bar")
                                    },
                                    #[track = "model.changed(WebWindow::pin_headerbar())"]
                                    add_css_class?: if model.pin_headerbar {
                                        Some("raised")
                                    } else {None},
                                    #[track = "model.changed(WebWindow::pin_headerbar())"]
                                    remove_css_class?: if !model.pin_headerbar {
                                        Some("raised")
                                    } else {None},
                                    connect_clicked => WebWindowInput::TogglePinHeaderBar
                                },

                                gtk::Button {
                                    set_icon_name: "move-to-window",
                                    set_tooltip_text: Some("Return to main window"),
                                    connect_clicked => WebWindowInput::ReturnToMainAppWindow,
                                },

                                #[name(toggle_fullscreen_btn)]
                                gtk::Button {
                                    #[track = "model.changed(WebWindow::fullscreen())"]
                                    set_icon_name: if model.fullscreen {
                                        "arrows-pointing-inward"
                                    } else {
                                        "arrows-pointing-outward"
                                    },
                                    set_tooltip_text: Some("Toggle fullscreen"),
                                    connect_clicked => WebWindowInput::ToggleFullscreen,
                                }
                            }
                        },
                    },

                    #[name(show_toolbars_box)]
                    add_overlay = &gtk::Box {
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Start,
                        set_height_request: 10,
                    },

                    gtk::GraphicsOffload {
                    #[name(web_view)]
                    webkit6::WebView {
                        load_uri: &model.url,
                        set_vexpand: true,
                        set_background_color: &match_style_with_rgb(relm4::main_adw_application()),
                        connect_load_changed[sender] => move |this_webview, _load_event| {
                            let url = match this_webview.uri() {
                                Some(url) => url,
                                None => GString::new()
                            };
                            sender.input(WebWindowInput::NavigationHistoryChanged(this_webview.can_go_back(), this_webview.can_go_forward()));
                            sender.input(WebWindowInput::UrlChanged(url.to_string()))
                        },
                        connect_title_notify[sender] => move |this_webview| {
                            let title = this_webview.title().map(|title| ToString::to_string(&title));
                            let url = match this_webview.uri() {
                                Some(url) => url,
                                None => GString::new()
                            };
                            sender.input(WebWindowInput::TitleChanged(match title.clone() {
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
                        // connect_notify: (Some("url"), clone!(@strong sender => move |this_webview, _| {
                        //     sender.input(WebWindowInput::UrlChanged(match this_webview.uri() {Some(url)=> url.to_string(), None=>"".to_string()}))
                        // }))
                        // connect_context_menu[sender] => move |_this_webview, context_menu, context| {
                        //     if context.context_is_link() {
                        //         let link = context.link_uri();
                        //         if let Some(link) = link {
                        //             let link_string = link.to_string();
                        //             // context_menu.prepend(&webkit6::ContextMenuItem::from_gaction(peek_action.gio_action(), "Peek", Some(&Variant::from_data::<String, String>(link_string)))); // TODO: DOES NOT WORK
                        //         }
                        //     }
                        //     false
                        // }
                    },
                    }
                }
            },

            connect_close_request[sender] => move |_| {
                sender.output(WebWindowOutput::Close).expect("Could not send output WebWindowOutput::Close");
                gtk::glib::Propagation::Stop
            } ,

            present: (),
        }
    }

    menu! {
        screenshot_menu: {
            "Take screenshot of full page" => FullPageScreenshotAction,
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
        let mut model = WebWindow {
            url: init.0.clone(),
            screenshot_flash_box,
            can_go_back: false,
            can_go_forward: false,
            show_headerbar: false,
            title: init.0,
            in_title_edit_mode: false,
            fullscreen: false,
            can_hide_headerbar: Inhibited::new(),
            pin_headerbar: false,
            web_view: None,
            toast_overlay: None,
            tracker: 0,
        };
        let widgets = view_output!();
        model.set_web_view(Some(widgets.web_view.clone()));
        model.set_toast_overlay(Some(widgets.toast_overlay.clone()));
        let fullpage_screenshot_action: RelmAction<FullPageScreenshotAction> = {
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(WebWindowInput::Screenshot(false, webkit6::SnapshotRegion::FullDocument));
            }))
        };
        let peek_action: RelmAction<PeekAction> = RelmAction::new_stateful_with_target_value(
            &(),
            clone!(@strong sender => move |_, _, url| {
                sender.input(WebWindowInput::Peek(url));
            }),
        );
        let mut webwindow_action_group: RelmActionGroup<WebWindowActionGroup> =
            RelmActionGroup::new();
        webwindow_action_group.add_action(fullpage_screenshot_action);
        // webwindow_action_group.add_action(peek_action);
        webwindow_action_group.register_for_widget(root.clone());

        widgets.padlock_image.set_icon_name(
            if model.url.starts_with("https://") || model.url.starts_with("webkit://") {
                Some("padlock2")
            } else if model.url.starts_with("http://") {
                Some("padlock2-open")
            } else {
                None
            },
        );
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
            sender.input(WebWindowInput::BeginHideHeaderBarTimeout);
        }));
        widgets
            .headerbar
            .add_controller(hide_toolbars_event_controller);

        // Set settings for the WebView
        if let Some(web_view_settings) =
            webkit6::prelude::WebViewExt::settings(&widgets.web_view.clone())
        {
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
                download_object.connect_created_destination(clone!(@strong toast_overlay, @strong did_download_fail, @strong sender => move |this_download_object, destination| {
                    let destination_string = destination.to_string();
                    this_download_object.connect_finished(clone!(@strong toast_overlay, @strong did_download_fail, @strong destination_string => move |this_download_object| {
                        if (*did_download_fail).load(std::sync::atomic::Ordering::Relaxed) {return;}
                        let toast = adw::Toast::new("File saved to Downloads folder");
                        toast.set_button_label(Some("Open"));
                        let toast_overlay_clone = toast_overlay.clone();
                        let destination_string_clone = destination_string.clone();
                        toast.connect_button_clicked(move |_| {
                            attempt(|| {
                                let document = Document::from_path(destination_string_clone.clone(), "download_file", Create::No)?;
                                document.launch_with_default_app()?;
                                Ok(())
                            }).catch(|error| toast_overlay_clone.add_toast(adw::Toast::new(format!("{error}").as_str())));
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
            with(
                &[Document::at(
                    Project(Data(&[]).with_id("com", "github.kdwk", "Spidey")),
                    "cookies.sqlite",
                    Create::No,
                )],
                |d| {
                    attempt(|| {
                        session.cookie_manager()?.set_persistent_storage(
                            &d["cookies.sqlite"].path(),
                            webkit6::CookiePersistentStorage::Sqlite,
                        );
                        Some(())
                    })
                },
            );
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
        attempt(|| {
            match message {
                WebWindowInput::Back => self.web_view.clone()?.go_back(),
                WebWindowInput::Forward => self.web_view.clone()?.go_forward(),
                WebWindowInput::Refresh => self.web_view.clone()?.reload(),
                WebWindowInput::CopyLink => {
                    let clipboard = widgets.web_view.clipboard();
                    attempt(|| {
                        clipboard.set_content(Some(&ContentProvider::for_value(
                            &gtk::glib::Value::from(if let Some(uri) = widgets.web_view.uri() {
                                uri.to_string()
                            } else {
                                String::from("")
                            }),
                        )))?;
                        sender.input(WebWindowInput::ShowToast("Copied link to clipboard".to_string()));
                        Ok(())
                    }).catch(|_| eprintln!("Could not copy link to clipboard"));
                }
                WebWindowInput::LoadUrl(url) => self.web_view.clone()?.load_uri(&url),
                WebWindowInput::CreateSmallWebWindow(new_webview) => {
                    let smallwebwindow_width = widgets.web_window.width() - 100;
                    let smallwebwindow_height = widgets.web_window.height() - 100;
                    let smallwebwindow = SmallWebWindow::builder()
                        .launch((new_webview, (smallwebwindow_width, smallwebwindow_height)))
                        .detach();
                    let small_web_window_widget = smallwebwindow.widgets().small_web_window.clone();
                    smallwebwindow.model().web_view.connect_title_notify(
                        clone!(@strong small_web_window_widget => move |this_webview| {
                            let title = this_webview
                                .title()
                                .map(|title| ToString::to_string(&title));
                            small_web_window_widget
                                .set_title(title.unwrap_or(String::from("")).as_str());
                        }),
                    );
                    smallwebwindow.model().web_view.connect_close(
                        clone!(@strong small_web_window_widget => move |this_webview| {
                            small_web_window_widget.close();
                        }),
                    );
                    small_web_window_widget.present(Some(root));
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
                WebWindowInput::UrlChanged(url) => {
                    self.set_url(url.clone());
                    sender.output(WebWindowOutput::UrlChanged(self.url.clone())).discard();
                }
                WebWindowInput::EnterTitleEditMode => {
                    self.can_hide_headerbar.inhibit();
                    self.set_in_title_edit_mode(true);
                }
                WebWindowInput::LeaveTitleEditMode => {
                    self.can_hide_headerbar.release();
                    self.set_in_title_edit_mode(false);
                    let input = widgets.title_edit_entry_buffer.text().to_string();
                    if input == "" {
                        return Some(());
                    }
                    let url = match process_url(input.clone()) {
                        Some(url) => url,
                        None => return Some(()),
                    };
                    if url != self.url {
                        self.set_title(input);
                        self.web_view.clone()?.load_uri(&url);
                    }
                }
                WebWindowInput::NavigationHistoryChanged(can_go_back, can_go_forward) => {
                    self.set_can_go_back(can_go_back);
                    self.set_can_go_forward(can_go_forward);
                    _ = sender.output(WebWindowOutput::LoadChanged(can_go_back, can_go_forward));
                }
                WebWindowInput::InsecureContentDetected => widgets
                    .toast_overlay
                    .add_toast(adw::Toast::new("This page is insecure")),
                WebWindowInput::Screenshot(need_return_main_app, snapshot_region) => {
                    widgets.web_view.snapshot(
                        snapshot_region,
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
                WebWindowInput::BeginHideHeaderBarTimeout => {
                    relm4::spawn_local(clone!(@strong sender => async move {
                        _ = tokio::time::sleep(Duration::from_millis(100));
                        sender.input(WebWindowInput::HideHeaderBar);
                    }));
                }
                WebWindowInput::HideHeaderBar => {
                    if self.can_hide_headerbar.is_clear() {
                        self.set_show_headerbar(false);
                    }
                }
                WebWindowInput::ToggleFullscreen => {
                    if widgets.web_window.is_fullscreen() {
                        widgets.web_window.unfullscreen();
                        self.set_fullscreen(false);
                    } else {
                        widgets.web_window.fullscreen();
                        self.set_fullscreen(true);
                    }
                }
                WebWindowInput::InhibitHideHeaderBar => self.can_hide_headerbar.inhibit(),
                WebWindowInput::ReleaseHideHeaderBar => self.can_hide_headerbar.release(),
                WebWindowInput::TogglePinHeaderBar => {
                    self.set_pin_headerbar(!self.pin_headerbar);
                    if self.pin_headerbar {
                        sender.input(WebWindowInput::InhibitHideHeaderBar);
                    } else {
                        sender.input(WebWindowInput::ReleaseHideHeaderBar);
                    }
                }
                WebWindowInput::Peek(url) => {
                    println!("{url}");
                }
                WebWindowInput::ShowToast(message) => self.toast_overlay.clone()?.add_toast(Toast::new(&message))
            };
        self.update_view(widgets, sender_clone);
        Some(())
    }).catch(|error| eprintln!("{error}"));
    }
}
