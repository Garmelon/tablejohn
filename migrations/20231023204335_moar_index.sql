CREATE INDEX idx_commits_hash_reachable
ON commits (hash, reachable);

CREATE INDEX idx_commit_links_child_parent
ON commit_links (child, parent);
