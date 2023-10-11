#![allow(unused_imports)]
#![allow(unused_variables)]
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;
use webkit6::prelude::*;

use crate::app::AppInput;
use crate::config::{APP_ID, PROFILE};
use crate::webwindow::*;

pub struct WebWindowControlBar {
    id: DynamicIndex,
    label: String,
    webwindow: Controller<WebWindow>,
    web_view_can_go_back: bool,
    web_view_can_go_forward: bool,
    screenshot_flash_box: gtk::Box,
}

pub type WebWindowControlBarInit = (String, Option<webkit6::UserContentFilterStore>);

#[derive(Debug)]
pub enum WebWindowControlBarInput {
    Back,
    Forward,
    Close,
    Refresh,
    Focus,
    Screenshot,
    BeginScreenshotFlash,
    ScreenshotFlashFinished,
    ReturnToMainAppWindow,
    LoadChanged((bool, bool)),
    TitleChanged(String),
}

#[derive(Debug)]
pub enum WebWindowControlBarOutput {
    ReturnToMainAppWindow,
    Remove(DynamicIndex), // pass the id
}

#[relm4::factory(pub)]
impl FactoryComponent for WebWindowControlBar {
    type Init = WebWindowControlBarInit;
    type Input = WebWindowControlBarInput;
    type Output = WebWindowControlBarOutput;
    type CommandOutput = ();
    type Widgets = WebWindowControlBarWidgets;
    type ParentWidget = gtk::Box;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 0,
            set_margin_all: 5,

            #[name(back_btn)]
            gtk::Button {
                add_css_class: "circular",
                add_css_class: "flat",
                set_icon_name: "left",
                set_tooltip_text: Some("Back"),
                #[watch]
                set_sensitive: self.web_view_can_go_back,
                connect_clicked => WebWindowControlBarInput::Back,
            },

            #[name(forward_btn)]
            gtk::Button {
                add_css_class: "circular",
                add_css_class: "flat",
                set_icon_name: "right",
                set_tooltip_text: Some("Forward"),
                #[watch]
                set_sensitive: self.web_view_can_go_forward,
                connect_clicked => WebWindowControlBarInput::Forward,
            },

            #[name(refresh_btn)]
            gtk::Button {
                add_css_class: "circular",
                add_css_class: "flat",
                set_icon_name: "refresh",
                set_tooltip_text: Some("Refresh"),
                connect_clicked => WebWindowControlBarInput::Refresh,
            },

            #[name(label)]
            gtk::Label {
                set_hexpand: true,
                set_halign: gtk::Align::Start,
                set_margin_start: 5,
                set_margin_end: 5,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                #[watch]
                set_label: &self.label,
            },

            #[name(screenshot_btn)]
            gtk::Button {
                add_css_class: "circular",
                add_css_class: "flat",
                add_css_class: "toolbar-button",
                set_icon_name: "screenshooter",
                set_tooltip_text: Some("Screenshot"),
                connect_clicked => WebWindowControlBarInput::Screenshot,
            },

            #[name(focus_btn)]
            gtk::Button {
                add_css_class: "circular",
                add_css_class: "flat",
                add_css_class: "toolbar-button",
                set_icon_name: "multitasking-windows",
                set_tooltip_text: Some("Focus"),
                connect_clicked => WebWindowControlBarInput::Focus,
            },

