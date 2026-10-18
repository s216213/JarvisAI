import { redirect } from "next/navigation";
import { brainFetch } from "@/lib/brain";

export const instant = false;

export default async function HomePage() {
  try {
    const res = await brainFetch("/v1/auth/me");
    if (res.ok) {
      redirect("/chat");
    }
  } catch {
    // Brain unreachable during local/build — fall through to login.
  }
  redirect("/login");
}
