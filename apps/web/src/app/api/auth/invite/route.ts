import { NextResponse } from "next/server";
import { ACCESS_COOKIE, REFRESH_COOKIE, brainUrl } from "@/lib/brain";

export async function POST(req: Request) {
  const body = await req.json();
  const res = await fetch(`${brainUrl()}/v1/auth/invite/accept`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const data = await res.json();
  if (!res.ok) {
    return NextResponse.json(data, { status: res.status });
  }

  const response = NextResponse.json({ user: data.user });
  const secure = process.env.NODE_ENV === "production";
  response.cookies.set(ACCESS_COOKIE, data.access_token, {
    httpOnly: true,
    sameSite: "lax",
    secure,
    path: "/",
    maxAge: 60 * 15,
  });
  response.cookies.set(REFRESH_COOKIE, data.refresh_token, {
    httpOnly: true,
    sameSite: "lax",
    secure,
    path: "/",
    maxAge: 60 * 60 * 24 * 30,
  });
  return response;
}
