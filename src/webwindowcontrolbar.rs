#![allow(unused_imports)]
#![allow(unused_variables)]
use ashpd::desktop::{clipboard::Clipboard, Request, Session};
use relm4::{
    actions::{RelmAction, RelmActionGroup},
    gtk::{glib::clone, prelude::*},
    prelude::*,
};
use webkit6::gdk::ContentProvider;
use webkit6::prelude::*;

use crate::app::{process_url, AppInput};
use crate::config::{APP_ID, PROFILE};
use crate::webwindow::*;

pub struct WebWindowControlBar {
    id: DynamicIndex,
    label: String,
    url: String,
    pub webwindow: Controller<WebWindow>,
    web_view_can_go_back: bool,
    web_view_can_go_forward: bool,
    in_title_edit_mode: bool,
    title_edit_textbuffer: gtk::EntryBuffer,
}

pub type WebWindowControlBarInit = (String, Option<webkit6::UserContentFilterStore>);

#[derive(Debug, Clone)]
pub enum WebWindowControlBarInput {
    Back,
    Forward,
    Close,
    Refresh,
    Focus,
    Screenshot,
    ReturnToMainAppWindow,
    RetroactivelyLoadUserContentFilter(webkit6::UserContentFilterStore),
    LoadChanged(bool, bool, String),
    TitleChanged(String, String),
    CopyLink,
    EnterTitleEditMode,
    LeaveTitleEditMode,
}

#[derive(Debug)]
pub enum WebWindowControlBarOutput {
    ReturnToMainAppWindow,
    Remove(DynamicIndex), // pass the id
}

relm4::new_action_group!(WebWindowControlBarActionGroup, "webwindowcontrolbar");
relm4::new_stateless_action!(BackAction, WebWindowControlBarActionGroup, "back");
relm4::new_stateless_action!(ForwardAction, WebWindowControlBarActionGroup, "forward");
relm4::new_stateless_action!(RefreshAction, WebWindowControlBarActionGroup, "refresh");
relm4::new_stateless_action!(
    ScreenshotAction,
    WebWindowControlBarActionGroup,
    "screenshot"
);
relm4::new_stateless_action!(FocusAction, WebWindowControlBarActionGroup, "focus");
relm4::new_stateless_action!(CopyLinkAction, WebWindowControlBarActionGroup, "copy-link");
#[relm4::factory(pub)]
impl FactoryComponent for WebWindowControlBar {
    type Init = WebWindowControlBarInit;
    type Input = WebWindowControlBarInput;
    type Output = WebWindowControlBarOutput;
    type CommandOutput = ();
    type Widgets = WebWindowControlBarWidgets;
    type ParentWidget = gtk::Box;

