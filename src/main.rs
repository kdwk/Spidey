#[rustfmt::skip]
mod config;
mod app;
mod setup;

use relm4::{
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    gtk, main_application, RelmApp,
};
use relm4::gtk::{prelude::*, Box, Label, Button, Orientation, Align, Video, Entry, InputHints, InputPurpose, EntryBuffer, ScrolledWindow};
use relm4::adw::{prelude::*, Window, HeaderBar, MessageDialog, ViewStack, StatusPage};
use relm4::{prelude::{*, FactoryComponent}, factory::FactoryVecDeque};
use relm4_macros::*;

use app::App;
use setup::setup;

relm4::new_action_group!(AppActionGroup, "app");
relm4::new_stateless_action!(QuitAction, AppActionGroup, "quit");

fn main() {
    // Enable logging
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(tracing::Level::INFO)
        .init();

    setup();

    let app = main_application();
    app.set_resource_base_path(Some("/com/github/kdwk/Spidey/"));

    let mut actions = RelmActionGroup::<AppActionGroup>::new();

    let quit_action = {
        let app = app.clone();
        RelmAction::<QuitAction>::new_stateless(move |_| {
            app.quit();
        })
    };
    actions.add_action(quit_action);
    actions.register_for_main_application();

    app.set_accelerators_for_action::<QuitAction>(&["<Control>q"]);

    let app = RelmApp::from_app(app);

    relm4_icons::initialize_icons();

    app.run::<App>(());
}
