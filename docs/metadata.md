# Metadata & Tags

vfs supports rich metadata for organizing and finding files through tags and custom key-value attributes.

## Tags

Tags are labels you can attach to files for organization and quick retrieval.

### Adding Tags

```bash
vfs tag <path> <tag>...
```

**Examples:**

```bash
# Single tag
vfs tag /docs/report.pdf important

# Multiple tags
vfs tag /docs/report.pdf important urgent work

# Tag with glob pattern
vfs tag /src/*.rs code rust

# Tag files recursively
vfs tag -r /project/ work
```

### Listing Tags

#### Tags on a file

```bash
$ vfs tag /docs/report.pdf
important, urgent, work
```

#### All tags in vault

```bash
$ vfs tag --list
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
vfs untag <path> <tag>...
```

**Examples:**

```bash
# Remove single tag
vfs untag /docs/report.pdf urgent

# Remove multiple tags
vfs untag /docs/report.pdf work important

# Remove from pattern
vfs untag /archive/*.txt important
```

### Finding Files by Tag

```bash
vfs find / -tag <tag>
```

**Examples:**

```bash
$ vfs find / -tag important
/docs/report.pdf
/config/production.yaml
/notes/meeting-2024-03-10.md

# Multiple tags (AND)
$ vfs find / -tag important -tag urgent
/docs/report.pdf

# Combine with other filters
$ vfs find /docs -tag work -name "*.pdf"
/docs/report.pdf
/docs/presentation.pdf
```

### Tag Colors

Tags can have colors for visual organization:

```bash
vfs tag --create important --color red
vfs tag --create work --color blue
vfs tag --create personal --color green
```

Colors are displayed in `vfs ls` output and interactive shell.

### Renaming Tags

```bash
vfs tag --rename old-tag new-tag
```

Updates the tag name on all associated files.

### Deleting Tags

```bash
vfs tag --delete unused-tag
```

Removes the tag from all files and deletes the tag definition.

## Custom Metadata

Beyond tags, you can store arbitrary key-value metadata on files.

### Setting Metadata

```bash
vfs meta <path> <key> <value>
```

**Examples:**

```bash
# Set author
vfs meta /docs/report.pdf author "Jane Doe"

# Set multiple properties
vfs meta /docs/report.pdf department "Engineering"
vfs meta /docs/report.pdf status "draft"
vfs meta /docs/report.pdf priority "high"

# Numeric values
vfs meta /data/dataset.csv row_count "10000"

# Date values
vfs meta /docs/contract.pdf signed_date "2024-03-15"
```

### Reading Metadata

```bash
# Get specific key
$ vfs meta /docs/report.pdf author
Jane Doe

# Get all metadata
$ vfs meta /docs/report.pdf
KEY          VALUE
author       Jane Doe
department   Engineering
status       draft
priority     high
```

### Removing Metadata

```bash
vfs meta --unset <path> <key>
```

**Example:**

```bash
vfs meta --unset /docs/report.pdf status
```

### Finding by Metadata

```bash
vfs find / -meta <key>=<value>
```

**Examples:**

```bash
# Find by author
$ vfs find / -meta author="Jane Doe"
/docs/report.pdf
/docs/presentation.pdf

# Find by status
$ vfs find / -meta status=draft
/docs/report.pdf
/src/feature.rs

# Combine with other filters
$ vfs find /docs -meta department=Engineering -name "*.pdf"
```

### Metadata Operators

```bash
# Equals
vfs find / -meta priority=high

# Not equals
vfs find / -meta status!=draft

# Contains (for string values)
vfs find / -meta author~=Jane

# Greater/less than (for numeric values)
vfs find / -meta row_count>1000
vfs find / -meta file_size<1048576
```

## Integration with Search

### Search with Tag Filter

```bash
vfs grep "TODO" / --tag code
```

Only searches files with the "code" tag.

### Search with Metadata Filter

```bash
vfs grep "bug" / --meta status=active
```

## Metadata in ls Output

### Long Format with Tags

```bash
$ vfs ls -l --tags /docs/
TYPE  SIZE     MODIFIED             TAGS                    NAME
f     125 KB   2024-03-10 14:22     important, work         report.pdf
f     45 KB    2024-03-09 10:15     work                    notes.txt
d     -        2024-03-08 09:30                             archive/
```

### Long Format with Metadata

```bash
$ vfs ls -l --meta /docs/
TYPE  SIZE     AUTHOR      STATUS  NAME
f     125 KB   Jane Doe    draft   report.pdf
f     45 KB    John Smith  final   notes.txt
```

Custom metadata columns with `--meta`:

```bash
vfs ls -l --meta=author,status,priority /docs/
```

## Bulk Operations

### Tag Multiple Files

```bash
# From find results
vfs find / -name "*.log" -exec vfs tag {} archive

# Using xargs
vfs find / -size +10M | xargs -I {} vfs tag {} large-file
```

### Copy Metadata

```bash
# Copy all tags from one file to another
vfs tag --copy /docs/template.pdf /docs/new-doc.pdf

# Copy specific metadata
vfs meta --copy /docs/template.pdf /docs/new-doc.pdf author department
```

### Export/Import Metadata

```bash
# Export metadata to JSON
vfs meta --export /docs/ > metadata.json

# Import metadata from JSON
vfs meta --import metadata.json
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
$ vfs meta /docs/file.txt _size
1234

$ vfs meta /docs/file.txt _modified
2024-03-10T14:22:15Z
```

## Use Cases

### Project Organization

```bash
# Set up project tags
vfs tag --create frontend --color blue
vfs tag --create backend --color green
vfs tag --create docs --color yellow

# Tag project files
vfs tag /src/ui/*.tsx frontend
vfs tag /src/api/*.rs backend
vfs tag /docs/*.md docs

# Find all frontend code
vfs find / -tag frontend
```

### Document Management

```bash
# Track document metadata
vfs meta /contracts/client-a.pdf client "Acme Corp"
vfs meta /contracts/client-a.pdf value "50000"
vfs meta /contracts/client-a.pdf expires "2025-12-31"

# Find expiring contracts
vfs find /contracts -meta expires<"2024-06-01"
```

### Asset Pipeline

```bash
# Track image metadata
vfs meta /assets/hero.png width "1920"
vfs meta /assets/hero.png height "1080"
vfs meta /assets/hero.png format "png"

# Find high-res images
vfs find /assets -meta width>1000
```

### Research Notes

```bash
# Organize research
vfs tag /notes/paper-1.md read cite
vfs meta /notes/paper-1.md source "arxiv:2024.12345"
vfs meta /notes/paper-1.md relevance "high"

# Find papers to cite
vfs find /notes -tag cite -meta relevance=high
```
