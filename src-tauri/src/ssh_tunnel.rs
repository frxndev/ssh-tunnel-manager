use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ssh2::Session;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::credential_store;
use crate::models::{
    AuthMethod, TunnelKind, TunnelLogEvent, TunnelProfile, TunnelStatsEvent, TunnelStatusEvent,
};

/// How often the stats reporter thread pushes a `tunnel-stats` event.
const STATS_REPORT_INTERVAL: Duration = Duration::from_millis(1500);

/// Mirrors `-o ServerAliveInterval=60`: how often we ask the server for a
/// keepalive reply.
const KEEPALIVE_INTERVAL_SECS: u32 = 60;
/// Mirrors `-o ServerAliveCountMax=3`: how many consecutive missed replies
/// we tolerate before treating the connection as dead and reconnecting.
const KEEPALIVE_MAX_FAILURES: u32 = 3;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn emit_status(app: &AppHandle, id: &str, status: &'static str, error: Option<String>) {
    let _ = app.emit(
        "tunnel-status",
        TunnelStatusEvent {
            id: id.to_string(),
            status,
            error,
        },
    );
}

fn emit_log(app: &AppHandle, id: &str, level: &'static str, message: impl Into<String>) {
    let _ = app.emit(
        "tunnel-log",
        TunnelLogEvent {
            id: id.to_string(),
            timestamp_ms: now_ms(),
            level,
            message: message.into(),
        },
    );
}

fn emit_stats(app: &AppHandle, id: &str, stats: &TunnelStats) {
    let _ = app.emit(
        "tunnel-stats",
        TunnelStatsEvent {
            id: id.to_string(),
            active_connections: stats.active_connections.load(Ordering::Relaxed),
            bytes_in: stats.bytes_in.load(Ordering::Relaxed),
            bytes_out: stats.bytes_out.load(Ordering::Relaxed),
        },
    );
}

/// Live counters for one running tunnel: how many forwarded connections are
/// open right now, and cumulative bytes moved since the tunnel last
/// (re)connected. `bytes_in` is data received from the SSH destination and
/// relayed to the local client; `bytes_out` is the reverse.
#[derive(Default)]
struct TunnelStats {
    active_connections: AtomicU64,
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
}

impl TunnelStats {
    fn reset(&self) {
        self.active_connections.store(0, Ordering::Relaxed);
        self.bytes_in.store(0, Ordering::Relaxed);
        self.bytes_out.store(0, Ordering::Relaxed);
    }
}

struct RunningTunnel {
    stop_tx: Sender<()>,
}

/// Tracks every tunnel currently running (or attempting to reconnect), keyed
/// by profile id. Shared as Tauri managed state.
#[derive(Default)]
pub struct TunnelManager {
    running: Mutex<HashMap<String, RunningTunnel>>,
}

impl TunnelManager {
    pub fn is_running(&self, id: &str) -> bool {
        self.running.lock().unwrap().contains_key(id)
    }

    pub fn stop(&self, id: &str) {
        if let Some(t) = self.running.lock().unwrap().remove(id) {
            let _ = t.stop_tx.send(());
        }
    }

    /// Spawns a background thread that connects over SSH and keeps the
    /// requested forward alive, reconnecting with backoff if the connection
    /// drops, until `stop` is called for this profile id.
    pub fn start(&self, app: AppHandle, profile: TunnelProfile) -> anyhow::Result<()> {
        let id = profile.id.clone();
        if self.is_running(&id) {
            return Ok(()); // already running, nothing to do
        }

        let (stop_tx, stop_rx) = mpsc::channel();
        self.running
            .lock()
            .unwrap()
            .insert(id.clone(), RunningTunnel { stop_tx });

        thread::spawn(move || run_with_reconnect(app, profile, stop_rx));
        Ok(())
    }
}

