// These types mirror src-tauri/src/models.rs field-for-field. If you add or
// rename a field on the Rust side, update it here too — nothing enforces
// this automatically across the Rust/TS boundary.

export type TunnelKind = "Local" | "Remote" | "Dynamic";

export type AuthMethod = "Password" | "PrivateKey" | "Agent";

export type TunnelRunStatus = "stopped" | "connecting" | "connected" | "error";

export type LogLevel = "info" | "warn" | "error";

export interface TunnelProfile {
  id: string;
  name: string;
  kind: TunnelKind;
  group?: string;

  sshHost: string;
  sshPort: number;
  sshUser: string;

  authMethod: AuthMethod;
  password?: string;
  privateKeyPath?: string;
  passphrase?: string;

  localHost: string;
  localPort: number;

  remoteHost?: string;
  remotePort?: number;
}

/** A blank profile draft — same shape, but every field has a concrete
 * default so the form never has to deal with `undefined`. */
export function createBlankProfile(): TunnelProfile {
  return {
    id: "",
    name: "",
    kind: "Local",
    group: "",
    sshHost: "",
    sshPort: 22,
    sshUser: "",
    authMethod: "PrivateKey",
    password: "",
    privateKeyPath: "",
    passphrase: "",
    localHost: "127.0.0.1",
    localPort: 8080,
    remoteHost: "",
    remotePort: 80,
  };
}

export interface SshTemplate {
  id: string;
  name: string;
  sshHost: string;
  sshPort: number;
  sshUser: string;
  authMethod: AuthMethod;
  privateKeyPath?: string;
}

export interface TunnelStatusEvent {
  id: string;
  status: TunnelRunStatus;
  error?: string;
}

export interface TunnelLogEvent {
  id: string;
  timestampMs: number;
  level: LogLevel;
  message: string;
}

export interface TunnelStatsEvent {
  id: string;
  activeConnections: number;
  bytesIn: number;
  bytesOut: number;
}

export interface TunnelStats {
  activeConnections: number;
  bytesIn: number;
  bytesOut: number;
}

export interface TunnelLogLine {
  timestampMs: number;
  level: LogLevel;
  message: string;
}

export interface GroupedProfiles {
  name: string | null;
  profiles: TunnelProfile[];
}

export interface TestConnectionResult {
  ok: boolean;
  message: string;
}
