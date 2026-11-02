//! The backend of a build without the `server` feature: there is no
//! server, so every call that would reach one fails with
//! [`BackendError::Unavailable`] and nothing pretends to be signed in.

use super::*;

/// No server at all: not offline, not signed out, just not there yet.
pub struct NoServer;

fn unavailable<'a, T: Send + 'a>() -> BoxFut<'a, T> {
    Box::pin(async { Err(BackendError::Unavailable) })
}

impl Backend for NoServer {
    fn server(&self) -> Server {
        Server::default()
    }
    fn set_server(&self, _: Server) -> BoxFut<'_, ()> {
        unavailable()
    }
    fn test_connection(&self, _: &Server) -> BoxFut<'_, ConnectionInfo> {
        unavailable()
    }
    fn sign_in(&self, _: SignInMethod) -> BoxFut<'_, Account> {
        unavailable()
    }
    fn sign_out(&self) -> BoxFut<'_, ()> {
        unavailable()
    }
    fn account(&self) -> Option<Account> {
        None
    }
    fn sync_status(&self, _: &CollectionKey) -> SyncStatus {
        SyncStatus::Off
    }
    fn sync(
        &self,
        _: &CollectionKey,
        _: &[SyncFile],
        _: &[SyncDeletion],
    ) -> BoxFut<'_, SyncReport> {
        unavailable()
    }
    fn sync_shared(&self, _: &CollectionKey, _: &str, _: &[SyncFile]) -> BoxFut<'_, SyncReport> {
        unavailable()
    }
    fn share_settings(&self, _: &CollectionKey) -> BoxFut<'_, ShareSettings> {
        unavailable()
    }
    fn update_share(&self, _: &CollectionKey, _: ShareUpdate) -> BoxFut<'_, ShareSettings> {
        unavailable()
    }
    fn invite(&self, _: &CollectionKey, _: &str) -> BoxFut<'_, Person> {
        unavailable()
    }
    fn remove_person(&self, _: &CollectionKey, _: &str) -> BoxFut<'_, ()> {
        unavailable()
    }
    fn resolve_link(&self, _: &ShareLink) -> BoxFut<'_, SharedCollection> {
        unavailable()
    }
    fn fetch_shared(&self, _: &str) -> BoxFut<'_, SharedSnapshot> {
        unavailable()
    }
    fn received(&self) -> Vec<ReceivedShare> {
        Vec::new()
    }
    fn add_received(&self, _: ReceivedShare) -> BoxFut<'_, ()> {
        unavailable()
    }
    fn dev(&self) -> Option<&dyn DevControls> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn no_server_refuses_every_call() {
        let b = NoServer;
        let key = CollectionKey {
            id: "c".into(),
            name: "C".into(),
        };
        fn unavailable<T>(call: BoxFut<'_, T>) -> bool {
            matches!(block_on(call), Err(BackendError::Unavailable))
        }
        assert!(unavailable(b.test_connection(&Server::Cloud)));
        assert!(unavailable(b.set_server(Server::Cloud)));
        assert!(unavailable(b.sign_in(SignInMethod::GitHub)));
        assert!(unavailable(b.sign_out()));
        assert!(unavailable(b.sync(&key, &[], &[])));
        assert!(unavailable(b.sync_shared(&key, "s", &[])));
        assert!(unavailable(b.share_settings(&key)));
        assert!(unavailable(b.update_share(&key, ShareUpdate::default())));
        assert!(unavailable(b.invite(&key, "a@b.c")));
        assert!(unavailable(b.remove_person(&key, "a@b.c")));
        let link = ShareLink::parse("curl-runner://open/acme/api?share=t").unwrap();
        assert!(unavailable(b.resolve_link(&link)));
        assert!(unavailable(b.fetch_shared("s")));
        let share = ReceivedShare {
            id: "s".into(),
            owner_org: "O".into(),
            name: "N".into(),
            added: chrono::Utc::now(),
        };
        assert!(unavailable(b.add_received(share)));
        assert_eq!(b.account(), None);
        assert_eq!(b.sync_status(&key), SyncStatus::Off);
        assert!(b.received().is_empty());
        assert!(b.dev().is_none());
        assert_eq!(
            BackendError::Unavailable.to_string(),
            "This needs the curl-runner server, which is coming soon."
        );
    }
}