fn run_with_reconnect(app: AppHandle, profile: TunnelProfile, stop_rx: Receiver<()>) {
    let stop_rx = Arc::new(Mutex::new(stop_rx));
    let mut backoff = Duration::from_secs(1);
    let stats = Arc::new(TunnelStats::default());

    loop {
        emit_status(&app, &profile.id, "connecting", None);
        emit_log(
            &app,
            &profile.id,
            "info",
            format!("conectando a {}:{}…", profile.ssh_host, profile.ssh_port),
        );

        let was_connected = Arc::new(AtomicBool::new(false));
        stats.reset();
        emit_stats(&app, &profile.id, &stats);

        match connect_and_serve(&app, &profile, &stop_rx, &stats, &was_connected) {
            Ok(StopReason::UserRequested) => {
                emit_status(&app, &profile.id, "stopped", None);
                emit_log(&app, &profile.id, "info", "túnel detenido por el usuario");
                return;
            }
            Err(e) => {
                emit_log(&app, &profile.id, "error", format!("error: {e}"));
                emit_status(&app, &profile.id, "error", Some(e.to_string()));

                // Only notify for a connection that *dropped* after working
                // — an initial failed attempt (bad host/credentials/port)
                // is already visible in the form/list, and would otherwise
                // spam a notification on every reconnect retry.
                if was_connected.load(Ordering::SeqCst) {
                    notify(&app, &profile.name, &format!("El túnel se cayó: {e}"));
                }
            }
        }

        // Check for a stop request during the backoff wait.
        let woke_by_stop = wait_or_stop(&stop_rx, backoff);
        if woke_by_stop {
            emit_status(&app, &profile.id, "stopped", None);
            return;
        }
        emit_log(
            &app,
            &profile.id,
            "info",
            format!("reintentando en {:.0}s…", backoff.as_secs_f32()),
        );
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

/// Best-effort native OS notification. Failures (e.g. no notification
/// server running, permission denied) are swallowed — this is a nice-to-have,
/// never something that should crash or block the tunnel.
fn notify(app: &AppHandle, tunnel_name: &str, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title(format!("Túnel SSH: {tunnel_name}"))
        .body(body)
        .show();
}

/// Why `connect_and_serve` returned. Connection drops (including a dead
/// keepalive) and other failures surface as `Err` (handled by the reconnect
/// loop in `run_with_reconnect`); this only distinguishes a deliberate
/// user-initiated stop.
enum StopReason {
    UserRequested,
}

fn wait_or_stop(stop_rx: &Arc<Mutex<Receiver<()>>>, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match stop_rx.lock().unwrap().try_recv() {
            Ok(()) => return true,
            Err(TryRecvError::Disconnected) => return true,
            Err(TryRecvError::Empty) => {}
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(150));
    }
}

/// Establishes the "control" SSH connection: proves the host/credentials
/// work, carries the keepalive, and — for `Remote` tunnels only — hosts the
/// actual reverse listener (that one has to live on this specific session).
///
/// For `Local` and `Dynamic` tunnels, this session is **not** used to move
/// any forwarded traffic. Each accepted connection opens its own dedicated
/// SSH session instead (see `handle_local_connection` / `handle_socks_connection`).
/// This matters: libssh2 is not safe to call concurrently from multiple
/// threads on the same session, even for *different* channels of that
/// session — an earlier version of this code shared one session across all
/// forwarded connections plus the keepalive thread, which silently corrupted
/// the SSH transport under concurrent load (symptom: the UI shows
/// "connected" but the forwarded client just hangs). Giving every forwarded
/// connection its own session trades a bit of extra SSH-handshake overhead
/// per new connection for correctness.
fn connect_and_serve(
    app: &AppHandle,
    profile: &TunnelProfile,
    stop_rx: &Arc<Mutex<Receiver<()>>>,
    stats: &Arc<TunnelStats>,
    was_connected: &Arc<AtomicBool>,
) -> anyhow::Result<StopReason> {
    let tcp = TcpStream::connect((profile.ssh_host.as_str(), profile.ssh_port))?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    authenticate(&mut session, profile)?;
    session.set_blocking(true);
    session.set_keepalive(true, KEEPALIVE_INTERVAL_SECS);
    emit_log(
        app,
        &profile.id,
        "info",
        format!("autenticado como {}", profile.ssh_user),
    );

    let session = Arc::new(Mutex::new(session));
    emit_status(app, &profile.id, "connected", None);
    was_connected.store(true, Ordering::SeqCst);

    // Set by the keepalive thread once `ServerAliveCountMax`-equivalent
    // consecutive failures are hit; the accept loops below poll it and bail
    // out (as an Err) so the outer reconnect loop takes over.
    let connection_dead = Arc::new(AtomicBool::new(false));
    let workers_stop = Arc::new(AtomicBool::new(false));
    let keepalive_handle = spawn_keepalive_thread(
        app.clone(),
        profile.id.clone(),
        Arc::clone(&session),
        Arc::clone(&connection_dead),
        Arc::clone(&workers_stop),
    );
    let stats_handle = spawn_stats_thread(
        app.clone(),
        profile.id.clone(),
        Arc::clone(stats),
        Arc::clone(&workers_stop),
    );

    let result = match profile.kind {
        TunnelKind::Local => serve_local_forward(app, profile, stop_rx, &connection_dead, stats),
        TunnelKind::Remote => {
            serve_remote_forward(app, &session, profile, stop_rx, &connection_dead, stats)
        }
        TunnelKind::Dynamic => serve_socks_forward(app, profile, stop_rx, &connection_dead, stats),
    };

    workers_stop.store(true, Ordering::SeqCst);
    let _ = keepalive_handle.join();
    let _ = stats_handle.join();
    emit_stats(app, &profile.id, stats);

    if result.is_err() && connection_dead.load(Ordering::SeqCst) {
        anyhow::bail!(
            "sin respuesta del servidor tras {} keepalives (ServerAliveCountMax)",
            KEEPALIVE_MAX_FAILURES
        );
    }
    result
}

/// Periodically pushes a `tunnel-stats` snapshot to the UI while a tunnel is
/// running, so the connection-count/byte-counter display updates live.
fn spawn_stats_thread(
    app: AppHandle,
    profile_id: String,
    stats: Arc<TunnelStats>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            thread::sleep(STATS_REPORT_INTERVAL);
            emit_stats(&app, &profile_id, &stats);
        }
    })
}

