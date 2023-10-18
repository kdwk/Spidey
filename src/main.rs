#[rustfmt::skip]
mod config;
mod app;
mod setup;
mod smallwebwindow;
mod webwindow;
mod webwindowcontrolbar;

use relm4::{
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    gtk::prelude::*,
    main_application, RelmApp, SharedState,
};

use app::App;
use setup::setup;

static IS_MAIN_WINDOW_OPEN: SharedState<bool> = SharedState::new();

relm4::new_action_group!(AppActionGroup, "app");
relm4::new_stateless_action!(QuitAction, AppActionGroup, "quit");
relm4::new_stateless_action!(PresentMainWindow, AppActionGroup, "present-main-window");

fn main() {
    // Enable logging
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(tracing::Level::INFO)
        .init();

    setup();

    // let mut is_main_window_open = false;
    // let (sender, receiver) = relm4::channel();
    // IS_MAIN_WINDOW_OPEN.subscribe(&sender, move |&value| {
    //     is_main_window_open = value;
    // });

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

    app.set_accelerators_for_action::<QuitAction>(&["<primary>q"]);

    let app = RelmApp::from_app(app);

    relm4_icons::initialize_icons();

    app.run::<App>(());
}
