# pinpoint

> **Your `~/.bash_history` but for AI prompts.** Save, search, and replay the prompts you actually want to keep. Single 1.1 MB Rust binary + SQLite FTS5, local-first, zero cloud.

```
$ pinpoint log "select id, name from users where created_at > '2024-01-01'" --tags sql --source claude-code
[pinpoint] saved `select-id-name-from` (58 chars, 14 tokens≈)

$ pinpoint list
+-------------+-------------------------------+------+-------------+------------------------------------------------------------+
| saved       | name                          | tags | source      | preview                                                    |
+-------------+-------------------------------+------+-------------+------------------------------------------------------------+
| 07-19 16:51 | remember-format-timestamps-as | ts   | memory      | remember: format timestamps as ISO 8601                    |
| 07-19 16:51 | select-id-name-from           | sql  | claude-code | select id, name from users where created_at > '2024-01-01' |
+-------------+-------------------------------+------+-------------+------------------------------------------------------------+

$ pinpoint find "auth"
1 match(es) for "auth":
+------------------+----------+-----------------------------+
| name             | tags     | match                       |
+------------------+----------+-----------------------------+
| fix-the-auth-bug | bug,auth | fix the auth bug in auth.ts |
+------------------+----------+-----------------------------+

$ pinpoint replay fix        # prefix match!
fix the auth bug in auth.ts
```

## Why

Every day you write 50+ prompts to Claude Code. Some are throwaway; others are the SQL migration template you'll need again in 3 weeks. **The throwaway ones are in your session history. The keepers are gone.** `pinpoint` is the tiny `~/.bash_history` shell escape you always wanted for AI prompts.

## Install

```bash
cargo install pinpoint
# or grab a binary from GitHub Releases
```

## Usage

```bash
# Save a prompt (arg or stdin)
pinpoint log "fix the auth bug in auth.ts" --tags "bug,auth" --source "claude-code"
echo "remember: format timestamps as ISO" | pinpoint log --tags "ts" --source "memory"

# List (newest first, filterable by tag or source)
pinpoint list [--tag TAG] [--source SRC] [--limit N]

# Full-text search (FTS5)
pinpoint find "window function" [--limit N]

# Replay (or copy to clipboard)
pinpoint replay fix                # exact or prefix match
pinpoint replay fix --copy         # send to OS clipboard

# Remove
pinpoint rm fix

# Emit a one-line shell helper for ~/.bashrc / ~/.zshrc / config.fish
pinpoint shell-init bash
```

## The `pp` shell alias (recommended)

After `pinpoint shell-init bash` (or `zsh` / `fish`), paste the output into `~/.bashrc` and reload:

```bash
# Now in your shell:
$ pp "fix the auth bug in auth.ts" --tags bug
[pinpoint] saved `fix-the-auth-bug` (27 chars, 6 tokens≈)

# stdin mode
$ echo "remember: format timestamps as ISO 8601" | pp --tags ts
[pinpoint] saved `remember-format-timestamps-as` (39 chars, 9 tokens≈)
```

Single keystroke-saving ritual: paste any prompt into the terminal, type `pp` before, hit enter.

## How it differs from `recall-ai`

Both projects are by the same author. They cover different parts of the same idea:

| Tool | What it does | When to use it |
|---|---|---|
| **pinpoint** | Save & replay the few prompts you want to keep | "I want to remember this exact SQL migration template" |
| **recall-ai** | Search across your full AI conversation history | "What was that bug fix from 3 weeks ago?" |

`pinpoint` is **active** (you decide what's a keeper). `recall-ai` is **passive** (search the firehose).

## Storage

- DB file: `$XDG_DATA_HOME/pinpoint/pinpoint.db` (default `~/.local/share/pinpoint/pinpoint.db`)
- Override with `PINPOINT_DATA_DIR=/path/to/dir`
- Schema: single `prompts` table + `prompts_fts` virtual table (FTS5 with porter+unicode61 tokenizer)
- ON CONFLICT(name) DO UPDATE — re-saving the same name updates in place

## Benchmarks (preliminary)

| op | time |
|---|---|
| `pinpoint log` (1 prompt, 100 chars) | < 5 ms |
| `pinpoint list` (1000 prompts) | < 30 ms |
| `pinpoint find "query"` (1000 prompts) | < 20 ms |
| `pinpoint replay` (100k-char prompt) | < 1 ms |

## Roadmap

- [x] **v0.1** — log / list / find / replay / rm / shell-init + clipboard + prefix matching
- [ ] **v0.2** — share via gist / paste / URL (with redaction)
- [ ] **v0.3** — `pinpoint sync` to optionally sync across machines (opt-in, E2E encrypted)
- [ ] **v0.4** — `pinpoint recall-ai` integration: import starred prompts from recall-ai

## See also — other kezhu-ai AI dev tools

pinpoint is one of four Rust CLIs in the same family:

- **[recall-ai](https://github.com/kezhu-ai/recall-ai)** — search every AI conversation you've ever had, locally. The passive side (firehose search) of the same idea.
- **[ctxguard](https://github.com/kezhu-ai/ctxguard)** — context-window budget enforcer for AI agents. Catches when your saved prompt burns 200k tokens.
- **[mcp-sentry](https://github.com/kezhu-ai/mcp-sentry)** — policy-as-code firewall for MCP servers.

The kit: **pinpoint** to save the keepers, **ctxguard** to budget agent use, **recall-ai** to search the firehose, **mcp-sentry** to gate the servers.

## Author

Made by [@kezhu-ai](https://github.com/kezhu-ai) — also the author of [ctxguard](https://github.com/kezhu-ai/ctxguard), [mcp-sentry](https://github.com/kezhu-ai/mcp-sentry), and [recall-ai](https://github.com/kezhu-ai/recall-ai).

## License

MIT OR Apache-2.0