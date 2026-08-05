use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TunnelKind {
    /// `ssh -L localPort:remoteHost:remotePort` — forward a local port to a
    /// destination reachable from the SSH server.
    Local,
    /// `ssh -R localPort:remoteHost:remotePort` — ask the SSH server to
    /// forward one of *its* ports back to a destination reachable from here.
    Remote,
    /// `ssh -D localPort` — open a local SOCKS5 proxy that tunnels arbitrary
    /// traffic through the SSH connection.
    Dynamic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMethod {
    Password,
    PrivateKey,
    /// Delegate to a running ssh-agent (or Pageant on Windows).
    Agent,
}

/// A saved tunnel configuration. Secrets (`password`, `passphrase`) are never
/// persisted in this struct on disk — see `credential_store.rs` — but the
/// struct carries them in memory while a tunnel is being started.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelProfile {
    pub id: String,
    pub name: String,
    pub kind: TunnelKind,

    /// Free-text grouping label for the UI (e.g. "Producción", "Clientes").
    /// `None`/empty means ungrouped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,

    pub auth_method: AuthMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,

    /// Bind address for `Local`/`Dynamic`; the address the server binds to
    /// for `Remote`. Usually "127.0.0.1" or "0.0.0.0".
    pub local_host: String,
    pub local_port: u16,

    /// Unused for `Dynamic` tunnels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<u16>,
}

/// A reusable "how to reach the SSH host" template — the fields most people
/// repeat across many tunnel profiles that all go through the same bastion.
/// Deliberately excludes secrets (`password`): applying a template only fills
/// in the connection fields, so it doesn't need the keychain at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTemplate {
    pub id: String,
    pub name: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub auth_method: AuthMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatusEvent {
    pub id: String,
    pub status: &'static str, // "connecting" | "connected" | "stopped" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One diagnostic line for a tunnel, streamed to the UI so the user can see
/// what's actually happening for a given connection attempt (SSH connect,
/// auth, channel open, data flow, errors) instead of just "connected"/"error".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelLogEvent {
    pub id: String,
    /// Milliseconds since the Unix epoch, for client-side formatting/sorting.
    pub timestamp_ms: u64,
    pub level: &'static str, // "info" | "warn" | "error"
    pub message: String,
}

/// Periodic live counters for a running tunnel: how many forwarded
/// connections are open right now, and cumulative bytes moved since it was
/// started (reset to 0 each time the tunnel (re)connects).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatsEvent {
    pub id: String,
    pub active_connections: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
}
