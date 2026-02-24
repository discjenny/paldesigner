import type {
  HealthResponse,
  ImportProgressEvent,
  ImportVersionDetailResponse,
  ImportVersionListResponse,
  ImportZipResponse,
  NormalizedResponse,
} from "@/lib/types";

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

export function listImportVersions(): Promise<ImportVersionListResponse> {
  return getJson<ImportVersionListResponse>("/v1/save/import-versions");
}

export function getImportVersion(id: string): Promise<ImportVersionDetailResponse> {
  return getJson<ImportVersionDetailResponse>(`/v1/save/import-versions/${id}`);
}

export function getNormalized(id: string): Promise<NormalizedResponse> {
  return getJson<NormalizedResponse>(`/v1/save/import-versions/${id}/normalized`);
}

export function importZip(
  file: File,
  onUploadProgress: (progressPct: number) => void,
): Promise<ImportZipResponse> {
  return new Promise<ImportZipResponse>((resolve, reject) => {
    const form = new FormData();
    form.append("file", file);

    const request = new XMLHttpRequest();
    request.open("POST", "/api/v1/save/import-zip");
    request.responseType = "json";

    request.upload.onprogress = (event) => {
      if (!event.lengthComputable || event.total <= 0) {
        return;
      }
      const pct = Math.min(100, Math.max(0, Math.round((event.loaded / event.total) * 100)));
      onUploadProgress(pct);
    };

    request.onload = () => {
      if (request.status < 200 || request.status >= 300) {
        reject(new Error(`request failed (${request.status}): /v1/save/import-zip`));
        return;
      }
      const payload =
        (request.response as ImportZipResponse | null) ??
        (JSON.parse(request.responseText) as ImportZipResponse);
      resolve(payload);
    };

    request.onerror = () => {
      reject(new Error("request failed: network error"));
    };

    request.send(form);
  });
}

export function subscribeImportProgress(
  importVersionId: string,
  onProgress: (event: ImportProgressEvent) => void,
  onError: (message: string) => void,
): () => void {
  const source = new EventSource(`/api/v1/save/import-versions/${importVersionId}/events`);

  source.addEventListener("progress", (event) => {
    try {
      const payload = JSON.parse((event as MessageEvent<string>).data) as ImportProgressEvent;
      onProgress(payload);
    } catch {
      onError("invalid progress event payload");
    }
  });

  source.addEventListener("progress_error", (event) => {
    try {
      const payload = JSON.parse((event as MessageEvent<string>).data) as { error?: string };
      onError(payload.error ?? "progress stream error");
    } catch {
      onError("progress stream error");
    }
  });

  source.addEventListener("done", () => {
    source.close();
  });

  source.onerror = () => {
    source.close();
  };

  return () => source.close();
}
