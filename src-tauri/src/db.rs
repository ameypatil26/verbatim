use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{now_iso, AppSettings, ChatMessage, Note};

pub struct Database {
    conn: Mutex<Connection>,
    path: String,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| format!("create data dir: {error}"))?;
        }
        let conn = Connection::open(path).map_err(|error| format!("open notes db: {error}"))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| format!("set busy timeout: {error}"))?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT '',
                source_type TEXT NOT NULL DEFAULT 'manual',
                markdown TEXT NOT NULL DEFAULT '',
                transcript TEXT NOT NULL DEFAULT '',
                duration REAL NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notes_source_type ON notes(source_type);

            CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                title, markdown, transcript,
                content='notes', content_rowid='rowid',
                tokenize='unicode61 remove_diacritics 2'
            );

            CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
                INSERT INTO notes_fts(rowid, title, markdown, transcript)
                VALUES (new.rowid, new.title, new.markdown, new.transcript);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, title, markdown, transcript)
                VALUES ('delete', old.rowid, old.title, old.markdown, old.transcript);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, title, markdown, transcript)
                VALUES ('delete', old.rowid, old.title, old.markdown, old.transcript);
                INSERT INTO notes_fts(rowid, title, markdown, transcript)
                VALUES (new.rowid, new.title, new.markdown, new.transcript);
            END;

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_messages_note ON messages(note_id, id);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            PRAGMA user_version = 1;
            "#,
        )
        .map_err(|error| format!("initialize notes db: {error}"))?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: path.display().to_string(),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn save_note(&self, note: Note) -> Result<Note, String> {
        let note = note.normalized();
        let tags = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".into());
        let conn = self.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
        conn.execute(
            r#"
            INSERT INTO notes (id, title, source, source_type, markdown, transcript, duration, tags, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                source = excluded.source,
                source_type = excluded.source_type,
                markdown = excluded.markdown,
                transcript = excluded.transcript,
                duration = excluded.duration,
                tags = excluded.tags,
                updated_at = excluded.updated_at
            "#,
            params![
                note.id,
                note.title,
                note.source,
                note.source_type,
                note.markdown,
                note.transcript,
                note.duration,
                tags,
                note.created_at,
                note.updated_at
            ],
        )
        .map_err(|error| format!("save note: {error}"))?;
        Ok(note)
    }

    pub fn get_notes(&self, query: Option<&str>) -> Result<Vec<Note>, String> {
        let conn = self.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
        let cleaned = query.map(str::trim).filter(|value| !value.is_empty());
        if let Some(query) = cleaned {
            let fts = fts_query(query);
            if !fts.is_empty() {
                let mut stmt = conn
                    .prepare(
                        r#"
                        SELECT n.id, n.title, n.source, n.source_type, n.markdown, n.transcript,
                               n.duration, n.created_at, n.updated_at, n.tags
                        FROM notes n
                        JOIN notes_fts f ON f.rowid = n.rowid
                        WHERE notes_fts MATCH ?1
                        ORDER BY rank, n.updated_at DESC
                        LIMIT 200
                        "#,
                    )
                    .map_err(|error| format!("prepare search: {error}"))?;
                let rows = stmt
                    .query_map(params![fts], row_to_note)
                    .map_err(|error| format!("search notes: {error}"))?;
                return collect_notes(rows);
            }
            let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, title, source, source_type, markdown, transcript,
                           duration, created_at, updated_at, tags
                    FROM notes
                    WHERE title LIKE ?1 ESCAPE '\' OR markdown LIKE ?1 ESCAPE '\' OR transcript LIKE ?1 ESCAPE '\'
                    ORDER BY updated_at DESC
                    LIMIT 200
                    "#,
                )
                .map_err(|error| format!("prepare keyword search: {error}"))?;
            let rows = stmt
                .query_map(params![like], row_to_note)
                .map_err(|error| format!("keyword search: {error}"))?;
            return collect_notes(rows);
        }

        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, title, source, source_type, markdown, transcript,
                       duration, created_at, updated_at, tags
                FROM notes
                ORDER BY updated_at DESC
                LIMIT 500
                "#,
            )
            .map_err(|error| format!("prepare note list: {error}"))?;
        let rows = stmt
            .query_map([], row_to_note)
            .map_err(|error| format!("list notes: {error}"))?;
        collect_notes(rows)
    }

    pub fn delete_note(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
        conn.execute("DELETE FROM notes WHERE id = ?1", params![id])
            .map_err(|error| format!("delete note: {error}"))?;
        Ok(())
    }

    pub fn list_messages(&self, note_id: &str) -> Result<Vec<ChatMessage>, String> {
        let conn = self.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, note_id, role, content, created_at FROM messages WHERE note_id = ?1 ORDER BY id ASC",
            )
            .map_err(|error| format!("prepare messages: {error}"))?;
        let rows = stmt
            .query_map(params![note_id], |row| {
                Ok(ChatMessage {
                    id: Some(row.get(0)?),
                    note_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|error| format!("read messages: {error}"))?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|error| format!("message row: {error}"))?);
        }
        Ok(messages)
    }

    pub fn save_message(&self, mut message: ChatMessage) -> Result<ChatMessage, String> {
        if message.created_at.trim().is_empty() {
            message.created_at = now_iso();
        }
        let conn = self.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO messages (note_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![message.note_id, message.role, message.content, message.created_at],
        )
        .map_err(|error| format!("save message: {error}"))?;
        message.id = Some(conn.last_insert_rowid());
        Ok(message)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("get setting: {error}"))
    }

    pub fn put_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "database lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|error| format!("save setting: {error}"))?;
        Ok(())
    }

    pub fn get_settings(&self) -> AppSettings {
        self.get_setting("app")
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<AppSettings>(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let raw = serde_json::to_string(settings).map_err(|error| format!("serialize settings: {error}"))?;
        self.put_setting("app", &raw)
    }
}

fn row_to_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    let tags: String = row.get(9)?;
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        source: row.get(2)?,
        source_type: row.get(3)?,
        markdown: row.get(4)?,
        transcript: row.get(5)?,
        duration: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
    })
}

fn collect_notes(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Note>>,
) -> Result<Vec<Note>, String> {
    let mut notes = Vec::new();
    for row in rows {
        notes.push(row.map_err(|error| format!("note row: {error}"))?);
    }
    Ok(notes)
}

/// Turn user input into a safe FTS5 prefix query.
pub fn fts_query(input: &str) -> String {
    let tokens: Vec<String> = input
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty() && token.len() <= 64)
        .take(12)
        .map(|token| format!("\"{}\"*", token.replace('"', "")))
        .collect();
    tokens.join(" OR ")
}
