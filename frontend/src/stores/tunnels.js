import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// A tunnel profile shape (mirrors the Rust `TunnelProfile` struct):
// {
//   id: string,
//   name: string,
//   kind: 'Local' | 'Remote' | 'Dynamic',
//   sshHost: string,
//   sshPort: number,
//   sshUser: string,
//   authMethod: 'Password' | 'PrivateKey' | 'Agent',
//   password: string | null,
//   privateKeyPath: string | null,
//   passphrase: string | null,
//   localHost: string,
//   localPort: number,
//   remoteHost: string | null,
//   remotePort: number | null,
// }

const MAX_LOG_LINES = 300

export const useTunnelStore = defineStore('tunnels', {
  state: () => ({
    profiles: [],
    templates: [],
    // id -> 'stopped' | 'connecting' | 'connected' | 'error'
    status: {},
    // id -> last error message
    errors: {},
    // id -> [{ timestampMs, level, message }, ...] (most recent last, capped)
    logs: {},
    // id -> { activeConnections, bytesIn, bytesOut }
    stats: {},
    unlistenStatus: null,
    unlistenLog: null,
    unlistenStats: null,
  }),

  getters: {
    isRunning: (state) => (id) => state.status[id] === 'connected',
    sortedProfiles: (state) => [...state.profiles].sort((a, b) => a.name.localeCompare(b.name)),
    logsFor: (state) => (id) => state.logs[id] ?? [],
    statsFor: (state) => (id) => state.stats[id] ?? { activeConnections: 0, bytesIn: 0, bytesOut: 0 },
    // Existing group names, for the "usar uno existente" datalist in the form.
    groupNames: (state) => {
      const names = new Set(state.profiles.map((p) => p.group).filter(Boolean))
      return [...names].sort()
    },
    // Profiles bucketed by group, group-less ones under `null`, groups sorted
    // alphabetically with the group-less bucket last.
    groupedProfiles: (state) => {
      const byGroup = new Map()
      for (const p of [...state.profiles].sort((a, b) => a.name.localeCompare(b.name))) {
        const key = p.group || null
        if (!byGroup.has(key)) byGroup.set(key, [])
        byGroup.get(key).push(p)
      }
      const groups = [...byGroup.keys()].filter((k) => k !== null).sort()
      if (byGroup.has(null)) groups.push(null)
      return groups.map((key) => ({ name: key, profiles: byGroup.get(key) }))
    },
  },

  actions: {
    async init() {
      this.profiles = await invoke('list_profiles')
      this.templates = await invoke('list_templates')
      for (const p of this.profiles) {
        this.status[p.id] = 'stopped'
      }

      // The Rust side emits `tunnel-status` events whenever a tunnel's
      // connection state changes (e.g. it drops and reconnects).
      if (!this.unlistenStatus) {
        this.unlistenStatus = await listen('tunnel-status', (event) => {
          const { id, status, error } = event.payload
          this.status[id] = status
          if (error) this.errors[id] = error
        })
      }

      // ...and `tunnel-log` for a running diagnostic feed per tunnel (SSH
      // connect/auth/channel-open/close/error steps) so failures that don't
      // simply bubble up as a status error are still visible somewhere.
      if (!this.unlistenLog) {
        this.unlistenLog = await listen('tunnel-log', (event) => {
          const { id, timestampMs, level, message } = event.payload
          if (!this.logs[id]) this.logs[id] = []
          this.logs[id].push({ timestampMs, level, message })
          if (this.logs[id].length > MAX_LOG_LINES) {
            this.logs[id].splice(0, this.logs[id].length - MAX_LOG_LINES)
          }
        })
      }

      // ...and `tunnel-stats` for the live active-connections/bytes counters.
      if (!this.unlistenStats) {
        this.unlistenStats = await listen('tunnel-stats', (event) => {
          const { id, activeConnections, bytesIn, bytesOut } = event.payload
          this.stats[id] = { activeConnections, bytesIn, bytesOut }
        })
      }
    },

    async saveProfile(profile) {
      const saved = await invoke('save_profile', { profile })
      const idx = this.profiles.findIndex((p) => p.id === saved.id)
      if (idx >= 0) this.profiles[idx] = saved
      else this.profiles.push(saved)
      if (!(saved.id in this.status)) this.status[saved.id] = 'stopped'
      return saved
    },

    async deleteProfile(id) {
      await this.stopTunnel(id)
      await invoke('delete_profile', { id })
      this.profiles = this.profiles.filter((p) => p.id !== id)
      delete this.status[id]
      delete this.errors[id]
      delete this.stats[id]
    },

    async startTunnel(id) {
      this.status[id] = 'connecting'
      delete this.errors[id]
      try {
        await invoke('start_tunnel', { id })
        this.status[id] = 'connected'
      } catch (err) {
        this.status[id] = 'error'
        this.errors[id] = String(err)
        throw err
      }
    },

    async stopTunnel(id) {
      try {
        await invoke('stop_tunnel', { id })
      } finally {
        this.status[id] = 'stopped'
        this.stats[id] = { activeConnections: 0, bytesIn: 0, bytesOut: 0 }
      }
    },

    clearLogs(id) {
      this.logs[id] = []
    },

    // -- SSH connection templates ------------------------------------

    async saveTemplate(template) {
      const saved = await invoke('save_template', { template })
      const idx = this.templates.findIndex((t) => t.id === saved.id)
      if (idx >= 0) this.templates[idx] = saved
      else this.templates.push(saved)
      return saved
    },

    async deleteTemplate(id) {
      await invoke('delete_template', { id })
      this.templates = this.templates.filter((t) => t.id !== id)
    },

    // -- Diagnostics ---------------------------------------------------

    async checkPortAvailable(host, port) {
      return invoke('check_port_available', { host, port })
    },

    async testConnection(profile) {
      return invoke('test_connection', { profile })
    },

    // -- Import / export (profiles only, secrets are never included) --

    async exportProfilesToFile(path) {
      return invoke('export_profiles_to_file', { path })
    },

    async importProfilesFromFile(path) {
      const count = await invoke('import_profiles_from_file', { path })
      this.profiles = await invoke('list_profiles')
      for (const p of this.profiles) {
        if (!(p.id in this.status)) this.status[p.id] = 'stopped'
      }
      return count
    },
  },
})
