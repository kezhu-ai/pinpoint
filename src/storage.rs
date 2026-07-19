//! SQLite store for saved prompts.

use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct Prompt {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub tags: String,
    pub source: String,
    pub created_at: String,
}

pub struct Store {
    pub conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        let s = Self { conn };
        s.init_schema()?;
        Ok(s)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS prompts (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL UNIQUE,
                content     TEXT NOT NULL,
                tags        TEXT,
                source      TEXT,
                created_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_prompts_created ON prompts(created_at);
            CREATE INDEX IF NOT EXISTS idx_prompts_tags ON prompts(tags);
            CREATE VIRTUAL TABLE IF NOT EXISTS prompts_fts USING fts5(
                content, name, tags,
                content='prompts', content_rowid='id',
                tokenize='porter unicode61'
            );
            CREATE TRIGGER IF NOT EXISTS prompts_ai AFTER INSERT ON prompts BEGIN
                INSERT INTO prompts_fts(rowid, content, name, tags)
                VALUES (new.id, new.content, new.name, new.tags);
            END;
            CREATE TRIGGER IF NOT EXISTS prompts_ad AFTER DELETE ON prompts BEGIN
                INSERT INTO prompts_fts(prompts_fts, rowid, content, name, tags)
                VALUES ('delete', old.id, old.content, old.name, old.tags);
            END;
            CREATE TRIGGER IF NOT EXISTS prompts_au AFTER UPDATE ON prompts BEGIN
                INSERT INTO prompts_fts(prompts_fts, rowid, content, name, tags)
                VALUES ('delete', old.id, old.content, old.name, old.tags);
                INSERT INTO prompts_fts(rowid, content, name, tags)
                VALUES (new.id, new.content, new.name, new.tags);
            END;
            "#,
        )?;
        Ok(())
    }

    pub fn log_prompt(&mut self, content: &str, name: Option<&str>, tags: &str, source: &str) -> Result<Prompt> {
        let now = Utc::now().to_rfc3339();
        let name = match name {
            Some(n) => n.to_string(),
            None => self.auto_name(content)?,
        };
        // upsert by name
        self.conn.execute(
            "INSERT INTO prompts(name, content, tags, source, created_at) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET content=excluded.content, tags=excluded.tags, source=excluded.source, created_at=excluded.created_at",
            params![name, content, tags, source, now],
        )?;
        self.get_prompt(&name)?.with_context(|| format!("just-saved prompt {:?} missing", name))
    }

    fn auto_name(&self, content: &str) -> Result<String> {
        // heuristic: first 3 alpha words, lowercased, joined by '-'
        let words: Vec<String> = content
            .split_whitespace()
            .filter_map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().into())
            .filter(|w: &String| !w.is_empty())
            .take(4)
            .map(|w| w.to_lowercase())
            .collect();
        let base = if words.is_empty() {
            format!("pp-{}", Utc::now().format("%Y%m%d-%H%M%S"))
        } else {
            words.join("-").chars().take(40).collect()
        };
        // ensure uniqueness by appending -2, -3, ... if needed
        let candidate = base.clone();
        let mut candidate = candidate;
        let mut n = 2;
        loop {
            let exists: bool = self.conn.query_row(
                "SELECT 1 FROM prompts WHERE name = ?",
                params![candidate],
                |_| Ok(true),
            ).optional()?.unwrap_or(false);
            if !exists { break; }
            candidate = format!("{}-{}", base, n);
            n += 1;
            if n > 99 { anyhow::bail!("could not auto-name prompt after 99 tries"); }
        }
        Ok(candidate)
    }

    pub fn list_prompts(&self, tag_filter: Option<&str>, source_filter: Option<&str>, limit: u32) -> Result<Vec<Prompt>> {
        let mut sql = String::from("SELECT id, name, content, tags, source, created_at FROM prompts WHERE 1=1");
        let mut args: Vec<String> = Vec::new();
        if let Some(t) = tag_filter {
            sql.push_str(" AND tags LIKE ?");
            args.push(format!("%{}%", t));
        }
        if let Some(s) = source_filter {
            sql.push_str(" AND source LIKE ?");
            args.push(format!("%{}%", s));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        args.push(limit.to_string());
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args), row_to_prompt)?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn find_prompts(&self, query: &str, limit: u32) -> Result<Vec<Prompt>> {
        // sanitize FTS5 query: wrap tokens in quotes
        let safe = sanitize_fts(query);
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.content, p.tags, p.source, p.created_at
             FROM prompts_fts JOIN prompts p ON p.id = prompts_fts.rowid
             WHERE prompts_fts MATCH ?
             ORDER BY rank LIMIT ?",
        )?;
        let rows = stmt.query_map(params![safe, limit], row_to_prompt)?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn get_prompt(&self, name: &str) -> Result<Option<Prompt>> {
        // exact match first; if none, try prefix match (unique shortest prefix wins)
        let mut stmt = self.conn.prepare(
            "SELECT id, name, content, tags, source, created_at FROM prompts WHERE name = ?",
        )?;
        let exact: Option<Prompt> = match stmt.query_row(params![name], row_to_prompt) {
            Ok(p) => Some(p),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(e.into()),
        };
        if let Some(p) = exact {
            return Ok(Some(p));
        }
        drop(stmt);
        // prefix match
        let like = format!("{}%", name);
        let like2 = format!("{}-%", name);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, content, tags, source, created_at FROM prompts WHERE name LIKE ? OR name LIKE ? ORDER BY name LIMIT 10",
        )?;
        let rows = stmt.query_map(rusqlite::params_from_iter([like, like2]), row_to_prompt)?;
        let matches: Vec<Prompt> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.into_iter().next()),
            _ => {
                let names: Vec<String> = matches.iter().map(|p| p.name.clone()).collect();
                anyhow::bail!("ambiguous prefix {:?}; matches: {}", name, names.join(", "))
            }
        }
    }

    pub fn rm_prompt(&mut self, name: &str) -> Result<bool> {
        // exact match first, then prefix match
        let n = self.conn.execute("DELETE FROM prompts WHERE name = ?", params![name])?;
        if n > 0 { return Ok(true); }
        let like = format!("{}%", name);
        let like2 = format!("{}-%", name);
        // delete only if exactly one prefix match (avoid accidental mass delete)
        let mut stmt = self.conn.prepare(
            "SELECT name FROM prompts WHERE name LIKE ? OR name LIKE ? ORDER BY name",
        )?;
        let names: Vec<String> = stmt.query_map(params![like, like2], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        match names.len() {
            0 => Ok(false),
            1 => {
                let n = self.conn.execute("DELETE FROM prompts WHERE name = ?", params![names[0]])?;
                Ok(n > 0)
            }
            _ => anyhow::bail!("ambiguous prefix {:?}; matches: {}", name, names.join(", "))
        }
    }
}

fn row_to_prompt(r: &rusqlite::Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: r.get(0)?,
        name: r.get(1)?,
        content: r.get(2)?,
        tags: r.get(3)?,
        source: r.get(4)?,
        created_at: r.get(5)?,
    })
}

fn sanitize_fts(q: &str) -> String {
    let mut out = String::new();
    let mut in_token = false;
    for c in q.chars() {
        match c {
            '"' => { out.push_str("\"\""); }
            ' ' | '\t' | '\n' if in_token => {
                out.push('"'); out.push(c); in_token = false;
            }
            ' ' | '\t' | '\n' => { out.push(c); }
            _ if !in_token => { out.push('"'); out.push(c); in_token = true; }
            _ => { out.push(c); }
        }
    }
    if in_token { out.push('"'); }
    if out.is_empty() { out.push_str("\"\""); }
    out
}

// OptionalExtension is a trait, used elsewhere; we don't need it here.
use rusqlite::OptionalExtension;