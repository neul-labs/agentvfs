# Search & Grep

vfs provides powerful search capabilities for finding files by name, content, and attributes.

## Full-Text Search

vfs uses SQLite FTS5 for fast full-text search across all file contents.

### Basic Search

```bash
vfs search <query>
```

**Examples:**

```bash
$ vfs search "database connection"
/src/db.rs:15: let connection = Database::connect(url)?;
/docs/setup.md:42: Configure your database connection string...
/config/example.yaml:8: database_connection: postgres://...

$ vfs search "TODO"
/src/main.rs:23: // TODO: implement error handling
/src/lib.rs:89: // TODO: add tests
/notes/tasks.md:5: - TODO: review PR #123
```

### Search in Directory

```bash
vfs search "query" /path/
```

**Examples:**

```bash
vfs search "import" /src/
vfs search "config" /docs/
```

### Search Options

| Option | Description |
|--------|-------------|
| `-i, --ignore-case` | Case-insensitive search |
| `-w, --word` | Match whole words only |
| `-l, --files-only` | Show only filenames |
| `-c, --count` | Show match counts |
| `--limit <N>` | Limit results |
| `--json` | JSON output |

**Examples:**

```bash
# Case insensitive
vfs search -i "error"

# Whole words only (won't match "errors")
vfs search -w "error"

# Just filenames
vfs search -l "TODO"

# Count matches
$ vfs search -c "TODO"
/src/main.rs: 3
/src/lib.rs: 7
/notes/tasks.md: 12
Total: 22 matches in 3 files
```

### FTS5 Query Syntax

vfs supports SQLite FTS5 query syntax for advanced searches:

```bash
# Phrase search
vfs search '"exact phrase"'

# AND (default)
vfs search "database connection"  # finds both words

# OR
vfs search "error OR warning"

# NOT
vfs search "config NOT test"

# Prefix matching
vfs search "data*"  # matches database, datatype, etc.

# NEAR (words within N tokens)
vfs search "NEAR(error handling, 5)"

# Column filtering (content only, not path)
vfs search "content:database"
```

## Grep

The `grep` command provides regex-based searching with more control over output format.

### Basic Grep

```bash
vfs grep <pattern> [path]
```

**Examples:**

```bash
$ vfs grep "fn \w+\(" /src/
/src/main.rs:5:fn main() {
/src/main.rs:12:fn process_args(args: &[String]) -> Result<Config> {
/src/lib.rs:8:fn new() -> Self {
```

### Regex Patterns

vfs uses Rust's regex crate, supporting:

```bash
# Character classes
vfs grep "[0-9]+" /data/          # Numbers
vfs grep "[A-Z][a-z]+" /docs/     # Capitalized words

# Quantifiers
vfs grep "colou?r" /docs/         # color or colour
vfs grep "go+gle" /notes/         # gogle, google, gooogle...

# Anchors
vfs grep "^import" /src/          # Lines starting with import
vfs grep ";\s*$" /src/            # Lines ending with semicolon

# Groups
vfs grep "(TODO|FIXME|HACK)" /src/

# Lookahead/lookbehind (advanced)
vfs grep "(?<=fn )\w+" /src/      # Function names after 'fn '
```

### Grep Options

| Option | Description |
|--------|-------------|
| `-i, --ignore-case` | Case-insensitive |
| `-r, --recursive` | Search recursively (default) |
| `-l, --files-with-matches` | Only show filenames |
| `-L, --files-without-match` | Files not matching |
| `-c, --count` | Show match count |
| `-n, --line-number` | Show line numbers (default) |
| `-v, --invert-match` | Non-matching lines |
| `-w, --word-regexp` | Match whole words |
| `-x, --line-regexp` | Match whole lines |
| `-A <N>` | Show N lines after |
| `-B <N>` | Show N lines before |
| `-C <N>` | Show N lines context |
| `--color` | Colorize output |
| `-o, --only-matching` | Show only matched part |

### Context Lines

```bash
# 3 lines after each match
$ vfs grep -A 3 "fn main" /src/main.rs
/src/main.rs:5:fn main() {
/src/main.rs-6-    let args: Vec<String> = env::args().collect();
/src/main.rs-7-    let config = process_args(&args)?;
/src/main.rs-8-    run(config)?;

# 2 lines before and after
$ vfs grep -C 2 "error" /logs/app.log
```

### Invert Match

