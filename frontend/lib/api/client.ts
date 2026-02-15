import { config } from "@/lib/config";

const BASE_URL = config.indexerUrl;

export async function apiFetch<T>(
  path: string,
  options?: RequestInit
): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options?.headers,
    },
  });

  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(body || `API error: ${res.status} ${res.statusText}`);
  }

  // Handle empty responses (e.g. 201 Created, 200 OK with no body)
  const text = await res.text();
  if (!text) return undefined as T;

  return JSON.parse(text);
}
