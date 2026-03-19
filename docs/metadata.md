# Metadata & Tags

avfs supports rich metadata for organizing and finding files through tags and custom key-value attributes.

## Tags

Tags are labels you can attach to files for organization and quick retrieval.

### Adding Tags

```bash
avfs tag <path> <tag>...
```

**Examples:**

```bash
# Single tag
avfs tag /docs/report.pdf important

# Multiple tags
avfs tag /docs/report.pdf important urgent work

# Tag with glob pattern
avfs tag /src/*.rs code rust

# Tag files recursively
avfs tag -r /project/ work
```

### Listing Tags

#### Tags on a file

```bash
$ avfs tag /docs/report.pdf
important, urgent, work
```

#### All tags in vault

```bash
$ avfs tag --list
TAG          COUNT
important    23
urgent       8
work         156
personal     45
archive      89
code         234
rust         78
```

### Removing Tags

```bash
avfs untag <path> <tag>...
```

**Examples:**

```bash
# Remove single tag
avfs untag /docs/report.pdf urgent

# Remove multiple tags
avfs untag /docs/report.pdf work important

# Remove from pattern
avfs untag /archive/*.txt important
```

### Finding Files by Tag

```bash
avfs find / -tag <tag>
```

**Examples:**

```bash
$ avfs find / -tag important
/docs/report.pdf
/config/production.yaml
/notes/meeting-2024-03-10.md

# Multiple tags (AND)
$ avfs find / -tag important -tag urgent
/docs/report.pdf

# Combine with other filters
$ avfs find /docs -tag work -name "*.pdf"
/docs/report.pdf
/docs/presentation.pdf
```

### Tag Colors

Tags can have colors for visual organization:

```bash
avfs tag --create important --color red
avfs tag --create work --color blue
avfs tag --create personal --color green
```

Colors are displayed in `avfs ls` output and interactive shell.

### Renaming Tags

```bash
avfs tag --rename old-tag new-tag
```

Updates the tag name on all associated files.

### Deleting Tags

```bash
avfs tag --delete unused-tag
```

Removes the tag from all files and deletes the tag definition.

## Custom Metadata

Beyond tags, you can store arbitrary key-value metadata on files.

### Setting Metadata

```bash
avfs meta <path> <key> <value>
```

**Examples:**

```bash
# Set author
avfs meta /docs/report.pdf author "Jane Doe"

# Set multiple properties
avfs meta /docs/report.pdf department "Engineering"
avfs meta /docs/report.pdf status "draft"
avfs meta /docs/report.pdf priority "high"

# Numeric values
avfs meta /data/dataset.csv row_count "10000"

# Date values
avfs meta /docs/contract.pdf signed_date "2024-03-15"
```

### Reading Metadata

```bash
# Get specific key
$ avfs meta /docs/report.pdf author
Jane Doe

# Get all metadata
$ avfs meta /docs/report.pdf
KEY          VALUE
author       Jane Doe
department   Engineering
status       draft
priority     high
```

### Removing Metadata

```bash
avfs meta --unset <path> <key>
```

**Example:**

```bash
avfs meta --unset /docs/report.pdf status
```

### Finding by Metadata

```bash
avfs find / -meta <key>=<value>
```

**Examples:**

```bash
# Find by author
$ avfs find / -meta author="Jane Doe"
/docs/report.pdf
/docs/presentation.pdf

# Find by status
$ avfs find / -meta status=draft
/docs/report.pdf
/src/feature.rs

# Combine with other filters
$ avfs find /docs -meta department=Engineering -name "*.pdf"
```

### Metadata Operators

```bash
# Equals
avfs find / -meta priority=high

# Not equals
avfs find / -meta status!=draft

# Contains (for string values)
avfs find / -meta author~=Jane

# Greater/less than (for numeric values)
avfs find / -meta row_count>1000
avfs find / -meta file_size<1048576
```

## Integration with Search

### Search with Tag Filter

```bash
avfs grep "TODO" / --tag code
```

Only searches files with the "code" tag.

### Search with Metadata Filter

```bash
avfs grep "bug" / --meta status=active
```

## Metadata in ls Output

### Long Format with Tags

```bash
$ avfs ls -l --tags /docs/
TYPE  SIZE     MODIFIED             TAGS                    NAME
f     125 KB   2024-03-10 14:22     important, work         report.pdf
f     45 KB    2024-03-09 10:15     work                    notes.txt
d     -        2024-03-08 09:30                             archive/
```

### Long Format with Metadata

```bash
$ avfs ls -l --meta /docs/
TYPE  SIZE     AUTHOR      STATUS  NAME
f     125 KB   Jane Doe    draft   report.pdf
f     45 KB    John Smith  final   notes.txt
```

Custom metadata columns with `--meta`:

```bash
avfs ls -l --meta=author,status,priority /docs/
```

## Bulk Operations

### Tag Multiple Files

```bash
# From find results
avfs find / -name "*.log" -exec avfs tag {} archive

# Using xargs
avfs find / -size +10M | xargs -I {} avfs tag {} large-file
```

### Copy Metadata

```bash
# Copy all tags from one file to another
avfs tag --copy /docs/template.pdf /docs/new-doc.pdf

# Copy specific metadata
avfs meta --copy /docs/template.pdf /docs/new-doc.pdf author department
```

### Export/Import Metadata

```bash
# Export metadata to JSON
avfs meta --export /docs/ > metadata.json

# Import metadata from JSON
avfs meta --import metadata.json
```

**JSON format:**

```json
{
  "/docs/report.pdf": {
    "tags": ["important", "work"],
    "metadata": {
      "author": "Jane Doe",
      "status": "draft"
    }
  }
}
```

## Reserved Metadata Keys

Some metadata keys are reserved for system use:

| Key | Description |
|-----|-------------|
| `_size` | File size (read-only) |
| `_created` | Creation timestamp (read-only) |
| `_modified` | Last modification (read-only) |
| `_type` | File type (read-only) |
| `_versions` | Version count (read-only) |
| `_hash` | Content hash (read-only) |

Access with:

```bash
$ avfs meta /docs/file.txt _size
1234

$ avfs meta /docs/file.txt _modified
2024-03-10T14:22:15Z
```

## Use Cases

### Project Organization

```bash
# Set up project tags
avfs tag --create frontend --color blue
avfs tag --create backend --color green
avfs tag --create docs --color yellow

# Tag project files
avfs tag /src/ui/*.tsx frontend
avfs tag /src/api/*.rs backend
avfs tag /docs/*.md docs

# Find all frontend code
avfs find / -tag frontend
```

### Document Management

```bash
# Track document metadata
avfs meta /contracts/client-a.pdf client "Acme Corp"
avfs meta /contracts/client-a.pdf value "50000"
avfs meta /contracts/client-a.pdf expires "2025-12-31"

# Find expiring contracts
avfs find /contracts -meta expires<"2024-06-01"
```

### Asset Pipeline

```bash
# Track image metadata
avfs meta /assets/hero.png width "1920"
avfs meta /assets/hero.png height "1080"
avfs meta /assets/hero.png format "png"

# Find high-res images
avfs find /assets -meta width>1000
```

### Research Notes

```bash
# Organize research
avfs tag /notes/paper-1.md read cite
avfs meta /notes/paper-1.md source "arxiv:2024.12345"
avfs meta /notes/paper-1.md relevance "high"

# Find papers to cite
avfs find /notes -tag cite -meta relevance=high
```
