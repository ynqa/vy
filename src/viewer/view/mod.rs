//! Full-screen page implementations. Shared interfaces are defined in the parent module.

mod browse;
mod config;
mod flatten;
mod help;
mod hstr;
mod preview;

pub(super) use self::{
    browse::BrowseView, config::ConfigView, flatten::FlattenView, help::HelpView, hstr::HstrView,
    preview::PreviewView,
};
