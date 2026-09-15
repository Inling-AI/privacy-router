use crate::{Result, Store};
use privacy_filter::Entity;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

/// One completed turn. Text stays in the request; only predictions are retained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceNode {
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub entities: Vec<Vec<Entity>>,
}

pub struct InferenceRepository<'a> {
    store: &'a Store,
}

impl<'a> InferenceRepository<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub async fn find_many(&self, hashes: &[[u8; 32]]) -> Result<HashMap<[u8; 32], InferenceNode>> {
        let mut found = HashMap::new();
        for chunk in hashes.chunks(400) {
            let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT hash, parent_hash, entities_json FROM inference_nodes WHERE hash IN (",
            );
            let mut separated = query.separated(", ");
            for hash in chunk {
                separated.push_bind(hash.to_vec());
            }
            separated.push_unseparated(")");
            for row in query.build().fetch_all(self.store.pool()).await? {
                let hash: Vec<u8> = row.try_get("hash")?;
                let parent: Vec<u8> = row.try_get("parent_hash")?;
                let node = InferenceNode {
                    hash: hash
                        .try_into()
                        .map_err(|_| crate::Error::Encoding("invalid node hash".into()))?,
                    parent_hash: parent
                        .try_into()
                        .map_err(|_| crate::Error::Encoding("invalid parent hash".into()))?,
                    entities: serde_json::from_str(&row.try_get::<String, _>("entities_json")?)
                        .map_err(|e| crate::Error::Encoding(e.to_string()))?,
                };
                found.insert(node.hash, node);
            }
        }
        Ok(found)
    }

    pub async fn insert(&self, nodes: &[InferenceNode]) -> Result<()> {
        let mut transaction = self.store.pool().begin().await?;
        for node in nodes {
            let entities = serde_json::to_string(&node.entities)
                .map_err(|e| crate::Error::Encoding(e.to_string()))?;
            sqlx::query("INSERT INTO inference_nodes (hash, parent_hash, entities_json) VALUES (?, ?, ?) ON CONFLICT(hash) DO NOTHING")
                .bind(node.hash.to_vec()).bind(node.parent_hash.to_vec()).bind(entities)
                .execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}
