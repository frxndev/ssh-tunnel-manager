import { ref } from "vue";
import { useTunnelStore } from "../stores/tunnels";
import type { TunnelKind } from "../types/tunnel";

export type PortStatus = "checking" | "free" | "busy" | null;

/** Checks whether a local host:port is free before the person saves a
 * profile, so a "port already in use" failure shows up here instead of only
 * when the tunnel fails to start later. */
export function usePortCheck() {
  const store = useTunnelStore();
  const portStatus = ref<PortStatus>(null);

  async function checkPort(kind: TunnelKind, host: string, port: number) {
    if (kind === "Remote") {
      // That port is bound on the *server* side for a remote forward, not
      // on this machine, so a local bind check doesn't apply.
      portStatus.value = null;
      return;
    }
    portStatus.value = "checking";
    try {
      const available = await store.checkPortAvailable(host || "127.0.0.1", port);
      portStatus.value = available ? "free" : "busy";
    } catch {
      portStatus.value = null;
    }
  }

  function resetPortStatus() {
    portStatus.value = null;
  }

  return { portStatus, checkPort, resetPortStatus };
}
