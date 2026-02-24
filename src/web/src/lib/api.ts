import type { HealthResponse } from "@/lib/types";

async function getJson<T>(path: string): Promise<T> {
  const response = await fetch(`/api${path}`);
  if (!response.ok) {
    throw new Error(`request failed (${response.status}): ${path}`);
  }
  return (await response.json()) as T;
}

export function getHealth(): Promise<HealthResponse> {
  return getJson<HealthResponse>("/health");
}

export function getReady(): Promise<HealthResponse> {
  return getJson<HealthResponse>("/ready");
}
