//! Output renderers: list table, find results.

use chrono::{DateTime, Local};
use tabled::{Table, Tabled};

use crate::storage::Prompt;

pub fn print_list(items: &[Prompt]) {
    if items.is_empty() {
        println!("(no saved prompts — `echo \"your prompt\" | pinpoint log` or `pinpoint log \"your prompt\"`)");
        return;
    }
    let rows: Vec<ListRow> = items.iter().map(|p| ListRow {
        when: format_when(&p.created_at),
        name: p.name.clone(),
        tags: if p.tags.is_empty() { "-".into() } else { p.tags.clone() },
        source: if p.source.is_empty() { "-".into() } else { p.source.clone() },
        preview: preview(&p.content, 80),
    }).collect();
    println!("{}", Table::new(rows));
    println!("\n{} saved prompt(s)", items.len());
}

pub fn print_find(items: &[Prompt], query: &str) {
    if items.is_empty() {
        println!("(no matches for \"{}\")", query);
        return;
    }
    println!("{} match(es) for \"{}\":\n", items.len(), query);
    let rows: Vec<FindRow> = items.iter().map(|p| FindRow {
        name: p.name.clone(),
        tags: if p.tags.is_empty() { "-".into() } else { p.tags.clone() },
        preview: preview(&p.content, 100),
    }).collect();
    println!("{}", Table::new(rows));
}

#[derive(Tabled)]
struct ListRow {
    #[tabled(rename = "saved")]
    when: String,
    #[tabled(rename = "name")]
    name: String,
    #[tabled(rename = "tags")]
    tags: String,
    #[tabled(rename = "source")]
    source: String,
    #[tabled(rename = "preview")]
    preview: String,
}

#[derive(Tabled)]
struct FindRow {
    #[tabled(rename = "name")]
    name: String,
    #[tabled(rename = "tags")]
    tags: String,
    #[tabled(rename = "match")]
    preview: String,
}

fn format_when(iso: &str) -> String {
    DateTime::parse_from_rfc3339(iso)
        .map(|d| d.with_timezone(&Local).format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| iso.to_string())
}

fn preview(s: &str, max: usize) -> String {
    let clean: String = s.chars().filter(|c| *c != '\n' && *c != '\r').collect();
    if clean.chars().count() <= max {
        clean
    } else {
        let mut out: String = clean.chars().take(max).collect();
        out.push('…');
        out
    }
}