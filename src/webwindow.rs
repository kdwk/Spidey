#![allow(unused_imports)]
#![allow(unused_variables)]
use directories;
use std::fs::{create_dir_all, File};

use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use webkit6::prelude::*;
use webkit6_sys::webkit_web_view_get_settings;

use crate::config::{APP_ID, PROFILE};
use crate::smallwebwindow::*;

pub struct WebWindow {
    pub url: String,
}

#[derive(Debug)]
pub enum WebWindowInput {
    CreateSmallWebWindow(webkit6::WebView),
    TitleChanged(String),
    InsecureContentDetected,
}

#[derive(Debug)]
pub enum WebWindowOutput {
    LoadChanged((bool, bool)),
    TitleChanged(String),
    Close,
}

relm4::new_action_group!(WebWindowActionGroup, "win");
relm4::new_stateless_action!(GoBack, WebWindowActionGroup, "go_back");
#[relm4::component(pub)]
impl Component for WebWindow {
    type Init = String;
    type Input = WebWindowInput;
    type Output = WebWindowOutput;
    type CommandOutput = ();

    view! {
        #[name(web_window)]
        adw::Window {
            set_default_height: 1000,
            set_default_width: 1000,

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
        let model = WebWindow { url: init };
        let widgets = view_output!();
        let web_view_settings_option = webkit6::prelude::WebViewExt::settings(&widgets.web_view);
        match web_view_settings_option {
            Some(web_view_settings) => {
                web_view_settings.set_media_playback_requires_user_gesture(true);
                if PROFILE == "Devel" {
                    web_view_settings.set_enable_developer_extras(true);
                }
            }
            None => {}
        }
        let network_session = widgets.web_view.network_session();
        let toast_overlay_widget_clone = widgets.toast_overlay.clone();
        match network_session {
            Some(session) => {
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
                session.set_itp_enabled(true);
                let cookie_manager = session.cookie_manager();
                match cookie_manager {
                    Some(cookie_manager) => {
                        if let Some(dir) =
                            directories::ProjectDirs::from("com", "github.kdwk", "Spidey")
                        {
                            create_dir_all(dir.data_dir()).unwrap();
                            let cookiesdb_file_path = dir.data_dir().join("cookies.sqlite");
                            cookie_manager.set_persistent_storage(
                                &cookiesdb_file_path.into_os_string().into_string().unwrap()[..],
                                webkit6::CookiePersistentStorage::Sqlite,
                            );
                        }
                    }
                    None => {}
                }
            }
            None => {}
        }
        let app = relm4::main_adw_application();
        let mut action_group = RelmActionGroup::<WebWindowActionGroup>::new();
        let web_view_widget_clone = widgets.web_view.clone();
        let go_back: RelmAction<GoBack> = RelmAction::new_stateless(move |_| {
            web_view_widget_clone.go_back();
        });
        app.set_accelerators_for_action::<GoBack>(&["<Alt>leftarrow"]);
        action_group.add_action(go_back);
        action_group.register_for_widget(&widgets.web_window);
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
        }
    }
}
