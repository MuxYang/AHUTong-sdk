use crate::data::api::client::AHUClient;

pub struct Crawler {
    pub client: AHUClient,
}

impl Crawler {
    pub fn new(client: AHUClient) -> Self {
        Self { client }
    }
}

#[cfg(feature = "domain-account")]
mod account;
#[cfg(feature = "domain-campus-card")]
mod campus_card;
#[cfg(feature = "domain-grades")]
mod grades;
#[cfg(all(not(target_arch = "wasm32"), feature = "domain-guide"))]
mod guide;
#[cfg(feature = "domain-schedule")]
mod schedule;
