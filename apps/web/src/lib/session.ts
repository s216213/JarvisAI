import { redirect } from "next/navigation";
import { SessionUser, brainFetch } from "@/lib/brain";

export async function requireUser(): Promise<SessionUser> {
  try {
    const res = await brainFetch("/v1/auth/me");
    if (!res.ok) {
      redirect("/login");
    }
    return (await res.json()) as SessionUser;
  } catch {
    redirect("/login");
  }
}

export async function requireSuperadmin(): Promise<SessionUser> {
  const user = await requireUser();
  if (user.role !== "superadmin") {
    redirect("/chat");
  }
  return user;
}
