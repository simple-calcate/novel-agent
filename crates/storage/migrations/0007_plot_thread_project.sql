ALTER TABLE plot_threads ADD COLUMN project_id TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_plot_threads_project ON plot_threads(project_id);
