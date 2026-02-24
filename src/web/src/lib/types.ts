export type HealthStatus = "ok" | "ready" | "not_ready";

export interface HealthResponse {
  status: HealthStatus;
}
