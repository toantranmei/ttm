//! The runner's API client, swappable while jobs run: the runner token lives 12 hours and is renewed by registering
//! again, which revokes the old one.

use std::sync::{Arc, RwLock};
use ttm_api::Client;

#[derive(Debug, Clone)]
pub struct SharedClient(Arc<RwLock<Client>>);

impl SharedClient {
    pub fn new(client: Client) -> Self {
        Self(Arc::new(RwLock::new(client)))
    }

    /// The current client (cheap clone).
    pub fn get(&self) -> Client {
        self.0
            .read()
            .map(|c| c.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    pub fn set(&self, client: Client) {
        match self.0.write() {
            Ok(mut c) => *c = client,
            Err(poisoned) => *poisoned.into_inner() = client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_is_seen_by_clones() {
        let shared = SharedClient::new(Client::new("http://a/api/v1", None).unwrap());
        let other = shared.clone();
        shared.set(Client::new("http://b/api/v1", None).unwrap());
        assert_eq!(other.get().api_base(), "http://b/api/v1");
    }
}
