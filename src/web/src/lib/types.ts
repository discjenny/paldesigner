export type HealthStatus = "ok" | "ready" | "not_ready";

export interface HealthResponse {
  status: HealthStatus;
}

export interface NormalizedSummary {
  player_count: number;
  pal_count: number;
  base_assignment_count: number;
}

export interface ImportZipResponse {
  import_version_id: string;
  world_root_path: string;
  persisted_file_count: number;
  supported_file_count: number;
  normalized_summary: NormalizedSummary;
}

export interface ImportVersionListItem {
  id: string;
  source_file_name: string;
  world_root_path: string;
  status: string;
  progress_phase: string;
  progress_pct: number;
  progress_message: string;
  failed_error: string | null;
  parse_metrics_json: ParseMetrics | null;
  created_at: string;
  completed_at: string | null;
  file_count: number;
  supported_file_count: number;
  variant_row_count: number;
  player_count: number;
  pal_count: number;
  base_assignment_count: number;
}

export interface ImportVersionListResponse {
  versions: ImportVersionListItem[];
}

export interface ImportVersionFileItem {
  id: string;
  relative_path: string;
  is_supported: boolean;
  ignored_reason: string | null;
  byte_size: number;
  sha256: string;
  xxh64: string;
  created_at: string;
  has_cnk_prefix: boolean | null;
  magic: string | null;
  save_type: number | null;
  compression: string | null;
  uncompressed_size: number | null;
  compressed_size: number | null;
  gvas_magic: string | null;
  decompressed_size: number | null;
  decode_status: string | null;
  decode_error: string | null;
}

export interface ImportVersionDetailResponse {
  version: ImportVersionListItem;
  files: ImportVersionFileItem[];
}

export interface NormalizedPlayerRow {
  id: string;
  player_uid: string;
  player_instance_id: string | null;
  player_name: string | null;
  guild_id: string | null;
  level: number | null;
  raw_file_ref: string | null;
  raw_entity_path: string;
}

export interface NormalizedPalRow {
  id: string;
  pal_instance_id: string;
  owner_player_uid: string | null;
  species_id: string | null;
  nickname: string | null;
  level: number | null;
  raw_file_ref: string | null;
  raw_entity_path: string;
}

export interface NormalizedAssignmentRow {
  id: string;
  base_id: string;
  pal_instance_id: string;
  assignment_kind: string | null;
  assignment_target: string | null;
  priority: number | null;
  raw_file_ref: string | null;
  raw_entity_path: string;
}

export interface NormalizedResponse {
  import_version_id: string;
  players: NormalizedPlayerRow[];
  pals: NormalizedPalRow[];
  base_assignments: NormalizedAssignmentRow[];
}

export interface ImportProgressEvent {
  import_version_id: string;
  status: string;
  progress_phase: string;
  progress_pct: number;
  progress_message: string;
  failed_error: string | null;
  parse_metrics_json: ParseMetrics | null;
  completed_at: string | null;
  player_count: number;
  pal_count: number;
  base_assignment_count: number;
}

export interface ParseMetrics {
  decode_wrapper_ms: number;
  parse_gvas_ms: number;
  hint_pass_count: number;
  hint_count_start: number;
  hint_count_end: number;
  character_map_total: number;
  character_map_selected: number;
  character_map_decoded: number;
  basecamp_count: number;
  container_count: number;
  disabled_property_skips: number;
}
