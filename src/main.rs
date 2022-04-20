#[macro_use]
extern crate diesel;
extern crate dotenv;

use anyhow::Result;
use relm4::{adw, gtk, MicroComponent, RelmApp};
use widgets::app::AppModel;

use crate::config::set_app;
use crate::widgets::content::ContentModel;

mod widgets;
mod storage;
mod services;
mod schema;
mod models;
mod config;

fn main() -> Result<()> {
    let application = adw::Application::builder()
        .application_id("do.edfloreshz.github")
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    let model = AppModel::new(MicroComponent::new(ContentModel::default(), ()));
    let app = RelmApp::with_app(model, application);
    set_app()?;
    app.run();
    Ok(())
}


