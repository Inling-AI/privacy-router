CREATE TABLE inference_nodes (
    hash BLOB PRIMARY KEY CHECK (length(hash) = 32),
    parent_hash BLOB NOT NULL CHECK (length(parent_hash) = 32),
    entities_json TEXT NOT NULL
);