/// Background thread that periodically calls `keepalive_send()` on the
/// control session, the libssh2 equivalent of OpenSSH's `ServerAliveInterval`
/// ping. After `KEEPALIVE_MAX_FAILURES` consecutive failures
/// (`ServerAliveCountMax`), it flags `connection_dead` so the accept loops
/// give up and let the reconnect loop in `run_with_reconnect` redial.
fn spawn_keepalive_thread(
    app: AppHandle,
    profile_id: String,
    session: Arc<Mutex<Session>>,
    connection_dead: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let failures = AtomicU32::new(0);
        loop {
            for _ in 0..KEEPALIVE_INTERVAL_SECS {
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                thread::sleep(Duration::from_secs(1));
            }
            if stop.load(Ordering::SeqCst) {
                return;
            }

            let sent = session.lock().unwrap().keepalive_send();
            match sent {
                Ok(_) => {
                    failures.store(0, Ordering::SeqCst);
                }
                Err(e) => {
                    let count = failures.fetch_add(1, Ordering::SeqCst) + 1;
                    emit_log(
                        &app,
                        &profile_id,
                        "warn",
                        format!("keepalive sin respuesta ({count}/{KEEPALIVE_MAX_FAILURES}): {e}"),
                    );
                    if count >= KEEPALIVE_MAX_FAILURES {
                        connection_dead.store(true, Ordering::SeqCst);
                        return;
                    }
                }
            }
        }
    })
}

