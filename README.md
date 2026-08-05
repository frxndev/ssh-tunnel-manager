# SSH Tunnel Manager

<img width="2504" height="1824" alt="imagen" src="https://github.com/user-attachments/assets/2c8ef9c8-2334-4402-a128-6e9e6cee7c0f" />

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
según el SO donde compiles.