    view! {
        adw::Clamp {
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 0,
                set_margin_all: 5,

                #[name(label)]
                if self.in_title_edit_mode {
                    #[name(title_edit_entry)]
                    gtk::Entry {
                        add_css_class: "circular",
                        set_hexpand: true,
                        set_margin_start: 10,
                        set_buffer: &self.title_edit_textbuffer,
                        set_placeholder_text: Some("Search the web or enter a link"),
                        set_input_purpose: gtk::InputPurpose::Url,
                        set_input_hints: gtk::InputHints::NO_SPELLCHECK,
                        set_icon_from_icon_name: (gtk::EntryIconPosition::Secondary, Some("arrow3-right-symbolic")),
                        set_icon_tooltip_text: (gtk::EntryIconPosition::Secondary, Some("Go")),
                        connect_activate => WebWindowControlBarInput::LeaveTitleEditMode,
                        connect_icon_press[sender] => move |_this_entry, icon_position| {
                            if let gtk::EntryIconPosition::Secondary = icon_position {
                                sender.input(WebWindowControlBarInput::LeaveTitleEditMode);
                            }
                        },
                    }
                } else {
                    #[name(title_button)]
                    gtk::Button {
                        set_hexpand: true,
                        set_can_shrink: true,
                        add_css_class: "flat",
                        add_css_class: "circular",
                        set_tooltip_text: Some("Click to enter link or search"),

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::Start,
                            set_margin_start: 15,
                            #[name(padlock_image)]
                            gtk::Image {
                                #[watch]
                                set_from_icon_name: if self.url.starts_with("https://") || self.url.starts_with("webkit://") {
                                    Some("padlock2")
                                } else if self.url.starts_with("http://") {
                                    Some("padlock2-open")
                                } else {None},
                            },

                            #[name(title_label)]
                            gtk::Label {
                                set_margin_start: 7,
                                #[watch]
                                set_label: self.label.as_str()
                            }
                        },

                        connect_clicked => WebWindowControlBarInput::EnterTitleEditMode,
                    }
                },

                #[name(action_menu_button)]
                gtk::MenuButton{
                    add_css_class: "circular",
                    add_css_class: "flat",
                    set_icon_name: "menu",
                    set_tooltip_text: Some("Actions"),
                    #[wrap(Some)]
                    set_popover = &gtk::PopoverMenu::from_model(Some(&action_menu)),
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
    }

    menu! {
        action_menu: {
            "Back" => BackAction,
            "Forward" => ForwardAction,
            "Refresh" => RefreshAction,
            "Screenshot" => ScreenshotAction,
            "Focus" => FocusAction,
            "Copy Link" => CopyLinkAction,
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: FactorySender<Self>,
    ) {
        match message {
                WebWindowControlBarInput::Close => {
                    self.webwindow.widgets().web_window.destroy();
                    let _ = sender.output(WebWindowControlBarOutput::Remove(self.id.clone()));
                }
                WebWindowControlBarInput::Back => self.webwindow.widgets().web_view.go_back(),
                WebWindowControlBarInput::Forward => self.webwindow.widgets().web_view.go_forward(),
                WebWindowControlBarInput::Refresh => self.webwindow.widgets().web_view.reload(),
                WebWindowControlBarInput::Screenshot => self
                    .webwindow
                    .sender()
                    .send(WebWindowInput::Screenshot(true, webkit6::SnapshotRegion::Visible))
                    .expect("Could not send WebWindowInput::Screenshot to WebWindow"),
                WebWindowControlBarInput::Focus => self.webwindow.widgets().web_window.present(),
                WebWindowControlBarInput::ReturnToMainAppWindow => _ = sender.output(WebWindowControlBarOutput::ReturnToMainAppWindow),
                WebWindowControlBarInput::LoadChanged(can_go_back, can_go_forward, url) => {
                    self.web_view_can_go_back = can_go_back;
                    self.web_view_can_go_forward = can_go_forward;
                    self.url = url;
                }
                WebWindowControlBarInput::EnterTitleEditMode => {
                    self.title_edit_textbuffer.set_text(self.url.clone());
                    self.in_title_edit_mode = true;
                    widgets.title_edit_entry.grab_focus();
                }
                WebWindowControlBarInput::LeaveTitleEditMode => {
                    let input = self.title_edit_textbuffer.text().to_string();
                    self.title_edit_textbuffer.set_text("");
                    self.in_title_edit_mode = false;
                    let url = match process_url(input.clone()) {
                        Some(url) => url,
                        None => self.url.clone(),
                    };
                    if url != self.url {
                        self.label = input;
                        self.url = url.clone();
                    }
                    _ = self.webwindow.sender().send(WebWindowInput::SetUrl(url));
                }
                WebWindowControlBarInput::TitleChanged(title, url) => {self.label = title; self.url = url;},
                WebWindowControlBarInput::RetroactivelyLoadUserContentFilter(
                    user_content_filter_store,
                ) => self
                    .webwindow
                    .sender()
                    .send(WebWindowInput::RetroactivelyLoadUserContentFilter(
                        user_content_filter_store,
                    ))
                    .expect("Could not send WebWindowInput::RetroactivelyLoadUserContentFilter to WebWindow"),
                WebWindowControlBarInput::ReturnToMainAppWindow => sender.output(WebWindowControlBarOutput::ReturnToMainAppWindow).expect("Could not send output WebWindowControlBarOutput::ReturnToMainAppWindow"),
                WebWindowControlBarInput::CopyLink => _ = self.webwindow.sender().send(WebWindowInput::CopyLink)
        }
        self.update_view(widgets, sender);
    }

    fn init_model(init: Self::Init, index: &Self::Index, sender: FactorySender<Self>) -> Self {
        let new_webwindow =
            WebWindow::builder()
                .launch(init.clone())
                .forward(sender.input_sender(), |message| match message {
                    WebWindowOutput::LoadChanged(can_go_back, can_go_forward, url) => {
                        WebWindowControlBarInput::LoadChanged(can_go_back, can_go_forward, url)
                    }
                    WebWindowOutput::TitleChanged(title, url) => {
                        WebWindowControlBarInput::TitleChanged(title, url)
                    }
                    WebWindowOutput::Close => WebWindowControlBarInput::Close,
                    WebWindowOutput::ReturnToMainAppWindow => {
                        WebWindowControlBarInput::ReturnToMainAppWindow
                    }
                });
        Self {
            id: index.clone(),
            label: init.0.clone(),
            url: init.0,
            webwindow: new_webwindow,
            web_view_can_go_back: false,
            web_view_can_go_forward: false,
            in_title_edit_mode: false,
            title_edit_textbuffer: gtk::EntryBuffer::new(Some("")),
        }
    }

    fn init_widgets(
        &mut self,
        index: &Self::Index,
        root: Self::Root,
        returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let back_action: RelmAction<BackAction> = {
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(WebWindowControlBarInput::Back);
            }))
        };
        let forward_action: RelmAction<ForwardAction> = {
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(WebWindowControlBarInput::Forward);
            }))
        };
        let refresh_action: RelmAction<RefreshAction> = {
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(WebWindowControlBarInput::Refresh);
            }))
        };
        let screenshot_action: RelmAction<ScreenshotAction> = {
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(WebWindowControlBarInput::Screenshot);
            }))
        };
        let focus_action: RelmAction<FocusAction> = {
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(WebWindowControlBarInput::Focus);
            }))
        };
        let copy_link_action: RelmAction<CopyLinkAction> = {
            RelmAction::new_stateless(clone!(@strong sender => move |_| {
                sender.input(WebWindowControlBarInput::CopyLink);
            }))
        };

        let mut webwindow_control_bar_action_group: RelmActionGroup<
            WebWindowControlBarActionGroup,
        > = RelmActionGroup::new();

        webwindow_control_bar_action_group.add_action(back_action);
        webwindow_control_bar_action_group.add_action(forward_action);
        webwindow_control_bar_action_group.add_action(refresh_action);
        webwindow_control_bar_action_group.add_action(screenshot_action);
        webwindow_control_bar_action_group.add_action(focus_action);
        webwindow_control_bar_action_group.add_action(copy_link_action);
        webwindow_control_bar_action_group.register_for_widget(root.clone());

        let widgets = view_output!();

        Self::Widgets {
            label: widgets.label,
            action_menu_button: widgets.action_menu_button,
            close_btn: widgets.close_btn,
            title_edit_entry: widgets.title_edit_entry,
            title_button: widgets.title_button,
            padlock_image: widgets.padlock_image,
            title_label: widgets.title_label,
        }
    }
}
