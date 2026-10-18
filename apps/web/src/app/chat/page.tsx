import { AppShell } from "@/components/AppShell";
import { ChatClient } from "@/components/ChatClient";
import { brainFetch } from "@/lib/brain";
import { requireUser } from "@/lib/session";
import type { Thread } from "@/lib/protocol";

export const instant = false;

export default async function ChatPage() {
  const user = await requireUser();
  const threadsRes = await brainFetch("/v1/threads");
  const threads: Thread[] = threadsRes.ok ? await threadsRes.json() : [];

  return (
    <AppShell user={user}>
      <ChatClient initialThreads={threads} />
    </AppShell>
  );
}
