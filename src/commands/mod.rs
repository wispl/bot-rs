use crate::{Data, Error};

mod admin;
mod music;
mod others;

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    music::commands()
        .into_iter()
        .chain(others::commands())
        .chain(admin::commands())
        .collect()
}
