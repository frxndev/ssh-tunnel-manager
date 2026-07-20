# Túneles SSH — Tauri + Rust + Vue

Skeleton de una app de escritorio multiplataforma (Windows/macOS/Linux) para
crear, guardar y controlar túneles SSH (`-L`, `-R`, `-D`/SOCKS) desde una UI.

## Estructura

```
ssh-tunnel-manager/
├── frontend/              # Vue 3 + TypeScript + Nuxt UI (standalone) + Pinia
│   └── src/
│       ├── types/tunnel.ts         # tipos TS que espejan src-tauri/src/models.rs
│       ├── stores/tunnels.ts       # estado + llamadas invoke()
│       ├── composables/            # lógica reutilizable (port check, test connection, plantillas)
│       ├── components/
│       │   ├── TunnelFormModal.vue     # alta/edición de perfiles, en un UModal real
│       │   ├── TunnelList.vue          # lista agrupada
│       │   ├── TunnelCard.vue          # una fila de túnel, con estado en vivo
│       │   └── ... (ver frontend/README.md para el resto)
│       └── App.vue
└── src-tauri/              # backend Rust
    └── src/
        ├── models.rs              # TunnelProfile, TunnelKind, AuthMethod
        ├── credential_store.rs    # secretos en el keychain del SO (crate `keyring`)
        ├── profile_store.rs       # perfiles (sin secretos) en JSON (tauri-plugin-store)
        ├── ssh_tunnel.rs          # motor de túneles: conecta con `ssh2`, hace
        │                          #   forwarding y reconecta con backoff
        ├── commands.rs            # comandos invoke() expuestos al frontend
        └── lib.rs / main.rs
```

## Qué se verificó en este entorno y qué no

- **Frontend (Vue 3 + TypeScript + Nuxt UI): compilado y verificado** — `npm run build` y `npx vue-tsc --noEmit` corren limpios, sin errores.
- **Backend (Rust/Tauri): no se compiló aquí** porque este entorno no tiene
  toolchain de Rust instalado. El código sigue los patrones estándar de
  Tauri 2 y la API estable y bien documentada de `ssh2` (`Session`,
  `channel_direct_tcpip`, `channel_forward_listen`), pero **corre
  `cargo build` en tu máquina antes de confiar en él** y ajusta lo que el
  compilador señale (versiones exactas de `tauri`/`tauri-plugin-store`/`ssh2`
  pueden requerir pequeños cambios de firma).

## Íconos

Los archivos en `src-tauri/icons/` son placeholders generados automáticamente
(un círculo simple en los colores de la app) sólo para que `cargo tauri dev`
y `cargo tauri build` no fallen por archivos faltantes. No incluí un
`icon.icns` (formato de macOS) porque no pude generarlo en este entorno.
Cuando tengas un ícono real, corre:

```bash
cargo tauri icon ruta/a/tu-icono.png
```

y va a regenerar todo el set (`.ico`, `.icns`, PNGs en varios tamaños)
automáticamente.

## Cómo correrlo

Requisitos: Node 18+, Rust estable (`rustup`), y las dependencias nativas de
Tauri para tu SO (ver https://v2.tauri.app/start/prerequisites/ — en Linux
necesitas `libwebkit2gtk`, en Windows el WebView2 runtime, etc.)

```bash
# instalar Tauri CLI si no lo tienes
cargo install tauri-cli --version "^2"

cd frontend
npm install

# desde la raíz del proyecto (src-tauri/tauri.conf.json ya apunta a ../frontend)
cargo tauri dev
```

`cargo tauri build` genera el instalador nativo (.msi/.dmg/.deb/.AppImage
según el SO donde compiles — Tauri no hace cross-compilation de GUI por
defecto, necesitas compilar en o para cada plataforma objetivo, p. ej. con
CI en GitHub Actions con runners de cada SO).

## Decisiones de diseño relevantes

- **`ssh2` en vez de `russh`**: `ssh2` (bindings a libssh2) tiene una API
  síncrona que no ha cambiado en años. Cada túnel corre en su propio hilo de
  SO (no bloquea el runtime async de Tauri), con reconexión automática y
  backoff exponencial (1s → 30s) si la conexión SSH cae.
- **Secretos fuera del JSON**: contraseñas y passphrases nunca tocan disco en
  texto plano — se guardan vía la crate `keyring` en Keychain (macOS),
  Credential Manager (Windows) o Secret Service/libsecret (Linux). El
  archivo `profiles.json` (vía `tauri-plugin-store`) sólo contiene metadata
  no sensible.
- **SOCKS dinámico simplificado**: la implementación de `-D` incluye sólo lo
  necesario del protocolo SOCKS5 (CONNECT, sin auth, sin IPv6/UDP) — es un
  punto de partida, no una implementación completa del RFC 1928.
