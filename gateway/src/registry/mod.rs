//! Redis-backed tunnel registry: tunnel name -> client address (host:port).
//! Shared across gateway instances so any instance can resolve and connect to the client.

use redis::AsyncCommands;

const KEY_PREFIX: &str = "tunex:tunnel:";
const DEFAULT_TTL_SECS: u64 = 60;

/// Redis-backed registry of active tunnels by name.
#[derive(Clone)]
pub struct Registry {
    redis: redis::aio::ConnectionManager,
    ttl_secs: u64,
}

impl Registry {
    pub fn new(redis: redis::aio::ConnectionManager, ttl_secs: Option<u64>) -> Self {
        Self {
            redis,
            ttl_secs: ttl_secs.unwrap_or(DEFAULT_TTL_SECS),
        }
    }

    fn key(name: &str) -> String {
        format!("{}{}", KEY_PREFIX, name)
    }

    /// Register a tunnel: store client address in Redis with TTL.
    pub async fn register(&self, name: String, address: String) -> Result<(), redis::RedisError> {
        let key = Self::key(&name);
        let mut conn = self.redis.clone();
        conn.set_ex(key, address, self.ttl_secs).await
    }

    /// Look up client address for a tunnel. Returns None if not found or expired.
    pub async fn get(&self, name: &str) -> Result<Option<String>, redis::RedisError> {
        let key = Self::key(name);
        let mut conn = self.redis.clone();
        conn.get(key).await
    }

    /// Remove a tunnel from the registry (optional; TTL will expire it if client disappears).
    pub async fn unregister(&self, name: &str) -> Result<(), redis::RedisError> {
        let key = Self::key(name);
        let mut conn = self.redis.clone();
        conn.del(key).await
    }
}
