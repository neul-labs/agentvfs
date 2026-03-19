# Metadata & Tags

VFS supports rich metadata for organizing and finding files through tags and custom key-value attributes.

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
```

### Listing Tags

#### Tags on a file

```bash
$ vfs tag /docs/report.pdf --list
important, urgent, work
```

#### All tags in vault

```bash
$ vfs tag --list-all
TAG          COUNT
important    23
urgent       8
work         156
personal     45
code         234
```

### Removing Tags

```bash
vfs untag <path> <tag>
```

**Examples:**

```bash
# Remove single tag
vfs untag /docs/report.pdf urgent

# Remove from pattern
vfs untag /archive/*.txt important
```

### Finding Files by Tag

```bash
vfs find / --tag <tag>
```

**Examples:**

```bash
$ vfs find / --tag important
/docs/report.pdf
/config/production.yaml
/notes/meeting-2024-03-10.md

# Multiple tags (AND)
$ vfs find / --tag important --tag urgent
/docs/report.pdf
```

### Managing Tags

```bash
# Create a new tag
vfs tag --create important

# Rename a tag
vfs tag --rename old-tag new-tag

# Delete a tag (removes from all files)
vfs tag --delete unused-tag
```

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
vfs meta /docs/report.pdf --delete status
```

### Finding by Metadata

```bash
vfs find / --meta <key>=<value>
```

**Examples:**

```bash
# Find by author
$ vfs find / --meta author="Jane Doe"
/docs/report.pdf
/docs/presentation.pdf

# Find by status
$ vfs find / --meta status=draft
/docs/report.pdf
/src/feature.rs
```

## Export/Import Metadata

```bash
# Export metadata to JSON
vfs meta /docs/ --export > metadata.json

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

## Use Cases

### Project Organization

```bash
# Set up project tags
vfs tag --create frontend
vfs tag --create backend
vfs tag --create docs

# Tag project files
vfs tag /src/ui/*.tsx frontend
vfs tag /src/api/*.rs backend
vfs tag /docs/*.md docs

# Find all frontend code
vfs find / --tag frontend
```

### Document Management

```bash
# Track document metadata
vfs meta /contracts/client-a.pdf client "Acme Corp"
vfs meta /contracts/client-a.pdf value "50000"
vfs meta /contracts/client-a.pdf expires "2025-12-31"
```

### Research Notes

```bash
# Organize research
vfs tag /notes/paper-1.md read cite
vfs meta /notes/paper-1.md source "arxiv:2024.12345"
vfs meta /notes/paper-1.md relevance "high"

# Find papers to cite
vfs find /notes --tag cite
```
