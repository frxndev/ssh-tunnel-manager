use std::net::TcpListener;
use std::net::TcpStream;
use std::time::Duration;

use ssh2::Session;

use crate::models::TunnelProfile;
use crate::ssh_tunnel::authenticate;

/// Tries to bind the given host/port briefly, just to see if something else
/// already has it — then immediately releases it. A `true` result is not a
/// hard guarantee (another process could grab the port a moment later), but
/// it catches the common case of "I already have a tunnel/DB client using
/// this port" before the person saves a profile that will fail to start.
#[tauri::command]
pub fn check_port_available(host: String, port: u16) -> bool {
    TcpListener::bind((host.as_str(), port)).is_ok()
}

/// Connects and authenticates to the profile's SSH host — nothing more, no
/// forwarding is started — so the person can validate host/credentials
/// while filling out the form, before saving or starting the real tunnel.
#[tauri::command]
pub fn test_connection(profile: TunnelProfile) -> Result<String, String> {
    test_connection_inner(&profile).map_err(|e| e.to_string())
}

fn test_connection_inner(profile: &TunnelProfile) -> anyhow::Result<String> {
    let tcp = TcpStream::connect((profile.ssh_host.as_str(), profile.ssh_port))?;
    tcp.set_read_timeout(Some(Duration::from_secs(10)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(10)))?;

    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    authenticate(&mut session, profile)?;

    let banner = session.banner().unwrap_or("(sin banner)").trim().to_string();
    Ok(format!(
        "Conectado y autenticado como {} en {}:{}. Banner del servidor: {banner}",
        profile.ssh_user, profile.ssh_host, profile.ssh_port
    ))
}