pub(crate) fn authenticate(session: &mut Session, profile: &TunnelProfile) -> anyhow::Result<()> {
    match profile.auth_method {
        AuthMethod::Password => {
            let password = profile
                .password
                .clone()
                .or_else(|| credential_store::load_secret(&profile.id, "password").ok().flatten())
                .ok_or_else(|| anyhow::anyhow!("no se encontró la contraseña para este perfil"))?;
            session.userauth_password(&profile.ssh_user, &password)?;
        }
        AuthMethod::PrivateKey => {
            let key_path = profile
                .private_key_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("no se especificó una clave privada"))?;
            let passphrase = profile
                .passphrase
                .clone()
                .or_else(|| credential_store::load_secret(&profile.id, "passphrase").ok().flatten());
            session.userauth_pubkey_file(
                &profile.ssh_user,
                None,
                std::path::Path::new(key_path),
                passphrase.as_deref(),
            )?;
        }
        AuthMethod::Agent => {
            let mut agent = session.agent()?;
            agent.connect()?;
            agent.list_identities()?;
            let identity = agent
                .identities()?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("no hay identidades en el ssh-agent"))?;
            agent.userauth(&profile.ssh_user, &identity)?;
        }
    }

    if !session.authenticated() {
        anyhow::bail!("autenticación SSH rechazada por el servidor");
    }
    Ok(())
}