Find lines NOT matching a pattern:

```bash
# Lines without comments
vfs grep -v "^\s*//" /src/main.rs

# Files without TODO
vfs grep -L "TODO" /src/
```

### Only Matching

Extract just the matched text:

```bash
$ vfs grep -o "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}" /contacts/
john@example.com
jane.doe@company.org
support@service.io
```

## Find

The `find` command locates files by name, attributes, and metadata.

### Find by Name

```bash
vfs find [path] -name <pattern>
```

Uses glob patterns (not regex):

```bash
# All text files
vfs find / -name "*.txt"

# Files starting with "test"
vfs find /src -name "test*"

# Single character wildcard
vfs find / -name "file?.txt"  # file1.txt, fileA.txt, etc.

# Character class
vfs find / -name "log[0-9].txt"  # log0.txt through log9.txt
```

### Find by Type

```bash
# Files only
vfs find / -type f

# Directories only
vfs find / -type d
```

### Find by Size

```bash
# Files larger than 10MB
vfs find / -size +10M

# Files smaller than 1KB
vfs find / -size -1K

# Exact size (rarely useful)
vfs find / -size 1024

# Size units: B, K, M, G
```

### Find by Time

```bash
# Modified in last 7 days
vfs find / -mtime -7

# Modified more than 30 days ago
vfs find / -mtime +30

# Modified exactly 1 day ago
vfs find / -mtime 1
```

### Find by Tag

```bash
vfs find / -tag important
vfs find / -tag work -tag urgent  # AND
```

### Find by Metadata

```bash
vfs find / -meta author="Jane Doe"
vfs find / -meta status!=draft
vfs find / -meta priority>5
```

### Combining Conditions

```bash
# Large Rust files
vfs find /src -name "*.rs" -size +10K

# Recent important documents
vfs find /docs -tag important -mtime -7

# Old log files
vfs find /logs -name "*.log" -mtime +30
```

### Execute on Results

```bash
# Delete old logs
vfs find /logs -name "*.log" -mtime +30 -exec rm {}

# Tag large files
vfs find / -size +100M -exec vfs tag {} large-file

# Grep in found files
vfs find /src -name "*.rs" -exec vfs grep "TODO" {}
```

## Performance Tips

### Index Status

Check FTS index status:

```bash
$ vfs search --status
FTS Index Status:
  Indexed files: 1,234
  Index size: 8.5 MB
  Last rebuild: 2024-03-10 10:30:00
```

### Rebuild Index

If search seems slow or incomplete:

```bash
vfs search --rebuild
```

### Search Large Vaults

For very large vaults:

```bash
# Limit results
vfs search --limit 100 "pattern"

# Search specific directory
vfs search "pattern" /relevant/directory/

# Use grep for targeted search
vfs grep "pattern" /specific/file.txt
```

### Exclude Paths

```bash
# Exclude directories
vfs grep "TODO" / --exclude "/vendor/*" --exclude "/node_modules/*"

# Or use find piped to grep
vfs find /src -name "*.rs" | xargs -I {} vfs grep "TODO" {}
```

## Output Formats

### Standard Output

```bash
$ vfs grep "error" /logs/
/logs/app.log:15:Error: Connection failed
/logs/app.log:23:error handling exception
```

### JSON Output

```bash
$ vfs grep --json "error" /logs/
{
  "matches": [
    {
      "path": "/logs/app.log",
      "line": 15,
      "content": "Error: Connection failed",
      "match": "Error"
    }
  ],
  "total": 2,
  "files": 1
}
```

### Quiet Mode

For scripts, just exit code:

```bash
if vfs grep -q "error" /logs/; then
    echo "Errors found!"
fi
```

## Integration Examples

### Find and Replace

```bash
# Find files with old API
vfs grep -l "old_api" /src/ | while read file; do
    vfs exec 'sed -i s/old_api/new_api/g' "$file"
done
```

### Search with Tags

```bash
# Search only in code files
vfs grep "TODO" $(vfs find / -tag code)

# Or using find -exec
vfs find / -tag code -exec vfs grep "TODO" {}
```

### Export Search Results

```bash
# Create report of all TODOs
vfs grep "TODO|FIXME|HACK" /src/ > ~/todos-report.txt

# JSON export
vfs grep --json "TODO" /src/ | jq '.matches[] | "\(.path):\(.line): \(.content)"'
```
