use crate::models::{ClipItem, ContentType, SearchQuery};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::Mutex;

pub trait ClipStorage: Send + Sync {
    fn save_clip(&self, item: ClipItem) -> Result<String, String>;
    fn get_clip(&self, id: &str) -> Result<Option<ClipItem>, String>;
    fn search_clips(&self, query: &SearchQuery) -> Result<Vec<ClipItem>, String>;
    fn delete_clip(&self, id: &str) -> Result<bool, String>;
    fn set_pinned(&self, id: &str, pinned: bool) -> Result<bool, String>;
    fn count(&self) -> Result<usize, String>;
}

pub struct SqliteStorage {
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    pub fn new(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_tables()?;
        Ok(storage)
    }

    pub fn in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS clips (
                id TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                raw_content TEXT NOT NULL,
                content_hash TEXT NOT NULL UNIQUE,
                preview_text TEXT NOT NULL,
                char_count INTEGER NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                source_app TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_clips_created ON clips (created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clips_hash ON clips (content_hash);
            CREATE INDEX IF NOT EXISTS idx_clips_pinned ON clips (is_pinned);

            CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
                id UNINDEXED,
                raw_content
            );
            "#,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl ClipStorage for SqliteStorage {
    fn save_clip(&self, item: ClipItem) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let type_json = serde_json::to_string(&item.content_type).unwrap_or_else(|_| "Text".into());

        // Check if item with same hash already exists (deduplication)
        let mut check_stmt = conn
            .prepare("SELECT id FROM clips WHERE content_hash = ?1")
            .map_err(|e| e.to_string())?;
        let existing_id: Option<String> = check_stmt
            .query_row(params![item.content_hash], |row| row.get(0))
            .ok();

        if let Some(id) = existing_id {
            // Update timestamp to bump it to top of recent items
            conn.execute(
                "UPDATE clips SET created_at = ?1 WHERE id = ?2",
                params![item.created_at, id],
            )
            .map_err(|e| e.to_string())?;
            return Ok(id);
        }

        conn.execute(
            r#"
            INSERT INTO clips (id, content_type, raw_content, content_hash, preview_text, char_count, is_pinned, created_at, source_app)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                item.id,
                type_json,
                item.raw_content,
                item.content_hash,
                item.preview_text,
                item.char_count as i64,
                if item.is_pinned { 1 } else { 0 },
                item.created_at,
                item.source_app,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Also insert into FTS index
        let _ = conn.execute(
            "INSERT INTO clips_fts (id, raw_content) VALUES (?1, ?2)",
            params![item.id, item.raw_content],
        );

        Ok(item.id)
    }

    fn get_clip(&self, id: &str) -> Result<Option<ClipItem>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, content_type, raw_content, content_hash, preview_text, char_count, is_pinned, created_at, source_app FROM clips WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let res = stmt.query_row(params![id], |row| {
            let type_str: String = row.get(1)?;
            let content_type: ContentType =
                serde_json::from_str(&type_str).unwrap_or(ContentType::Text);
            let pinned_int: i64 = row.get(6)?;
            let char_count: i64 = row.get(5)?;

            Ok(ClipItem {
                id: row.get(0)?,
                content_type,
                raw_content: row.get(2)?,
                content_hash: row.get(3)?,
                preview_text: row.get(4)?,
                char_count: char_count as usize,
                is_pinned: pinned_int != 0,
                created_at: row.get(7)?,
                source_app: row.get(8)?,
            })
        });

        match res {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn search_clips(&self, query: &SearchQuery) -> Result<Vec<ClipItem>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let limit = if query.limit == 0 { 50 } else { query.limit };

        let q = query.query.trim();

        if q.is_empty() {
            // Retrieve recent items
            let sql = if query.pinned_only {
                "SELECT id, content_type, raw_content, content_hash, preview_text, char_count, is_pinned, created_at, source_app FROM clips WHERE is_pinned = 1 ORDER BY created_at DESC LIMIT ?1"
            } else {
                "SELECT id, content_type, raw_content, content_hash, preview_text, char_count, is_pinned, created_at, source_app FROM clips ORDER BY is_pinned DESC, created_at DESC LIMIT ?1"
            };

            let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![limit as i64], |row| {
                    let type_str: String = row.get(1)?;
                    let content_type: ContentType =
                        serde_json::from_str(&type_str).unwrap_or(ContentType::Text);
                    let pinned_int: i64 = row.get(6)?;
                    let char_count: i64 = row.get(5)?;

                    Ok(ClipItem {
                        id: row.get(0)?,
                        content_type,
                        raw_content: row.get(2)?,
                        content_hash: row.get(3)?,
                        preview_text: row.get(4)?,
                        char_count: char_count as usize,
                        is_pinned: pinned_int != 0,
                        created_at: row.get(7)?,
                        source_app: row.get(8)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            let mut items = Vec::new();
            for r in rows {
                items.push(r.map_err(|e| e.to_string())?);
            }
            return Ok(items);
        }

        // FTS search or LIKE search fallback
        let fts_query = format!("{}*", q.replace('"', "\"\""));
        let sql = r#"
            SELECT c.id, c.content_type, c.raw_content, c.content_hash, c.preview_text, c.char_count, c.is_pinned, c.created_at, c.source_app
            FROM clips c
            JOIN clips_fts f ON c.id = f.id
            WHERE clips_fts MATCH ?1
            ORDER BY c.is_pinned DESC, c.created_at DESC
            LIMIT ?2
        "#;

        let mut stmt = conn.prepare(sql);
        let items = match stmt {
            Ok(ref mut s) => {
                let rows = s.query_map(params![fts_query, limit as i64], |row| {
                    let type_str: String = row.get(1)?;
                    let content_type: ContentType =
                        serde_json::from_str(&type_str).unwrap_or(ContentType::Text);
                    let pinned_int: i64 = row.get(6)?;
                    let char_count: i64 = row.get(5)?;

                    Ok(ClipItem {
                        id: row.get(0)?,
                        content_type,
                        raw_content: row.get(2)?,
                        content_hash: row.get(3)?,
                        preview_text: row.get(4)?,
                        char_count: char_count as usize,
                        is_pinned: pinned_int != 0,
                        created_at: row.get(7)?,
                        source_app: row.get(8)?,
                    })
                });

                match rows {
                    Ok(r) => {
                        let mut list = Vec::new();
                        for item in r {
                            if let Ok(it) = item {
                                list.push(it);
                            }
                        }
                        list
                    }
                    Err(_) => Vec::new(),
                }
            }
            Err(_) => Vec::new(),
        };

        if !items.is_empty() {
            return Ok(items);
        }

        // Fallback to LIKE search if FTS syntax is tricky
        let like_query = format!("%{}%", q);
        let fallback_sql = "SELECT id, content_type, raw_content, content_hash, preview_text, char_count, is_pinned, created_at, source_app FROM clips WHERE raw_content LIKE ?1 ORDER BY is_pinned DESC, created_at DESC LIMIT ?2";
        let mut f_stmt = conn.prepare(fallback_sql).map_err(|e| e.to_string())?;
        let rows = f_stmt
            .query_map(params![like_query, limit as i64], |row| {
                let type_str: String = row.get(1)?;
                let content_type: ContentType =
                    serde_json::from_str(&type_str).unwrap_or(ContentType::Text);
                let pinned_int: i64 = row.get(6)?;
                let char_count: i64 = row.get(5)?;

                Ok(ClipItem {
                    id: row.get(0)?,
                    content_type,
                    raw_content: row.get(2)?,
                    content_hash: row.get(3)?,
                    preview_text: row.get(4)?,
                    char_count: char_count as usize,
                    is_pinned: pinned_int != 0,
                    created_at: row.get(7)?,
                    source_app: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    fn delete_clip(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count = conn
            .execute("DELETE FROM clips WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        let _ = conn.execute("DELETE FROM clips_fts WHERE id = ?1", params![id]);
        Ok(count > 0)
    }

    fn set_pinned(&self, id: &str, pinned: bool) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count = conn
            .execute(
                "UPDATE clips SET is_pinned = ?1 WHERE id = ?2",
                params![if pinned { 1 } else { 0 }, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(count > 0)
    }

    fn count(&self) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(count as usize)
    }
}

/// Hand-written fake storage for clean, isolated unit testing without SQLite file dependencies.
#[derive(Default)]
pub struct FakeStorage {
    items: Mutex<HashMap<String, ClipItem>>,
}

impl FakeStorage {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(HashMap::new()),
        }
    }
}

impl ClipStorage for FakeStorage {
    fn save_clip(&self, item: ClipItem) -> Result<String, String> {
        let mut map = self.items.lock().unwrap();
        // check duplicate hash
        for (id, existing) in map.iter_mut() {
            if existing.content_hash == item.content_hash {
                existing.created_at = item.created_at;
                return Ok(id.clone());
            }
        }
        let id = item.id.clone();
        map.insert(id.clone(), item);
        Ok(id)
    }

    fn get_clip(&self, id: &str) -> Result<Option<ClipItem>, String> {
        let map = self.items.lock().unwrap();
        Ok(map.get(id).cloned())
    }

    fn search_clips(&self, query: &SearchQuery) -> Result<Vec<ClipItem>, String> {
        let map = self.items.lock().unwrap();
        let mut res: Vec<ClipItem> = map
            .values()
            .filter(|item| {
                if query.pinned_only && !item.is_pinned {
                    return false;
                }
                if query.query.is_empty() {
                    return true;
                }
                item.raw_content
                    .to_lowercase()
                    .contains(&query.query.to_lowercase())
            })
            .cloned()
            .collect();

        res.sort_by(|a, b| {
            b.is_pinned
                .cmp(&a.is_pinned)
                .then_with(|| b.created_at.cmp(&a.created_at))
        });
        if query.limit > 0 && res.len() > query.limit {
            res.truncate(query.limit);
        }
        Ok(res)
    }

    fn delete_clip(&self, id: &str) -> Result<bool, String> {
        let mut map = self.items.lock().unwrap();
        Ok(map.remove(id).is_some())
    }

    fn set_pinned(&self, id: &str, pinned: bool) -> Result<bool, String> {
        let mut map = self.items.lock().unwrap();
        if let Some(item) = map.get_mut(id) {
            item.is_pinned = pinned;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn count(&self) -> Result<usize, String> {
        let map = self.items.lock().unwrap();
        Ok(map.len())
    }
}
