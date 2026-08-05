# Frontend — SSH Tunnel Manager

Vue 3 + TypeScript + [Nuxt UI](https://ui.nuxt.com/) (usado standalone, sin Nuxt) + Tailwind CSS v4 + Pinia.

## Estructura

```
src/
├── types/tunnel.ts          # Tipos TS que espejan src-tauri/src/models.rs
├── stores/tunnels.ts        # Store de Pinia: estado + llamadas invoke()/listen()
├── composables/             # Lógica reutilizable, separada de los componentes
│   ├── usePortCheck.ts      #   chequeo de puerto ocupado
│   ├── useConnectionTest.ts #   botón "Probar conexión"
│   └── useSshTemplates.ts   #   selector/guardado de plantillas SSH
├── components/
│   ├── TunnelKindBadge.vue         # Badge L/R/D
│   ├── TunnelStatusIndicator.vue   # Punto de estado + texto
│   ├── LogConsole.vue              # Panel de logs de un túnel
│   ├── TunnelCard.vue              # Una fila de túnel (usa los 3 de arriba)
│   ├── GroupSection.vue            # Sección colapsable por grupo
│   ├── TunnelList.vue              # Orquesta GroupSection + TunnelCard
│   ├── TemplatePicker.vue          # Selector de plantilla SSH (dentro del form)
│   └── TunnelFormModal.vue         # El formulario, dentro de un UModal real
└── App.vue                  # Layout: rail lateral, header, import/export
```

## Por qué Nuxt UI sin Nuxt

Desde la v3, Nuxt UI es una librería de componentes Vue que funciona en cualquier
proyecto con Vite (no requiere el framework Nuxt) — solo pide `vue-router` como
dependencia (aunque no la uses para rutas reales) y Tailwind CSS v4. Ver
`vite.config.ts` y `src/main.ts` para el setup completo.

## Comandos

```bash
npm run dev     # servidor de desarrollo (normalmente lo llama `cargo tauri dev`)
npm run build   # build de producción (normalmente lo llama `cargo tauri build`)
npx vue-tsc --noEmit   # chequeo de tipos, sin generar archivos
```
