# Search

VFS provides powerful search capabilities for finding files by name, content, and attributes.

## Full-Text Search

VFS uses SQLite FTS5 for fast full-text search across all file contents.

### Basic Search

```bash
avfs search <query>
```

**Examples:**

```bash
$ avfs search "database connection"
/src/db.rs:15: let connection = Database::connect(url)?;
/docs/setup.md:42: Configure your database connection string...
/config/example.yaml:8: database_connection: postgres://...

$ avfs search "TODO"
/src/main.rs:23: // TODO: implement error handling
/src/lib.rs:89: // TODO: add tests
```

### Search in Directory

```bash
avfs search "query" /path/
```

### Search Options

| Option | Description |
|--------|-------------|
| `--limit <N>` | Limit results |
| `--rebuild` | Rebuild search index |
| `--json` | JSON output |

### FTS5 Query Syntax

VFS supports SQLite FTS5 query syntax for advanced searches:

```bash
# Phrase search
avfs search '"exact phrase"'

# AND (default)
avfs search "database connection"  # finds both words

# OR
avfs search "error OR warning"

# NOT
avfs search "config NOT test"

# Prefix matching
avfs search "data*"  # matches database, datatype, etc.
```

## Grep - Regex Search

The `grep` command provides regex-based searching with more control.

### Basic Grep

```bash
avfs grep <pattern> [path]
```

**Examples:**

```bash
$ avfs grep "fn \w+\(" /src/
/src/main.rs:5:fn main() {
/src/main.rs:12:fn process_args(args: &[String]) -> Result<Config> {
/src/lib.rs:8:fn new() -> Self {
```

### Regex Patterns

VFS uses Rust's regex crate:

```bash
# Character classes
avfs grep "[0-9]+" /data/          # Numbers
avfs grep "[A-Z][a-z]+" /docs/     # Capitalized words

# Quantifiers
avfs grep "colou?r" /docs/         # color or colour

# Anchors
avfs grep "^import" /src/          # Lines starting with import
avfs grep ";\s*$" /src/            # Lines ending with semicolon

# Groups
avfs grep "(TODO|FIXME|HACK)" /src/
```

### Grep Options

| Option | Description |
|--------|-------------|
| `-i, --ignore-case` | Case-insensitive |
| `-n, --line-numbers` | Show line numbers (default) |
| `-l, --files-with-matches` | Only show filenames |
| `-c, --count` | Show match count |
| `--limit <N>` | Limit results |

### Context Lines

```bash
# 3 lines after each match
$ avfs grep -A 3 "fn main" /src/main.rs
/src/main.rs:5:fn main() {
/src/main.rs-6-    let args: Vec<String> = env::args().collect();
/src/main.rs-7-    let config = process_args(&args)?;

# 2 lines before and after
avfs grep -C 2 "error" /logs/app.log
```

## Find - Locate Files

The `find` command locates files by name, attributes, and metadata.

### Find by Name

```bash
avfs find [path] -n <pattern>
```

Uses glob patterns:

```bash
# All text files
avfs find / -n "*.txt"

# Files starting with "test"
avfs find /src -n "test*"

# Single character wildcard
avfs find / -n "file?.txt"  # file1.txt, fileA.txt, etc.
```

### Find by Type

```bash
# Files only
avfs find / -t f

# Directories only
avfs find / -t d
```

### Find by Size

```bash
# Files larger than 10MB
avfs find / --min-size 10485760

# Files smaller than 1KB
avfs find / --max-size 1024
```

### Find by Tag

```bash
avfs find / --tag important
avfs find / --tag work --tag urgent  # Multiple tags
```

### Find by Metadata

```bash
avfs find / --meta author="Jane Doe"
```

### Combining Conditions

```bash
# Large Rust files
avfs find /src -n "*.rs" --min-size 10240

# Files with specific tag
avfs find /docs --tag important
```

## Performance Tips

### Rebuild Index

If search seems slow or incomplete:

```bash
avfs search --rebuild
```

### Search Large Vaults

For very large vaults:

```bash
# Limit results
avfs search --limit 100 "pattern"

# Search specific directory
avfs search "pattern" /relevant/directory/

# Use grep for targeted search
avfs grep "pattern" /specific/file.txt
```

## Output Formats

### Standard Output

```bash
$ avfs grep "error" /logs/
/logs/app.log:15:Error: Connection failed
/logs/app.log:23:error handling exception
```

### JSON Output

```bash
$ avfs grep --json "error" /logs/
{
  "matches": [
    {
      "path": "/logs/app.log",
      "line": 15,
      "content": "Error: Connection failed"
    }
  ]
}
```

## Integration Examples

### Find and Process

```bash
# Find all Python files
avfs find /src -n "*.py" | while read file; do
    echo "Processing $file"
    avfs cat "$file" | wc -l
done
```

### Search with Tags

```bash
# Search only in tagged files
avfs find / --tag code | while read file; do
    avfs grep "TODO" "$file"
done
```

### Export Search Results

```bash
# Create report of all TODOs
avfs grep "TODO|FIXME" /src/ > ~/todos-report.txt
```
