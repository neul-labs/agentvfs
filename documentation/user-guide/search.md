# Search

VFS provides powerful search capabilities for finding files by name, content, and attributes.

## Full-Text Search

VFS uses SQLite FTS5 for fast full-text search across all file contents.

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
```

### Search in Directory

```bash
vfs search "query" /path/
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
vfs search '"exact phrase"'

# AND (default)
vfs search "database connection"  # finds both words

# OR
vfs search "error OR warning"

# NOT
vfs search "config NOT test"

# Prefix matching
vfs search "data*"  # matches database, datatype, etc.
```

## Grep - Regex Search

The `grep` command provides regex-based searching with more control.

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

VFS uses Rust's regex crate:

```bash
# Character classes
vfs grep "[0-9]+" /data/          # Numbers
vfs grep "[A-Z][a-z]+" /docs/     # Capitalized words

# Quantifiers
vfs grep "colou?r" /docs/         # color or colour

# Anchors
vfs grep "^import" /src/          # Lines starting with import
vfs grep ";\s*$" /src/            # Lines ending with semicolon

# Groups
vfs grep "(TODO|FIXME|HACK)" /src/
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
$ vfs grep -A 3 "fn main" /src/main.rs
/src/main.rs:5:fn main() {
/src/main.rs-6-    let args: Vec<String> = env::args().collect();
/src/main.rs-7-    let config = process_args(&args)?;

# 2 lines before and after
vfs grep -C 2 "error" /logs/app.log
```

## Find - Locate Files

The `find` command locates files by name, attributes, and metadata.

### Find by Name

```bash
vfs find [path] -n <pattern>
```

Uses glob patterns:

```bash
# All text files
vfs find / -n "*.txt"

# Files starting with "test"
vfs find /src -n "test*"

# Single character wildcard
vfs find / -n "file?.txt"  # file1.txt, fileA.txt, etc.
```

### Find by Type

```bash
# Files only
vfs find / -t f

# Directories only
vfs find / -t d
```

### Find by Size

```bash
# Files larger than 10MB
vfs find / --min-size 10485760

# Files smaller than 1KB
vfs find / --max-size 1024
```

### Find by Tag

```bash
vfs find / --tag important
vfs find / --tag work --tag urgent  # Multiple tags
```

### Find by Metadata

```bash
vfs find / --meta author="Jane Doe"
```

### Combining Conditions

```bash
# Large Rust files
vfs find /src -n "*.rs" --min-size 10240

# Files with specific tag
vfs find /docs --tag important
```

## Performance Tips

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
      "content": "Error: Connection failed"
    }
  ]
}
```

## Integration Examples

### Find and Process

```bash
# Find all Python files
vfs find /src -n "*.py" | while read file; do
    echo "Processing $file"
    vfs cat "$file" | wc -l
done
```

### Search with Tags

```bash
# Search only in tagged files
vfs find / --tag code | while read file; do
    vfs grep "TODO" "$file"
done
```

### Export Search Results

```bash
# Create report of all TODOs
vfs grep "TODO|FIXME" /src/ > ~/todos-report.txt
```
