# Metadata & Tags

VFS supports rich metadata for organizing and finding files through tags and custom key-value attributes.

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
```

### Listing Tags

#### Tags on a file

```bash
$ avfs tag /docs/report.pdf --list
important, urgent, work
```

#### All tags in vault

```bash
$ avfs tag --list-all
TAG          COUNT
important    23
urgent       8
work         156
personal     45
code         234
```

### Removing Tags

```bash
avfs untag <path> <tag>
```

**Examples:**

```bash
# Remove single tag
avfs untag /docs/report.pdf urgent

# Remove from pattern
avfs untag /archive/*.txt important
```

### Finding Files by Tag

```bash
avfs find / --tag <tag>
```

**Examples:**

```bash
$ avfs find / --tag important
/docs/report.pdf
/config/production.yaml
/notes/meeting-2024-03-10.md

# Multiple tags (AND)
$ avfs find / --tag important --tag urgent
/docs/report.pdf
```

### Managing Tags

```bash
# Create a new tag
avfs tag --create important

# Rename a tag
avfs tag --rename old-tag new-tag

# Delete a tag (removes from all files)
avfs tag --delete unused-tag
```

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
avfs meta /docs/report.pdf --delete status
```

### Finding by Metadata

```bash
avfs find / --meta <key>=<value>
```

**Examples:**

```bash
# Find by author
$ avfs find / --meta author="Jane Doe"
/docs/report.pdf
/docs/presentation.pdf

# Find by status
$ avfs find / --meta status=draft
/docs/report.pdf
/src/feature.rs
```

## Export/Import Metadata

```bash
# Export metadata to JSON
avfs meta /docs/ --export > metadata.json

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

## Use Cases

### Project Organization

```bash
# Set up project tags
avfs tag --create frontend
avfs tag --create backend
avfs tag --create docs

# Tag project files
avfs tag /src/ui/*.tsx frontend
avfs tag /src/api/*.rs backend
avfs tag /docs/*.md docs

# Find all frontend code
avfs find / --tag frontend
```

### Document Management

```bash
# Track document metadata
avfs meta /contracts/client-a.pdf client "Acme Corp"
avfs meta /contracts/client-a.pdf value "50000"
avfs meta /contracts/client-a.pdf expires "2025-12-31"
```

### Research Notes

```bash
# Organize research
avfs tag /notes/paper-1.md read cite
avfs meta /notes/paper-1.md source "arxiv:2024.12345"
avfs meta /notes/paper-1.md relevance "high"

# Find papers to cite
avfs find /notes --tag cite
```
