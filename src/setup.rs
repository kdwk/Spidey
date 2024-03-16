use relm4::{gtk, RelmApp};

use gettextrs::{gettext, LocaleCategory};
use gtk::{gio, glib};

use crate::config::{APP_ID, GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};

pub fn setup<Model: std::fmt::Debug>(app: &RelmApp<Model>) {
    // Initialize GTK
    gtk::init().unwrap();

    setup_gettext();

    glib::set_application_name(&gettext("Spidey"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    setup_css(&res, app);

    gtk::Window::set_default_icon_name(APP_ID);
}

fn setup_gettext() {
    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");
}

fn setup_css<Model: std::fmt::Debug>(res: &gio::Resource, app: &RelmApp<Model>) {
    let data = res
        .lookup_data(
            "/com/github/kdwk/Spidey/style.css",
            gio::ResourceLookupFlags::NONE,
        )
        .unwrap();
    app.set_global_css(&glib::GString::from_utf8_checked(data.to_vec()).unwrap());
}