/// `ssh -L localPort:remoteHost:remotePort` — accept local TCP connections
/// and, for each one, dial a brand-new SSH session dedicated to that single
/// connection (see the note on `connect_and_serve` for why).
fn serve_local_forward(
    app: &AppHandle,
    profile: &TunnelProfile,
    stop_rx: &Arc<Mutex<Receiver<()>>>,
    connection_dead: &Arc<AtomicBool>,
    stats: &Arc<TunnelStats>,
) -> anyhow::Result<StopReason> {
    let listener = TcpListener::bind((profile.local_host.as_str(), profile.local_port))?;
    listener.set_nonblocking(true)?;
    emit_log(
        app,
        &profile.id,
        "info",
        format!("escuchando en {}:{}", profile.local_host, profile.local_port),
    );

    let remote_host = profile
        .remote_host
        .clone()
        .ok_or_else(|| anyhow::anyhow!("falta el host destino"))?;
    let remote_port = profile
        .remote_port
        .ok_or_else(|| anyhow::anyhow!("falta el puerto destino"))?;

    let conn_counter = AtomicU64::new(0);

    loop {
        if let Ok(()) | Err(TryRecvError::Disconnected) = stop_rx.lock().unwrap().try_recv() {
            return Ok(StopReason::UserRequested);
        }
        if connection_dead.load(Ordering::SeqCst) {
            anyhow::bail!("keepalive sin respuesta, se perdió la conexión SSH");
        }

        match listener.accept() {
            Ok((tcp_stream, addr)) => {
                let n = conn_counter.fetch_add(1, Ordering::SeqCst) + 1;
                emit_log(app, &profile.id, "info", format!("conexión #{n} entrante desde {addr}"));

                let app = app.clone();
                let profile = profile.clone();
                let remote_host = remote_host.clone();
                let stats = Arc::clone(stats);
                stats.active_connections.fetch_add(1, Ordering::Relaxed);
                thread::spawn(move || {
                    if let Err(e) = handle_local_connection(
                        &app,
                        &profile,
                        n,
                        tcp_stream,
                        &remote_host,
                        remote_port,
                        &stats,
                    ) {
                        emit_log(&app, &profile.id, "error", format!("conexión #{n} falló: {e}"));
                    } else {
                        emit_log(&app, &profile.id, "info", format!("conexión #{n} cerrada"));
                    }
                    stats.active_connections.fetch_sub(1, Ordering::Relaxed);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Dials, authenticates, and opens a `direct-tcpip` channel on a session
/// used by nobody but this one connection, then pumps data until either
/// side closes. Kept fully self-contained (and blocking) precisely so it
/// never needs to coordinate with any other thread's use of libssh2.
fn handle_local_connection(
    app: &AppHandle,
    profile: &TunnelProfile,
    n: u64,
    tcp_stream: TcpStream,
    remote_host: &str,
    remote_port: u16,
    stats: &Arc<TunnelStats>,
) -> anyhow::Result<()> {
    let tcp = TcpStream::connect((profile.ssh_host.as_str(), profile.ssh_port))?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    authenticate(&mut session, profile)?;
    session.set_blocking(true);

    emit_log(
        app,
        &profile.id,
        "info",
        format!("conexión #{n}: autenticado, abriendo canal hacia {remote_host}:{remote_port}"),
    );
    let channel = session.channel_direct_tcpip(remote_host, remote_port, None)?;
    emit_log(app, &profile.id, "info", format!("conexión #{n}: canal establecido, moviendo datos"));

    // Switch to non-blocking only now, for the data pump — see pump_channel's
    // doc comment for why blocking mode there causes a deadlock. Handshake,
    // auth, and channel setup above are simpler and more reliable in the
    // default blocking mode, so we don't touch that.
    session.set_blocking(false);

    // `session` stays alive for as long as `pump_channel` runs, which is
    // important: the channel is only valid while its parent session is.
    pump_channel(tcp_stream, channel, stats);
    drop(session);
    Ok(())
}

/// `ssh -R localPort:remoteHost:remotePort` — ask the server to listen on
/// `local_host:local_port` on its side, then for each incoming channel,
/// connect out to `remote_host:remote_port` reachable from this machine.
///
/// Unlike `Local`/`Dynamic`, the reverse listener is inherently tied to the
/// control session (the server maintains it for *this* SSH connection), so
/// accepted channels here do share that session with the keepalive thread.
/// To stay safe under libssh2's single-threaded-per-session rule, every
/// `listener.accept()` call and every connection's full pump both hold the
/// shared session lock, so they never overlap with a concurrent
/// `keepalive_send()`. The cost: keepalives pause while this thread is
/// blocked waiting for (or handling) a connection — acceptable for now, but
/// worth knowing if you see keepalive gaps on a busy `-R` tunnel.
fn serve_remote_forward(
    app: &AppHandle,
    session: &Arc<Mutex<Session>>,
    profile: &TunnelProfile,
    stop_rx: &Arc<Mutex<Receiver<()>>>,
    connection_dead: &Arc<AtomicBool>,
    stats: &Arc<TunnelStats>,
) -> anyhow::Result<StopReason> {
    let remote_host = profile
        .remote_host
        .clone()
        .ok_or_else(|| anyhow::anyhow!("falta el host destino"))?;
    let remote_port = profile
        .remote_port
        .ok_or_else(|| anyhow::anyhow!("falta el puerto destino"))?;

    let (mut listener, _bound_port) = session.lock().unwrap().channel_forward_listen(
        profile.local_port,
        Some(&profile.local_host),
        None,
    )?;
    emit_log(
        app,
        &profile.id,
        "info",
        format!("servidor escuchando en {}:{} (remoto)", profile.local_host, profile.local_port),
    );

    let conn_counter = AtomicU64::new(0);

    loop {
        if let Ok(()) | Err(TryRecvError::Disconnected) = stop_rx.lock().unwrap().try_recv() {
            return Ok(StopReason::UserRequested);
        }
        if connection_dead.load(Ordering::SeqCst) {
            anyhow::bail!("keepalive sin respuesta, se perdió la conexión SSH");
        }

        // Accept is itself a call into the shared session, so it goes
        // through the same lock the keepalive thread uses — otherwise
        // accept() and a concurrent keepalive_send() could race on the
        // same libssh2 session, which isn't safe. This does mean keepalives
        // pause while we're blocked here waiting for the next connection —
        // see the function doc comment.
        let accepted = {
            let _sess = session.lock().unwrap();
            listener.accept()
        };

        match accepted {
            Ok(channel) => {
                let n = conn_counter.fetch_add(1, Ordering::SeqCst) + 1;
                emit_log(app, &profile.id, "info", format!("conexión remota #{n} entrante"));
                match TcpStream::connect((remote_host.as_str(), remote_port)) {
                    Ok(tcp_stream) => {
                        emit_log(app, &profile.id, "info", format!("conexión remota #{n}: moviendo datos"));
                        // Hold the session lock for the *entire* pump so
                        // this never runs at the same time as the keepalive
                        // thread's use of the same session (see the module
                        // note above). Trade-off: keepalives pause while a
                        // remote-forwarded connection is active.
                        let sess = session.lock().unwrap();
                        sess.set_blocking(false);
                        stats.active_connections.fetch_add(1, Ordering::Relaxed);
                        pump_channel(tcp_stream, channel, stats);
                        stats.active_connections.fetch_sub(1, Ordering::Relaxed);
                        sess.set_blocking(true);
                        drop(sess);
                        emit_log(app, &profile.id, "info", format!("conexión remota #{n} cerrada"));
                    }
                    Err(e) => {
                        emit_log(
                            app,
                            &profile.id,
                            "error",
                            format!("conexión remota #{n}: no se pudo conectar a {remote_host}:{remote_port}: {e}"),
                        );
                    }
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// `ssh -D localPort` — a minimal synchronous SOCKS5 server (CONNECT only,
/// no auth). Just like `Local`, each accepted connection gets its own
/// dedicated SSH session for the same correctness reason.
fn serve_socks_forward(
    app: &AppHandle,
    profile: &TunnelProfile,
    stop_rx: &Arc<Mutex<Receiver<()>>>,
    connection_dead: &Arc<AtomicBool>,
    stats: &Arc<TunnelStats>,
) -> anyhow::Result<StopReason> {
    let listener = TcpListener::bind((profile.local_host.as_str(), profile.local_port))?;
    listener.set_nonblocking(true)?;
    emit_log(
        app,
        &profile.id,
        "info",
        format!("proxy SOCKS escuchando en {}:{}", profile.local_host, profile.local_port),
    );

    let conn_counter = AtomicU64::new(0);

    loop {
        if let Ok(()) | Err(TryRecvError::Disconnected) = stop_rx.lock().unwrap().try_recv() {
            return Ok(StopReason::UserRequested);
        }
        if connection_dead.load(Ordering::SeqCst) {
            anyhow::bail!("keepalive sin respuesta, se perdió la conexión SSH");
        }

        match listener.accept() {
            Ok((mut tcp_stream, addr)) => {
                let n = conn_counter.fetch_add(1, Ordering::SeqCst) + 1;
                emit_log(app, &profile.id, "info", format!("conexión SOCKS #{n} entrante desde {addr}"));

                tcp_stream.set_nonblocking(false)?;
                let app = app.clone();
                let profile = profile.clone();
                let stats = Arc::clone(stats);
                stats.active_connections.fetch_add(1, Ordering::Relaxed);
                thread::spawn(move || {
                    match socks5_handshake(&mut tcp_stream) {
                        Some((host, port)) => {
                            emit_log(
                                &app,
                                &profile.id,
                                "info",
                                format!("conexión SOCKS #{n}: destino {host}:{port}"),
                            );
                            if let Err(e) =
                                handle_socks_connection(&app, &profile, n, tcp_stream, &host, port, &stats)
                            {
                                emit_log(&app, &profile.id, "error", format!("conexión SOCKS #{n} falló: {e}"));
                            } else {
                                emit_log(&app, &profile.id, "info", format!("conexión SOCKS #{n} cerrada"));
                            }
                        }
                        None => {
                            emit_log(
                                &app,
                                &profile.id,
                                "warn",
                                format!("conexión SOCKS #{n}: handshake SOCKS5 inválido, descartada"),
                            );
                        }
                    }
                    stats.active_connections.fetch_sub(1, Ordering::Relaxed);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

fn handle_socks_connection(
    app: &AppHandle,
    profile: &TunnelProfile,
    n: u64,
    tcp_stream: TcpStream,
    host: &str,
    port: u16,
    stats: &Arc<TunnelStats>,
) -> anyhow::Result<()> {
    let tcp = TcpStream::connect((profile.ssh_host.as_str(), profile.ssh_port))?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    authenticate(&mut session, profile)?;
    session.set_blocking(true);

    let channel = session.channel_direct_tcpip(host, port, None)?;
    emit_log(app, &profile.id, "info", format!("conexión SOCKS #{n}: canal establecido"));

    session.set_blocking(false);
    pump_channel(tcp_stream, channel, stats);
    drop(session);
    Ok(())
}

/// Performs just enough of the SOCKS5 protocol to learn the requested
/// destination and reply "granted", returning `(host, port)` on success.
fn socks5_handshake(stream: &mut TcpStream) -> Option<(String, u16)> {
    let mut greeting = [0u8; 2];
    stream.read_exact(&mut greeting).ok()?;
    let nmethods = greeting[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).ok()?;
    stream.write_all(&[0x05, 0x00]).ok()?; // version 5, "no auth"

    let mut header = [0u8; 4];
    stream.read_exact(&mut header).ok()?;
    let (_ver, cmd, _rsv, atyp) = (header[0], header[1], header[2], header[3]);
    if cmd != 0x01 {
        return None; // only CONNECT is supported
    }

    let host = match atyp {
        0x01 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).ok()?;
            format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).ok()?;
            let mut domain = vec![0u8; len[0] as usize];
            stream.read_exact(&mut domain).ok()?;
            String::from_utf8(domain).ok()?
        }
        _ => return None, // IPv6 omitted for brevity
    };
    let mut port_buf = [0u8; 2];
    stream.read_exact(&mut port_buf).ok()?;
    let port = u16::from_be_bytes(port_buf);

    // Reply: success, bound address 0.0.0.0:0 (we don't expose a real bind).
    stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .ok()?;

    Some((host, port))
}

/// Pumps bytes in both directions between a plain TCP socket and an SSH
/// channel, on a single thread, until either side closes.
///
/// This requires the channel's session to already be in non-blocking mode
/// (`session.set_blocking(false)`) — deliberately so. An earlier version
/// used one thread per direction with a mutex guarding blocking reads on a
/// shared channel; that deadlocks the moment the *destination* speaks
/// second, which is exactly how most database wire protocols work (e.g.
/// SQL Server's TDS: the client sends PRELOGIN first). Here's why: the
/// "channel -> tcp" side would call a blocking `channel.read()` while
/// holding the lock, which blocks forever with nothing to read yet — and
/// while it's blocked *holding the lock*, the "tcp -> channel" side can
/// never acquire that same lock to forward the client's first bytes to the
/// server, so neither side ever moves. The UI showed "connected" and the
/// channel really was open; the client was just never able to speak.
///
/// Non-blocking + single thread avoids this entirely: neither direction
/// ever blocks waiting for the other.
fn pump_channel(mut tcp_stream: TcpStream, mut channel: ssh2::Channel, stats: &Arc<TunnelStats>) {
    if tcp_stream.set_nonblocking(true).is_err() {
        return;
    }
    let mut buf = [0u8; 8192];

    loop {
        let mut made_progress = false;

        match tcp_stream.read(&mut buf) {
            Ok(0) => break, // local side closed its end
            Ok(n) => {
                if write_all_retrying(&mut channel, &buf[..n]).is_err() {
                    break;
                }
                stats.bytes_out.fetch_add(n as u64, Ordering::Relaxed);
                made_progress = true;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        match channel.read(&mut buf) {
            Ok(0) => {
                if channel.eof() {
                    break;
                }
            }
            Ok(n) => {
                if write_all_retrying(&mut tcp_stream, &buf[..n]).is_err() {
                    break;
                }
                stats.bytes_in.fetch_add(n as u64, Ordering::Relaxed);
                made_progress = true;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        if channel.eof() {
            break;
        }
        if !made_progress {
            thread::sleep(Duration::from_millis(5));
        }
    }

    let _ = channel.close();
}

/// `Write::write_all` gives up on the very first `WouldBlock`, which is the
/// *expected*, routine outcome when writing to a non-blocking socket or
/// channel — so this retries instead of treating it as a hard failure.
fn write_all_retrying<W: Write>(w: &mut W, mut data: &[u8]) -> std::io::Result<()> {
    while !data.is_empty() {
        match w.write(data) {
            Ok(0) => {
                return Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "write devolvió 0 bytes"))
            }
            Ok(n) => data = &data[n..],
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
