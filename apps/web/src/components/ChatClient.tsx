"use client";

import { FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DeviceInfo, Thread, WsServerMessage } from "@/lib/protocol";
import { WEB_CAPABILITIES } from "@/lib/protocol";

type Msg = { id: string; role: "user" | "assistant"; content: string; provider?: string };

export function ChatClient({ initialThreads }: { initialThreads: Thread[] }) {
  const [threads, setThreads] = useState(initialThreads);
  const [threadId, setThreadId] = useState<string | null>(initialThreads[0]?.id ?? null);
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [status, setStatus] = useState<"connecting" | "online" | "offline">("connecting");
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [streaming, setStreaming] = useState("");
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const deviceId = useMemo(() => {
    if (typeof window === "undefined") return "web-unknown";
    const key = "jarvis_web_device_id";
    let id = localStorage.getItem(key);
    if (!id) {
      id = `web-${crypto.randomUUID()}`;
      localStorage.setItem(key, id);
    }
    return id;
  }, []);

  const loadMessages = useCallback(async (id: string) => {
    const res = await fetch(`/api/brain/threads/${id}/messages`);
    if (!res.ok) return;
    const data = await res.json();
    setMessages(
      data.map((m: { id: string; role: string; content: string; provider?: string }) => ({
        id: m.id,
        role: m.role === "assistant" ? "assistant" : "user",
        content: m.content,
        provider: m.provider,
      })),
    );
  }, []);

  useEffect(() => {
    if (threadId) loadMessages(threadId);
  }, [threadId, loadMessages]);

  useEffect(() => {
    let cancelled = false;
    let heartbeat: ReturnType<typeof setInterval> | undefined;

    async function connect() {
      setStatus("connecting");
      const ticketRes = await fetch("/api/auth/ws-ticket", { method: "POST" });
      if (!ticketRes.ok) {
        setStatus("offline");
        setError("Could not obtain WS ticket");
        return;
      }
      const { ticket } = await ticketRes.json();
      const wsBase = process.env.NEXT_PUBLIC_BRAIN_WS_URL;
      if (!wsBase) {
        setStatus("offline");
        setError("NEXT_PUBLIC_BRAIN_WS_URL is not set");
        return;
      }

      const chatUrl = `${wsBase.replace(/\/$/, "")}/v1/chat/ws?ticket=${encodeURIComponent(ticket)}`;
      const deviceUrl = `${wsBase.replace(/\/$/, "")}/v1/device/ws?ticket=${encodeURIComponent(ticket)}`;

      const chatWs = new WebSocket(chatUrl);
      const deviceWs = new WebSocket(deviceUrl);
      wsRef.current = chatWs;

      chatWs.onopen = () => {
        if (!cancelled) setStatus("online");
      };
      chatWs.onclose = () => {
        if (!cancelled) setStatus("offline");
      };
      chatWs.onmessage = (ev) => {
        const msg = JSON.parse(ev.data) as WsServerMessage;
        if (msg.type === "chat.assistant.delta") {
          setStreaming((s) => s + msg.delta);
        } else if (msg.type === "chat.assistant.done") {
          setStreaming((current) => {
            setMessages((prev) => [
              ...prev,
              {
                id: msg.message_id,
                role: "assistant",
                content: current,
                provider: msg.provider,
              },
            ]);
            return "";
          });
        } else if (msg.type === "llm.providers_exhausted") {
          setError(msg.message);
        } else if (msg.type === "error") {
          setError(msg.message);
        }
      };

      deviceWs.onopen = () => {
        deviceWs.send(
          JSON.stringify({
            type: "device.hello",
            device_id: deviceId,
            platform: "web",
            capabilities: [...WEB_CAPABILITIES],
            label: "Web Console",
          }),
        );
        heartbeat = setInterval(() => {
          if (deviceWs.readyState === WebSocket.OPEN) {
            deviceWs.send(JSON.stringify({ type: "device.heartbeat", device_id: deviceId }));
          }
        }, 20000);
      };
      deviceWs.onmessage = (ev) => {
        const msg = JSON.parse(ev.data) as WsServerMessage & { devices?: DeviceInfo[] };
        if (msg.type === "device.list" && "devices" in msg) {
          setDevices((msg as { devices: DeviceInfo[] }).devices);
        }
      };

      return () => {
        chatWs.close();
        deviceWs.close();
      };
    }

    const cleanupPromise = connect();
    return () => {
      cancelled = true;
      if (heartbeat) clearInterval(heartbeat);
      void cleanupPromise.then((cleanup) => cleanup?.());
    };
  }, [deviceId]);

  async function createThread() {
    const res = await fetch("/api/brain/threads", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ title: "New chat" }),
    });
    if (!res.ok) return;
    const thread = (await res.json()) as Thread;
    setThreads((t) => [thread, ...t]);
    setThreadId(thread.id);
    setMessages([]);
  }

  async function onSend(e: FormEvent) {
    e.preventDefault();
    if (!threadId || !input.trim() || !wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      return;
    }
    setError(null);
    const content = input.trim();
    setInput("");
    setMessages((m) => [...m, { id: crypto.randomUUID(), role: "user", content }]);
    wsRef.current.send(
      JSON.stringify({ type: "chat.user", thread_id: threadId, content }),
    );
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[220px_1fr_220px]">
      <aside className="panel p-3">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm text-[var(--muted)]">Threads</h2>
          <button className="btn-ghost px-2 py-1 text-xs" type="button" onClick={createThread}>
            New
          </button>
        </div>
        <ul className="space-y-1 text-sm">
          {threads.map((t) => (
            <li key={t.id}>
              <button
                type="button"
                className={`w-full truncate px-2 py-1.5 text-left ${
                  threadId === t.id ? "bg-[var(--line)] text-[var(--text)]" : "text-[var(--muted)]"
                }`}
                onClick={() => setThreadId(t.id)}
              >
                {t.title}
              </button>
            </li>
          ))}
        </ul>
      </aside>

      <section className="panel flex min-h-[70vh] flex-col">
        <div className="flex items-center justify-between border-b border-[var(--line)] px-4 py-3 text-sm">
          <span className="brand text-[var(--accent)]">Chat</span>
          <span className={status === "online" ? "text-[var(--ok)]" : "text-[var(--muted)]"}>
            {status}
          </span>
        </div>
        <div className="flex-1 space-y-3 overflow-y-auto px-4 py-4">
          {messages.map((m) => (
            <div
              key={m.id}
              className={`max-w-[85%] whitespace-pre-wrap rounded-sm px-3 py-2 text-sm ${
                m.role === "user"
                  ? "ml-auto bg-[#1a2433]"
                  : "bg-[#151a22] border border-[var(--line)]"
              }`}
            >
              {m.content}
              {m.provider && (
                <div className="mt-1 text-[10px] uppercase tracking-wide text-[var(--muted)]">
                  {m.provider}
                </div>
              )}
            </div>
          ))}
          {streaming && (
            <div className="max-w-[85%] whitespace-pre-wrap border border-[var(--line)] bg-[#151a22] px-3 py-2 text-sm">
              {streaming}
            </div>
          )}
        </div>
        {error && <p className="px-4 text-sm text-[var(--danger)]">{error}</p>}
        <form onSubmit={onSend} className="flex gap-2 border-t border-[var(--line)] p-3">
          <input
            className="input"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Message Jarvis…"
            disabled={!threadId || status !== "online"}
          />
          <button className="btn" type="submit" disabled={!threadId || status !== "online"}>
            Send
          </button>
        </form>
      </section>

      <aside className="panel p-3">
        <h2 className="mb-3 text-sm text-[var(--muted)]">Devices</h2>
        <ul className="space-y-2 text-sm">
          {devices.length === 0 && <li className="text-[var(--muted)]">No devices online</li>}
          {devices.map((d) => (
            <li key={d.device_id} className="border border-[var(--line)] p-2">
              <div className="font-medium">{d.label ?? d.device_id}</div>
              <div className="text-xs text-[var(--muted)]">
                {d.platform} · {d.online ? "online" : "offline"}
              </div>
            </li>
          ))}
        </ul>
      </aside>
    </div>
  );
}
