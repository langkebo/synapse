use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationBlacklistEntry {
    pub server_name: String,
    pub reason: String,
    pub added_at: i64,
    pub added_by: String,
    pub expires_at: Option<i64>,
    pub is_permanent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationWhitelistEntry {
    pub server_name: String,
    pub reason: String,
    pub added_at: i64,
    pub added_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationAccess {
    Allowed,
    Blocked,
    NotWhitelisted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPolicy {
    pub whitelist_enabled: bool,
    pub blacklist_enabled: bool,
    pub default_allow: bool,
}

impl Default for FederationPolicy {
    fn default() -> Self {
        Self {
            whitelist_enabled: false,
            blacklist_enabled: true,
            default_allow: true,
        }
    }
}

pub struct FederationAccessControl {
    blacklist: Arc<RwLock<HashSet<String>>>,
    whitelist: Arc<RwLock<HashSet<String>>>,
    blacklist_entries: Arc<RwLock<std::collections::HashMap<String, FederationBlacklistEntry>>>,
    whitelist_entries: Arc<RwLock<std::collections::HashMap<String, FederationWhitelistEntry>>>,
    policy: FederationPolicy,
}

impl FederationAccessControl {
    pub fn new(policy: FederationPolicy) -> Self {
        Self {
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            whitelist: Arc::new(RwLock::new(HashSet::new())),
            blacklist_entries: Arc::new(RwLock::new(std::collections::HashMap::new())),
            whitelist_entries: Arc::new(RwLock::new(std::collections::HashMap::new())),
            policy,
        }
    }

    pub async fn add_to_blacklist(
        &self,
        server_name: &str,
        reason: &str,
        added_by: &str,
        expires_at: Option<i64>,
    ) {
        let entry = FederationBlacklistEntry {
            server_name: server_name.to_string(),
            reason: reason.to_string(),
            added_at: Utc::now().timestamp_millis(),
            added_by: added_by.to_string(),
            expires_at,
            is_permanent: expires_at.is_none(),
        };

        self.blacklist.write().await.insert(server_name.to_string());
        self.blacklist_entries.write().await.insert(server_name.to_string(), entry);

        info!(
            server_name = %server_name,
            reason = %reason,
            added_by = %added_by,
            "Server added to federation blacklist"
        );
    }

    pub async fn remove_from_blacklist(&self, server_name: &str) -> bool {
        let removed = self.blacklist.write().await.remove(server_name);
        self.blacklist_entries.write().await.remove(server_name);

        if removed {
            info!(server_name = %server_name, "Server removed from federation blacklist");
        }

        removed
    }

    pub async fn add_to_whitelist(
        &self,
        server_name: &str,
        reason: &str,
        added_by: &str,
    ) {
        let entry = FederationWhitelistEntry {
            server_name: server_name.to_string(),
            reason: reason.to_string(),
            added_at: Utc::now().timestamp_millis(),
            added_by: added_by.to_string(),
        };

        self.whitelist.write().await.insert(server_name.to_string());
        self.whitelist_entries.write().await.insert(server_name.to_string(), entry);

        info!(
            server_name = %server_name,
            reason = %reason,
            added_by = %added_by,
            "Server added to federation whitelist"
        );
    }

    pub async fn remove_from_whitelist(&self, server_name: &str) -> bool {
        let removed = self.whitelist.write().await.remove(server_name);
        self.whitelist_entries.write().await.remove(server_name);

        if removed {
            info!(server_name = %server_name, "Server removed from federation whitelist");
        }

        removed
    }

    pub async fn check_access(&self, server_name: &str) -> FederationAccess {
        if self.policy.blacklist_enabled
            && self.blacklist.read().await.contains(server_name) {
                debug!(server_name = %server_name, "Server is blacklisted");
                return FederationAccess::Blocked;
            }

        if self.policy.whitelist_enabled
            && !self.whitelist.read().await.contains(server_name) {
                debug!(server_name = %server_name, "Server not in whitelist");
                return FederationAccess::NotWhitelisted;
            }

        FederationAccess::Allowed
    }

    pub async fn is_allowed(&self, server_name: &str) -> bool {
        matches!(self.check_access(server_name).await, FederationAccess::Allowed)
    }

    pub async fn get_blacklist_entries(&self) -> Vec<FederationBlacklistEntry> {
        self.blacklist_entries.read().await.values().cloned().collect()
    }

    pub async fn get_whitelist_entries(&self) -> Vec<FederationWhitelistEntry> {
        self.whitelist_entries.read().await.values().cloned().collect()
    }

    pub async fn get_blacklist_entry(&self, server_name: &str) -> Option<FederationBlacklistEntry> {
        self.blacklist_entries.read().await.get(server_name).cloned()
    }

    pub async fn cleanup_expired_entries(&self) -> usize {
        let now = Utc::now().timestamp_millis();
        let mut blacklist = self.blacklist.write().await;
        let mut entries = self.blacklist_entries.write().await;

        let expired: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| {
                !entry.is_permanent && entry.expires_at.map(|exp| exp < now).unwrap_or(false)
            })
            .map(|(name, _)| name.clone())
            .collect();

        let count = expired.len();
        for server_name in expired {
            blacklist.remove(&server_name);
            entries.remove(&server_name);
            info!(server_name = %server_name, "Expired blacklist entry removed");
        }

        count
    }

    pub async fn batch_add_to_blacklist(
        &self,
        servers: Vec<(String, String)>,
        added_by: &str,
    ) {
        for (server_name, reason) in servers {
            self.add_to_blacklist(&server_name, &reason, added_by, None).await;
        }
    }

    pub async fn batch_add_to_whitelist(
        &self,
        servers: Vec<(String, String)>,
        added_by: &str,
    ) {
        for (server_name, reason) in servers {
            self.add_to_whitelist(&server_name, &reason, added_by).await;
        }
    }

    pub fn policy(&self) -> &FederationPolicy {
        &self.policy
    }

    pub async fn set_policy(&mut self, policy: FederationPolicy) {
        self.policy = policy;
        info!(
            whitelist_enabled = self.policy.whitelist_enabled,
            blacklist_enabled = self.policy.blacklist_enabled,
            default_allow = self.policy.default_allow,
            "Federation policy updated"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blacklist_operations() {
        let acl = FederationAccessControl::new(FederationPolicy::default());

        acl.add_to_blacklist("evil.com", "Spam server", "@admin:example.com", None).await;

        assert!(!acl.is_allowed("evil.com").await);
        assert!(acl.is_allowed("good.com").await);

        assert!(acl.remove_from_blacklist("evil.com").await);
        assert!(acl.is_allowed("evil.com").await);
    }

    #[tokio::test]
    async fn test_whitelist_operations() {
        let policy = FederationPolicy {
            whitelist_enabled: true,
            blacklist_enabled: false,
            default_allow: false,
        };
        let acl = FederationAccessControl::new(policy);

        acl.add_to_whitelist("trusted.com", "Partner server", "@admin:example.com").await;

        assert!(acl.is_allowed("trusted.com").await);
        assert!(!acl.is_allowed("untrusted.com").await);
    }

    #[tokio::test]
    async fn test_blacklist_takes_precedence() {
        let policy = FederationPolicy {
            whitelist_enabled: true,
            blacklist_enabled: true,
            default_allow: true,
        };
        let acl = FederationAccessControl::new(policy);

        acl.add_to_whitelist("server.com", "Trusted", "@admin:example.com").await;
        acl.add_to_blacklist("server.com", "Later blocked", "@admin:example.com", None).await;

        assert!(!acl.is_allowed("server.com").await);
    }

    #[tokio::test]
    async fn test_expired_entries() {
        let acl = FederationAccessControl::new(FederationPolicy::default());

        let past_time = Utc::now().timestamp_millis() - 1000;
        acl.add_to_blacklist("expired.com", "Test", "@admin:example.com", Some(past_time)).await;
        acl.add_to_blacklist("permanent.com", "Test", "@admin:example.com", None).await;

        let removed = acl.cleanup_expired_entries().await;
        assert_eq!(removed, 1);

        assert!(acl.is_allowed("expired.com").await);
        assert!(!acl.is_allowed("permanent.com").await);
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let acl = FederationAccessControl::new(FederationPolicy::default());

        acl.batch_add_to_blacklist(
            vec![
                ("spam1.com".to_string(), "Spam".to_string()),
                ("spam2.com".to_string(), "Spam".to_string()),
            ],
            "@admin:example.com",
        ).await;

        assert!(!acl.is_allowed("spam1.com").await);
        assert!(!acl.is_allowed("spam2.com").await);

        let entries = acl.get_blacklist_entries().await;
        assert_eq!(entries.len(), 2);
    }
}
