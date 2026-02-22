/**
 * @module api
 *
 * Authenticated fetch wrapper for admin API endpoints.
 *
 * Reads the current Supabase session and attaches the JWT as a
 * `Bearer` token in the `Authorization` header. All admin endpoints
 * on the Rust backend require this token — the `RequireAdmin`
 * extractor returns 401/403 without it.
 */

import { supabase } from "@/lib/supabase";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "";

/** Fetch wrapper that auto-attaches the Supabase JWT for admin endpoints. */
export async function adminFetch(
  path: string,
  options?: RequestInit,
): Promise<Response> {
  const {
    data: { session },
  } = await supabase.auth.getSession();
  const headers = new Headers(options?.headers);
  if (session?.access_token) {
    headers.set("Authorization", `Bearer ${session.access_token}`);
  }
  return fetch(`${API_BASE}${path}`, { ...options, headers });
}