            #[name(close_btn)]
            gtk::Button {
                add_css_class: "circular",
                add_css_class: "flat",
                add_css_class: "toolbar-button",
                set_icon_name: "cross",
                set_tooltip_text: Some("Close"),
                connect_clicked => WebWindowControlBarInput::Close,
            }
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            WebWindowControlBarInput::Close => {
                self.webwindow.widgets().web_window.destroy();
                sender.output(WebWindowControlBarOutput::Remove(self.id.clone()));
            }
            WebWindowControlBarInput::Back => self.webwindow.widgets().web_view.go_back(),
            WebWindowControlBarInput::Forward => self.webwindow.widgets().web_view.go_forward(),
            WebWindowControlBarInput::Refresh => self.webwindow.widgets().web_view.reload(),
            WebWindowControlBarInput::Screenshot => {
                let web_window_widget_clone = self.webwindow.widgets().web_window.clone();
                let toast_overlay_widget_clone = self.webwindow.widgets().toast_overlay.clone();
                self.webwindow.widgets().web_view.snapshot(
                    webkit6::SnapshotRegion::Visible,
                    webkit6::SnapshotOptions::INCLUDE_SELECTION_HIGHLIGHTING,
                    gtk::gio::Cancellable::NONE,
                    move |snapshot_result| match snapshot_result {
                        Ok(texture) => {
                            // Present the WebWindow to show off the beautiful animation that took an afternoon to figure out
                            web_window_widget_clone.present();
                            let sender_clone = sender.clone();
                            // Animation is 800ms, so after that and 30ms of leeway send the ScreenShotFlashFinished input to remove the effect box
                            thread::spawn(move || {
                                thread::sleep(Duration::from_millis(300));
                                sender_clone.input(WebWindowControlBarInput::BeginScreenshotFlash);
                                thread::sleep(Duration::from_millis(830));
                                sender_clone
                                    .input(WebWindowControlBarInput::ScreenshotFlashFinished);
                                thread::sleep(Duration::from_millis(350));
                                sender_clone.input(WebWindowControlBarInput::ReturnToMainAppWindow);
                            });
                            if let Some(dir) = directories::UserDirs::new() {
                                // Create the ~/Pictures/Screenshots folder if it doesn't exist
                                create_dir_all(Path::new(
                                    &dir.picture_dir()
                                        .unwrap()
                                        .join("Screenshots")
                                        .into_os_string()
                                        .into_string()
                                        .unwrap(),
                                ))
                                .unwrap();
                                // Function to get the screenshot save path with the suffix as a String
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
                                let texture_png_bytes = texture.save_to_png_bytes();
                                File::create(Path::new(&screenshot_save_path_final)).unwrap();
                                let mut screenshot_file = OpenOptions::new()
                                    .write(true)
                                    .open(Path::new(&screenshot_save_path_final))
                                    .unwrap();
                                // Actually write the PNG bytes to the file
                                screenshot_file.write_all(&texture_png_bytes).unwrap();
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
            WebWindowControlBarInput::BeginScreenshotFlash => {
                // Add the effect box with CSS class "screenshot-in-progress" to the main overlay of the WebWindow, CSS animation "screenshot-flash" automatically begins
                self.webwindow
                    .widgets()
                    .main_overlay
                    .add_overlay(&self.screenshot_flash_box);
            }
            WebWindowControlBarInput::ScreenshotFlashFinished => {
                self.webwindow
                    .widgets()
                    .main_overlay
                    .remove_overlay(&self.screenshot_flash_box);
            }
            WebWindowControlBarInput::ReturnToMainAppWindow => {
                sender.output(WebWindowControlBarOutput::ReturnToMainAppWindow)
            }
            WebWindowControlBarInput::Focus => self.webwindow.widgets().web_window.present(),
            WebWindowControlBarInput::LoadChanged((can_go_back, can_go_forward)) => {
                self.web_view_can_go_back = can_go_back;
                self.web_view_can_go_forward = can_go_forward;
            }
            WebWindowControlBarInput::TitleChanged(title) => self.label = title,
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let new_webwindow =
            WebWindow::builder()
                .launch(init.clone())
                .forward(sender.input_sender(), |message| match message {
                    WebWindowOutput::LoadChanged((can_go_back, can_go_forward)) => {
                        WebWindowControlBarInput::LoadChanged((can_go_back, can_go_forward))
                    }
                    WebWindowOutput::TitleChanged(title) => {
                        WebWindowControlBarInput::TitleChanged(title)
                    }
                    WebWindowOutput::Close => WebWindowControlBarInput::Close,
                });
        let screenshot_effect_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        screenshot_effect_box.set_valign(gtk::Align::Fill);
        screenshot_effect_box.set_halign(gtk::Align::Fill);
        screenshot_effect_box.add_css_class("screenshot-in-progress");
        Self {
            id: index.clone(),
            label: init.0,
            webwindow: new_webwindow,
            web_view_can_go_back: false,
            web_view_can_go_forward: false,
            screenshot_flash_box: screenshot_effect_box,
        }
    }
}
