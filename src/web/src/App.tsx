import { useEffect, useMemo, useRef, useState } from "react";
import { DatabaseZap, RefreshCw, UploadCloud } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  getImportVersion,
  getNormalized,
  importZip,
  listImportVersions,
  subscribeImportProgress,
} from "@/lib/api";
import type {
  ImportProgressEvent,
  ImportVersionDetailResponse,
  ImportVersionListItem,
  NormalizedResponse,
} from "@/lib/types";

interface PhaseProgress {
  phase: string;
  pct: number;
  message: string;
  status: "uploading" | "processing" | "ready" | "failed";
}

function App() {
  const [versions, setVersions] = useState<ImportVersionListItem[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<ImportVersionDetailResponse | null>(null);
  const [normalized, setNormalized] = useState<NormalizedResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [phaseProgress, setPhaseProgress] = useState<PhaseProgress | null>(null);
  const closeProgressStreamRef = useRef<(() => void) | null>(null);

  const selectedVersion = useMemo(
    () => versions.find((version) => version.id === selectedId) ?? null,
    [versions, selectedId],
  );

  async function refreshVersions() {
    setLoading(true);
    setError(null);
    try {
      const response = await listImportVersions();
      setVersions(response.versions);
      if (!selectedId && response.versions.length > 0) {
        setSelectedId(response.versions[0].id);
      }
      if (selectedId && !response.versions.some((version) => version.id === selectedId)) {
        setSelectedId(response.versions[0]?.id ?? null);
      }
    } catch (requestError) {
      setError((requestError as Error).message);
    } finally {
      setLoading(false);
    }
  }

  async function loadDetail(importVersionId: string) {
    setLoading(true);
    setError(null);
    try {
      const [detailResponse, normalizedResponse] = await Promise.all([
        getImportVersion(importVersionId),
        getNormalized(importVersionId),
      ]);
      setDetail(detailResponse);
      setNormalized(normalizedResponse);
    } catch (requestError) {
      setError((requestError as Error).message);
      setDetail(null);
      setNormalized(null);
    } finally {
      setLoading(false);
    }
  }

  async function uploadSelectedZip() {
    if (!selectedFile) {
      return;
    }

    closeProgressStreamRef.current?.();
    closeProgressStreamRef.current = null;

    setUploading(true);
    setError(null);
    setPhaseProgress({
      phase: "uploading_zip",
      pct: 1,
      message: "Uploading ZIP to server",
      status: "uploading",
    });

    try {
      const response = await importZip(selectedFile, (uploadPct) => {
        const overallPct = Math.round((uploadPct / 100) * 25);
        setPhaseProgress({
          phase: "uploading_zip",
          pct: overallPct,
          message: `Uploading ZIP (${uploadPct}%)`,
          status: "uploading",
        });
      });

      setPhaseProgress({
        phase: "queued_decode",
        pct: 30,
        message: "Upload complete, waiting for decode queue",
        status: "processing",
      });

      await refreshVersions();
      setSelectedId(response.import_version_id);
      setSelectedFile(null);

      closeProgressStreamRef.current = subscribeImportProgress(
        response.import_version_id,
        (event: ImportProgressEvent) => {
          const mappedPct = event.status === "ready" ? 100 : event.progress_pct;

          setPhaseProgress({
            phase: event.progress_phase,
            pct: mappedPct,
            message: event.progress_message,
            status:
              event.status === "failed"
                ? "failed"
                : event.status === "ready"
                  ? "ready"
                  : "processing",
          });

          setVersions((previous) =>
            previous.map((version) =>
              version.id === event.import_version_id
                ? {
                    ...version,
                    status: event.status,
                    progress_phase: event.progress_phase,
                    progress_pct: event.progress_pct,
                    progress_message: event.progress_message,
                    failed_error: event.failed_error,
                    parse_metrics_json: event.parse_metrics_json,
                    player_count: event.player_count,
                    pal_count: event.pal_count,
                    base_assignment_count: event.base_assignment_count,
                    completed_at:
                      event.status === "ready" || event.status === "failed"
                        ? (event.completed_at ?? version.completed_at)
                        : version.completed_at,
                  }
                : version,
            ),
          );

          if (selectedId === event.import_version_id) {
            setDetail((previous) => {
              if (!previous || previous.version.id !== event.import_version_id) {
                return previous;
              }
              return {
                ...previous,
                version: {
                  ...previous.version,
                  status: event.status,
                  progress_phase: event.progress_phase,
                  progress_pct: event.progress_pct,
                  progress_message: event.progress_message,
                  failed_error: event.failed_error,
                  parse_metrics_json: event.parse_metrics_json,
                  player_count: event.player_count,
                  pal_count: event.pal_count,
                  base_assignment_count: event.base_assignment_count,
                  completed_at:
                    event.status === "ready" || event.status === "failed"
                      ? (event.completed_at ?? previous.version.completed_at)
                      : previous.version.completed_at,
                },
              };
            });
          }

          if (event.status === "ready" || event.status === "failed") {
            closeProgressStreamRef.current?.();
            closeProgressStreamRef.current = null;
            void refreshVersions();
            void loadDetail(event.import_version_id);
            if (event.status === "failed") {
              setError(event.failed_error ?? "import failed");
            }
            setUploading(false);
          }
        },
        (message: string) => {
          setError(message);
          setUploading(false);
          closeProgressStreamRef.current?.();
          closeProgressStreamRef.current = null;
        },
      );
    } catch (requestError) {
      setError((requestError as Error).message);
      setPhaseProgress({
        phase: "upload_failed",
        pct: 100,
        message: "Upload failed",
        status: "failed",
      });
      closeProgressStreamRef.current?.();
      closeProgressStreamRef.current = null;
      setUploading(false);
    } finally {
      if (!closeProgressStreamRef.current) {
        setUploading(false);
      }
    }
  }

  useEffect(() => {
    void refreshVersions();
  }, []);

  useEffect(() => {
    return () => {
      closeProgressStreamRef.current?.();
      closeProgressStreamRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      setNormalized(null);
      return;
    }

    void loadDetail(selectedId);
  }, [selectedId]);

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-7xl flex-col gap-6 px-6 py-10">
      <header className="flex flex-col gap-4 rounded-lg border bg-card p-5 md:flex-row md:items-end md:justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <DatabaseZap className="h-4 w-4" />
            PostgreSQL Import Viewer
          </div>
          <h1 className="text-2xl font-semibold tracking-tight">Paldesigner Save Inspector</h1>
          <p className="text-sm text-muted-foreground">
            Inspect imported ZIP artifacts, decode metadata, and normalized planner rows.
          </p>
        </div>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
          <input
            className="rounded-md border bg-background px-2 py-1 text-sm"
            type="file"
            accept=".zip,application/zip"
            onChange={(event) => setSelectedFile(event.target.files?.[0] ?? null)}
          />
          <Button onClick={() => void uploadSelectedZip()} disabled={!selectedFile || uploading}>
            <UploadCloud className="mr-2 h-4 w-4" />
            {uploading ? "Uploading..." : "Import ZIP"}
          </Button>
          <Button variant="outline" onClick={() => void refreshVersions()} disabled={loading}>
            <RefreshCw className="mr-2 h-4 w-4" />
            Refresh
          </Button>
        </div>
      </header>
      {phaseProgress && (
        <section className="rounded-lg border bg-card p-4">
          <div className="mb-2 flex items-center justify-between text-xs text-muted-foreground">
            <span>
              phase={phaseProgress.phase} status={phaseProgress.status}
            </span>
            <span>{phaseProgress.pct}%</span>
          </div>
          <div className="h-2 w-full overflow-hidden rounded bg-muted">
            <div
              className={`h-full transition-all ${
                phaseProgress.status === "failed"
                  ? "bg-destructive"
                  : phaseProgress.status === "ready"
                    ? "bg-emerald-500"
                    : "bg-primary"
              }`}
              style={{ width: `${Math.max(0, Math.min(100, phaseProgress.pct))}%` }}
            />
          </div>
          <p className="mt-2 text-sm text-muted-foreground">{phaseProgress.message}</p>
        </section>
      )}

      {error && <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm">{error}</p>}

      <section className="grid gap-4 lg:grid-cols-[320px_1fr]">
        <aside className="rounded-lg border bg-card p-4">
          <h2 className="mb-3 text-sm font-semibold">Import Versions</h2>
          <div className="space-y-2">
            {versions.length === 0 && (
              <p className="text-sm text-muted-foreground">No imports yet.</p>
            )}
            {versions.map((version) => (
              <button
                key={version.id}
                type="button"
                className={`w-full rounded-md border px-3 py-2 text-left text-sm transition-colors ${
                  version.id === selectedId
                    ? "border-primary bg-primary/10"
                    : "border-border hover:bg-muted/60"
                }`}
                onClick={() => setSelectedId(version.id)}
              >
                <div className="font-medium">{version.source_file_name}</div>
                <div className="text-xs text-muted-foreground">
                  status={version.status} phase={version.progress_phase} progress={version.progress_pct}% files={version.file_count} variants={version.variant_row_count}
                </div>
              </button>
            ))}
          </div>
        </aside>

        <section className="space-y-4">
          {!selectedVersion && (
            <article className="rounded-lg border bg-card p-4 text-sm text-muted-foreground">
              Select an import version to view details.
            </article>
          )}

          {selectedVersion && detail && (
            <>
              <article className="rounded-lg border bg-card p-4">
                <h2 className="mb-3 text-sm font-semibold">Import Summary</h2>
                <div className="grid gap-2 text-sm md:grid-cols-2">
                  <SummaryRow label="ID" value={detail.version.id} />
                  <SummaryRow label="Status" value={detail.version.status} />
                  <SummaryRow
                    label="Progress"
                    value={`${detail.version.progress_phase} (${detail.version.progress_pct}%)`}
                  />
                  <SummaryRow label="Progress Message" value={detail.version.progress_message || "(none)"} />
                  <SummaryRow label="World Root" value={detail.version.world_root_path || "(root)"} />
                  <SummaryRow label="Created At" value={detail.version.created_at} />
                  <SummaryRow label="Files" value={`${detail.version.file_count}`} />
                  <SummaryRow
                    label="Normalized Counts"
                    value={`players=${detail.version.player_count} pals=${detail.version.pal_count} assignments=${detail.version.base_assignment_count}`}
                  />
                  <SummaryRow
                    label="Parse Metrics"
                    value={
                      detail.version.parse_metrics_json
                        ? `decode=${detail.version.parse_metrics_json.decode_wrapper_ms}ms gvas=${detail.version.parse_metrics_json.parse_gvas_ms}ms hints=${detail.version.parse_metrics_json.hint_pass_count} (${detail.version.parse_metrics_json.hint_count_start}->${detail.version.parse_metrics_json.hint_count_end})`
                        : "(not available)"
                    }
                  />
                </div>
              </article>

              <article className="rounded-lg border bg-card p-4">
                <h2 className="mb-3 text-sm font-semibold">Saved Files</h2>
                <div className="overflow-auto">
                  <table className="min-w-full text-left text-sm">
                    <thead className="text-xs text-muted-foreground">
                      <tr>
                        <th className="px-2 py-1">Path</th>
                        <th className="px-2 py-1">Size</th>
                        <th className="px-2 py-1">Compression</th>
                        <th className="px-2 py-1">Decode</th>
                        <th className="px-2 py-1">GVAS</th>
                      </tr>
                    </thead>
                    <tbody>
                      {detail.files.map((file) => (
                        <tr key={file.id} className="border-t">
                          <td className="px-2 py-1 font-mono text-xs">{file.relative_path}</td>
                          <td className="px-2 py-1">{formatBytes(file.byte_size)}</td>
                          <td className="px-2 py-1">
                            {file.compression ?? "n/a"} {file.magic ? `(${file.magic}/${file.save_type ?? "?"})` : ""}
                          </td>
                          <td className="px-2 py-1">{file.decode_status ?? "n/a"}</td>
                          <td className="px-2 py-1">
                            {file.gvas_magic ?? "n/a"}
                            {file.decompressed_size ? ` (${formatBytes(file.decompressed_size)})` : ""}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </article>
            </>
          )}

          {selectedVersion && normalized && (
            <article className="rounded-lg border bg-card p-4">
              <h2 className="mb-3 text-sm font-semibold">Normalized Rows</h2>
              <p className="mb-3 text-xs text-muted-foreground">
                players={normalized.players.length} pals={normalized.pals.length} assignments=
                {normalized.base_assignments.length}
              </p>
              <div className="grid gap-3 md:grid-cols-3">
                <SmallList title="Players" items={normalized.players.map((row) => row.player_uid)} />
                <SmallList title="Pals" items={normalized.pals.map((row) => row.pal_instance_id)} />
                <SmallList
                  title="Assignments"
                  items={normalized.base_assignments.map((row) => `${row.base_id}:${row.pal_instance_id}`)}
                />
              </div>
            </article>
          )}
        </section>
      </section>
    </main>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border bg-background px-2 py-1">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="break-all">{value}</div>
    </div>
  );
}

function SmallList({ title, items }: { title: string; items: string[] }) {
  return (
    <section className="rounded-md border bg-background p-2">
      <h3 className="mb-1 text-xs font-semibold text-muted-foreground">{title}</h3>
      <ul className="max-h-40 space-y-1 overflow-auto text-xs">
        {items.length === 0 && <li className="text-muted-foreground">none</li>}
        {items.map((item) => (
          <li key={item} className="rounded bg-muted/50 px-1 py-0.5 font-mono">
            {item}
          </li>
        ))}
      </ul>
    </section>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

export default App;
