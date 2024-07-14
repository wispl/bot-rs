use crate::{Error, Data};

mod music;
mod others;
mod admin;

pub fn commands() -> Vec<poise::Command<Data, Error>> {
    music::commands().into_iter()
        .chain(others::commands())
        .chain(admin::commands())
        .collect()
}
