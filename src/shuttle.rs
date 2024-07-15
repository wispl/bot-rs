use shuttle_runtime::{CustomError, Error};
use std::net::SocketAddr;
use serenity::Client;

// Reimplementation of shuttle_runtime for serenity 0.12 next

pub struct SerenityService(pub Client);

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for SerenityService {
    async fn bind(mut self, _addr: SocketAddr) -> Result<(), Error> {
        self.0.start_autosharded().await.map_err(CustomError::new)?;

        Ok(())
    }
}

impl From<Client> for SerenityService {
    fn from(router: Client) -> Self {
        Self(router)
    }
}

pub type ShuttleSerenity = Result<SerenityService, Error>;

