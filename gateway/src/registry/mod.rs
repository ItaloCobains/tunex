//! In-memory tunnel registry: name -> channel to forward requests.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Handle to forward a request to a tunnel and receive the response via oneshot.
pub type TunnelHandle = mpsc::Sender<(Vec<u8>, tokio::sync::oneshot::Sender<Vec<u8>>)>;

/// In-memory registry of active tunnels by name.
#[derive(Clone)]
pub struct Registry {
    inner: Arc<RwLock<HashMap<String, TunnelHandle>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, name: String, handle: TunnelHandle) {
        let mut guard = self.inner.write().await;
        guard.insert(name, handle);
    }

    pub async fn unregister(&self, name: &str) {
        let mut guard = self.inner.write().await;
        guard.remove(name);
    }

    pub async fn get(&self, name: &str) -> Option<TunnelHandle> {
        let guard = self.inner.read().await;
        guard.get(name).cloned()
    }
}
