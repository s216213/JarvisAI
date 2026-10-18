export type Role = "superadmin" | "user";

export type Platform = "web" | "windows" | "android";

export type TrustLevel = "safe" | "confirm" | "superadmin_only";

export type ProviderName = "groq" | "gemini" | "openrouter";

export interface UserPublic {
  id: string;
  email: string;
  role: Role;
  display_name: string | null;
}

export interface Thread {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface ChatMessage {
  id: string;
  thread_id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  provider: string | null;
  created_at: string;
}

export interface DeviceInfo {
  device_id: string;
  platform: Platform;
  capabilities: string[];
  label: string | null;
  online: boolean;
  last_seen: string;
}

export interface UserProfile {
  display_name: string | null;
  preferences: Record<string, unknown>;
  response_style: string | null;
}

export interface UserFact {
  id: string;
  key: string;
  value: string;
  source_thread: string | null;
  updated_at: string;
}

export interface PersonaVersion {
  id: string;
  label: string;
  system_prompt: string;
  created_by: string | null;
  created_at: string;
  is_active: boolean;
}

export interface ProviderHealth {
  name: ProviderName;
  state: "ready" | "cooling" | "disabled";
  requests: number;
  errors: number;
  last_429_at: string | null;
  cooling_until: string | null;
  latency_ms_ema: number | null;
}

export interface FeatureFlag {
  key: string;
  enabled: boolean;
  value: unknown;
}

export type WsClientMessage =
  | { type: "chat.user"; thread_id: string; content: string }
  | {
      type: "device.hello";
      device_id: string;
      platform: Platform;
      capabilities: string[];
      label?: string;
    }
  | { type: "device.heartbeat"; device_id: string }
  | { type: "tool.result"; invoke_id: string; ok: boolean; result?: unknown; error?: string };

export type WsServerMessage =
  | { type: "chat.assistant.delta"; thread_id: string; delta: string; provider?: string }
  | { type: "chat.assistant.done"; thread_id: string; message_id: string; provider?: string }
  | {
      type: "tool.invoke";
      invoke_id: string;
      tool: string;
      args: Record<string, unknown>;
      device_id?: string;
      trust?: TrustLevel;
    }
  | { type: "error"; code: string; message: string }
  | { type: "llm.providers_exhausted"; retry_after_secs: number; message: string }
  | { type: "device.list"; devices: DeviceInfo[] };

export const WEB_CAPABILITIES = [
  "web.notes.create",
  "web.memory.list",
  "web.devices.list",
] as const;
