CREATE TABLE IF NOT EXISTS planner_player_links (
    id UUID PRIMARY KEY,
    planner_player_id UUID NOT NULL REFERENCES planner_players(id) ON DELETE RESTRICT,
    save_file_id UUID NOT NULL REFERENCES save_files(id) ON DELETE RESTRICT,
    raw_entity_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (planner_player_id, save_file_id, raw_entity_path)
);

CREATE TABLE IF NOT EXISTS planner_pal_links (
    id UUID PRIMARY KEY,
    planner_pal_id UUID NOT NULL REFERENCES planner_pals(id) ON DELETE RESTRICT,
    save_file_id UUID NOT NULL REFERENCES save_files(id) ON DELETE RESTRICT,
    raw_entity_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (planner_pal_id, save_file_id, raw_entity_path)
);

CREATE TABLE IF NOT EXISTS planner_base_assignment_links (
    id UUID PRIMARY KEY,
    planner_base_assignment_id UUID NOT NULL REFERENCES planner_base_assignments(id) ON DELETE RESTRICT,
    save_file_id UUID NOT NULL REFERENCES save_files(id) ON DELETE RESTRICT,
    raw_entity_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (planner_base_assignment_id, save_file_id, raw_entity_path)
);
