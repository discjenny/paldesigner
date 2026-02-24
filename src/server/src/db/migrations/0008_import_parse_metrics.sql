ALTER TABLE save_import_versions
ADD COLUMN IF NOT EXISTS parse_metrics_json JSONB;
