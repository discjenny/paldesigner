ALTER TABLE save_import_versions
ADD COLUMN IF NOT EXISTS progress_phase TEXT NOT NULL DEFAULT 'upload_received';

ALTER TABLE save_import_versions
ADD COLUMN IF NOT EXISTS progress_pct INTEGER NOT NULL DEFAULT 0;

ALTER TABLE save_import_versions
ADD COLUMN IF NOT EXISTS progress_message TEXT NOT NULL DEFAULT '';

ALTER TABLE save_import_versions
ADD COLUMN IF NOT EXISTS failed_error TEXT;
