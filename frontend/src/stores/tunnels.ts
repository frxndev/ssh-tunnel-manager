import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  GroupedProfiles,
  SshTemplate,
  TunnelLogEvent,
  TunnelLogLine,
  TunnelProfile,
  TunnelRunStatus,
  TunnelStats,
  TunnelStatsEvent,
  TunnelStatusEvent,
} from "../types/tunnel";

const MAX_LOG_LINES = 300;

interface TunnelStoreState {
  profiles: TunnelProfile[];
  templates: SshTemplate[];
  status: Record<string, TunnelRunStatus>;
  errors: Record<string, string>;
  logs: Record<string, TunnelLogLine[]>;
  stats: Record<string, TunnelStats>;
  unlistenStatus: (() => void) | null;
  unlistenLog: (() => void) | null;
  unlistenStats: (() => void) | null;
}

export const useTunnelStore = defineStore("tunnels", {
  state: (): TunnelStoreState => ({
    profiles: [],
    templates: [],
    status: {},
    errors: {},
    logs: {},
    stats: {},
    unlistenStatus: null,
    unlistenLog: null,
    unlistenStats: null,
  }),

  getters: {
    isRunning: (state) => (id: string) => state.status[id] === "connected",
    sortedProfiles: (state) => [...state.profiles].sort((a, b) => a.name.localeCompare(b.name)),
    logsFor: (state) => (id: string) => state.logs[id] ?? [],
    statsFor:
      (state) =>
      (id: string): TunnelStats =>
        state.stats[id] ?? { activeConnections: 0, bytesIn: 0, bytesOut: 0 },
    groupNames: (state): string[] => {
      const names = new Set(state.profiles.map((p) => p.group).filter((g): g is string => !!g));
      return [...names].sort();
    },
    groupedProfiles: (state): GroupedProfiles[] => {
      const byGroup = new Map<string | null, TunnelProfile[]>();
      for (const p of [...state.profiles].sort((a, b) => a.name.localeCompare(b.name))) {
        const key = p.group || null;
        if (!byGroup.has(key)) byGroup.set(key, []);
        byGroup.get(key)!.push(p);
      }
      const groups = [...byGroup.keys()].filter((k): k is string => k !== null).sort();
      const ordered: (string | null)[] = [...groups];
      if (byGroup.has(null)) ordered.push(null);
      return ordered.map((key) => ({ name: key, profiles: byGroup.get(key)! }));
    },
  },

  actions: {
    async init() {
      this.profiles = await invoke<TunnelProfile[]>("list_profiles");
      this.templates = await invoke<SshTemplate[]>("list_templates");
      for (const p of this.profiles) {
        this.status[p.id] = "stopped";
      }

      if (!this.unlistenStatus) {
        this.unlistenStatus = await listen<TunnelStatusEvent>("tunnel-status", (event) => {
          const { id, status, error } = event.payload;
          this.status[id] = status;
          if (error) this.errors[id] = error;
        });
      }

      if (!this.unlistenLog) {
        this.unlistenLog = await listen<TunnelLogEvent>("tunnel-log", (event) => {
          const { id, timestampMs, level, message } = event.payload;
          if (!this.logs[id]) this.logs[id] = [];
          this.logs[id].push({ timestampMs, level, message });
          if (this.logs[id].length > MAX_LOG_LINES) {
            this.logs[id].splice(0, this.logs[id].length - MAX_LOG_LINES);
          }
        });
      }

      if (!this.unlistenStats) {
        this.unlistenStats = await listen<TunnelStatsEvent>("tunnel-stats", (event) => {
          const { id, activeConnections, bytesIn, bytesOut } = event.payload;
          this.stats[id] = { activeConnections, bytesIn, bytesOut };
        });
      }
    },

    async saveProfile(profile: TunnelProfile): Promise<TunnelProfile> {
      const saved = await invoke<TunnelProfile>("save_profile", { profile });
      const idx = this.profiles.findIndex((p) => p.id === saved.id);
      if (idx >= 0) this.profiles[idx] = saved;
      else this.profiles.push(saved);
      if (!(saved.id in this.status)) this.status[saved.id] = "stopped";
      return saved;
    },

    async deleteProfile(id: string) {
      await this.stopTunnel(id);
      await invoke("delete_profile", { id });
      this.profiles = this.profiles.filter((p) => p.id !== id);
      delete this.errors[id];
      delete this.stats[id];
      delete this.status[id];
    },

    async startTunnel(id: string) {
      this.status[id] = "connecting";
      delete this.errors[id];
      try {
        await invoke("start_tunnel", { id });
        this.status[id] = "connected";
      } catch (err) {
        this.status[id] = "error";
        this.errors[id] = String(err);
        throw err;
      }
    },

    async stopTunnel(id: string) {
      try {
        await invoke("stop_tunnel", { id });
      } finally {
        this.status[id] = "stopped";
        this.stats[id] = { activeConnections: 0, bytesIn: 0, bytesOut: 0 };
      }
    },

    clearLogs(id: string) {
      this.logs[id] = [];
    },

    // -- SSH connection templates ------------------------------------

    async saveTemplate(template: SshTemplate): Promise<SshTemplate> {
      const saved = await invoke<SshTemplate>("save_template", { template });
      const idx = this.templates.findIndex((t) => t.id === saved.id);
      if (idx >= 0) this.templates[idx] = saved;
      else this.templates.push(saved);
      return saved;
    },

    async deleteTemplate(id: string) {
      await invoke("delete_template", { id });
      this.templates = this.templates.filter((t) => t.id !== id);
    },

    // -- Diagnostics ---------------------------------------------------

    async checkPortAvailable(host: string, port: number): Promise<boolean> {
      return invoke<boolean>("check_port_available", { host, port });
    },

    async testConnection(profile: TunnelProfile): Promise<string> {
      return invoke<string>("test_connection", { profile });
    },

    // -- Import / export (profiles only, secrets are never included) --

    async exportProfilesToFile(path: string): Promise<void> {
      return invoke("export_profiles_to_file", { path });
    },

    async importProfilesFromFile(path: string): Promise<number> {
      const count = await invoke<number>("import_profiles_from_file", { path });
      this.profiles = await invoke<TunnelProfile[]>("list_profiles");
      for (const p of this.profiles) {
        if (!(p.id in this.status)) this.status[p.id] = "stopped";
      }
      return count;
    },
  },
});
