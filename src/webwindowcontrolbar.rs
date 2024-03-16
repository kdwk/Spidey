#![allow(unused_imports)]
#![allow(unused_variables)]
use ashpd::desktop::clipboard::Clipboard;
use ashpd::desktop::{Request, Session};
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::gtk::{glib::clone, prelude::*};
use relm4::prelude::*;
use webkit6::gdk::ContentProvider;
use webkit6::prelude::*;

use crate::app::AppInput;
use crate::config::{APP_ID, PROFILE};
use crate::webwindow::*;

pub struct WebWindowControlBar {
    id: DynamicIndex,
    label: String,
    pub webwindow: Controller<WebWindow>,
    web_view_can_go_back: bool,
    web_view_can_go_forward: bool,
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
    LoadChanged((bool, bool)),
    TitleChanged(String),
    CopyLink,
}

#[derive(Debug)]
pub enum WebWindowControlBarOutput {
    ReturnToMainAppWindow,
    Remove(DynamicIndex), // pass the id
}

relm4::new_action_group!(WebWindowControlBarActionGroup, "webwindowcontrolbar");
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

    menu! {
        action_menu: {
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
                    .send(WebWindowInput::Screenshot)
                    .expect("Could not send WebWindowInput::Screenshot to WebWindow"),
                WebWindowControlBarInput::Focus => self.webwindow.widgets().web_window.present(),
                WebWindowControlBarInput::ReturnToMainAppWindow => {
                    let _ = sender.output(WebWindowControlBarOutput::ReturnToMainAppWindow);
                }
                WebWindowControlBarInput::LoadChanged((can_go_back, can_go_forward)) => {
                    self.web_view_can_go_back = can_go_back;
                    self.web_view_can_go_forward = can_go_forward;
                }
                WebWindowControlBarInput::TitleChanged(title) => self.label = title,
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
                WebWindowControlBarInput::CopyLink => {
                    let clipboard = widgets.action_menu_button.clipboard();
                    if let Err(_) = clipboard.set_content(Some(&ContentProvider::for_value(&gtk::glib::Value::from(if let Some(uri)=self.webwindow.widgets().web_view.uri() {uri.to_string()} else {String::from("")})))) {
                        eprintln!("Could not copy link to clipboard");
                    }
                }
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
                    WebWindowOutput::ReturnToMainAppWindow => {
                        WebWindowControlBarInput::ReturnToMainAppWindow
                    }
                });
        Self {
            id: index.clone(),
            label: init.0,
            webwindow: new_webwindow,
            web_view_can_go_back: false,
            web_view_can_go_forward: false,
        }
    }

    fn init_widgets(
        &mut self,
        index: &Self::Index,
        root: Self::Root,
        returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
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

        webwindow_control_bar_action_group.add_action(screenshot_action);
        webwindow_control_bar_action_group.add_action(focus_action);
        webwindow_control_bar_action_group.add_action(copy_link_action);
        webwindow_control_bar_action_group.register_for_widget(root.clone());

        let widgets = view_output!();

        Self::Widgets {
            back_btn: widgets.back_btn,
            forward_btn: widgets.forward_btn,
            refresh_btn: widgets.refresh_btn,
            label: widgets.label,
            action_menu_button: widgets.action_menu_button,
            close_btn: widgets.close_btn,
        }
    }
}