- **Keepalive automático (`ServerAliveInterval`/`ServerAliveCountMax`)**: cada
  túnel activo envía un keepalive cada 60s (constantes `KEEPALIVE_INTERVAL_SECS`
  / `KEEPALIVE_MAX_FAILURES` en `ssh_tunnel.rs`, hoy fijas en 60s / 3 fallos,
  igual que `-o ServerAliveInterval=60 -o ServerAliveCountMax=3`). Si el
  servidor deja de responder, se da por caída la conexión y entra en el mismo
  ciclo de reconexión con backoff.
- **Una sesión SSH por conexión reenviada (`Local`/`Dynamic`)**: una versión
  anterior de este skeleton compartía una sola sesión SSH entre todas las
  conexiones reenviadas y el hilo de keepalive — libssh2 no permite eso de
  forma segura ni siquiera para *canales distintos* de la misma sesión, y
  causaba corrupción silenciosa del tráfico (síntoma real: la UI decía
  "conectado" pero el cliente de base de datos se quedaba colgado sin
  responder nunca). Ahora cada conexión local aceptada abre su propia sesión
  SSH dedicada — un poco más de overhead por handshake, pero correcto. El
  forwarding `Remote` (`-R`) sigue compartiendo la sesión de control porque
  el listener del lado servidor está atado a ella; por eso ahí las conexiones
  se atienden de a una a la vez en vez de concurrentemente (limitación
  conocida, documentada en el código).
- **Consola de logs por túnel**: el botón "⌘ Logs" en cada fila despliega un
  panel con la línea de tiempo real de esa conexión (evento `tunnel-log`) —
  cuándo se conecta al host SSH, cuándo autentica, cuándo entra una conexión
  local, cuándo abre el canal hacia el destino, y cualquier error en el
  camino. Útil para ver exactamente en qué paso se cuelga o falla una
  conexión, en vez de solo "Activo"/"Error" en la lista.
- **El pump de datos es de un solo hilo y no bloqueante**: la primera versión
  usaba dos hilos (uno por dirección) compartiendo el canal SSH detrás de un
  mutex, cada uno haciendo lecturas *bloqueantes*. Eso se traba apenas el
  destino "habla segundo" — que es como funcionan la mayoría de protocolos de
  bases de datos (por ejemplo el TDS de SQL Server: el cliente manda el
  PRELOGIN primero). El hilo de lectura del canal se quedaba bloqueado
  esperando datos del servidor *mientras sostenía el mutex*, y el hilo que
  necesitaba mandarle al servidor el primer mensaje del cliente nunca podía
  tomar ese mismo mutex — deadlock total (síntoma real: la UI decía
  "conectado", el canal realmente estaba abierto, pero el cliente jamás
  lograba hablar). Ahora `pump_channel` corre en un solo hilo con la sesión
  en modo no bloqueante, alternando lecturas de ambos lados sin bloquear
  nunca.

## Compilar sin instalar nada localmente

**Docker (`Dockerfile.build`) — sólo cubre el bundle de Linux.** Tauri
empaqueta el WebView nativo del sistema operativo, así que no existe
cross-compilación real hacia Windows/macOS desde un contenedor Linux. Lo que
sí puedes hacer es compilar el `.deb`/`.AppImage` sin tocar tu máquina:

```bash
docker build -f Dockerfile.build --target export \
  --output type=local,dest=./dist-linux .
```

Esto instala Rust, Node y las libs de GTK/webkit dentro del contenedor, y
deja los artefactos listos en `./dist-linux`.

**GitHub Actions (`.github/workflows/build.yml`) — la pieza que sí cubre
Windows y macOS.** Usa runners ya provistos por GitHub con los toolchains
nativos de cada SO, así que obtienes los tres instaladores (`.msi`, `.dmg`,
`.deb`/`.AppImage`) sin instalar absolutamente nada en tu equipo. Dispáralo
manualmente desde la pestaña "Actions" o empujando un tag `vX.Y.Z`.

**Para desarrollo día a día (`cargo tauri dev`)**, lo más práctico sigue
siendo instalar Rust localmente (es sólo `rustup` + las libs del sistema, no
es pesado) — un contenedor con GUI reenviada por X11 funciona en Linux pero
es incómodo en Windows/macOS, así que no lo recomiendo como flujo principal.

## Próximos pasos sugeridos

- Validar puertos duplicados/ocupados antes de guardar un perfil.
- Añadir `known_hosts` / verificación de fingerprint del servidor (ahora mismo
  no se valida la key del host — típico "TOFU" a implementar con
  `session.host_key()` + un store de fingerprints conocidos).
- Soporte de proxy jump (bastion intermedio) encadenando dos sesiones SSH.
- Empaquetar para móvil si lo necesitas: Tauri 2 soporta iOS/Android, pero
  el I/O crudo de sockets cambia bastante ahí (revisar límites de la
  plataforma para abrir listeners locales en segundo plano).
