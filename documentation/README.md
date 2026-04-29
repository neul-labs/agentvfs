# AgentVFS Documentation

User-facing documentation for AgentVFS, built with [MkDocs](https://www.mkdocs.org/) + [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).

This subtree is **self-contained**: everything needed to build and serve the docs site lives under `documentation/`. You can develop and deploy it without touching the rust crate.

```
documentation/
├── mkdocs.yml          # site config
├── requirements.txt    # pinned mkdocs + theme versions
├── Makefile            # install / serve / build helpers
├── README.md           # this file
└── docs/               # markdown sources (mkdocs default docs_dir)
    ├── index.md
    ├── getting-started/
    ├── user-guide/
    ├── advanced/
    └── reference/
```

## Local development

```bash
cd documentation
make install   # one-time: create .venv with mkdocs + material
make serve     # http://127.0.0.1:8000, live-reloads on file save
```

## Build the static site

```bash
make build     # outputs ./site (builds with --strict, fails on broken links)
```

The `site/` directory is the deployable artifact. Drop it on any static host (GitHub Pages, Cloudflare Pages, S3, Netlify, etc.).

## Editing pages

Markdown sources live under `docs/`. The nav order is defined in `mkdocs.yml`'s `nav:` block — adding a new page means dropping it under `docs/` and adding the entry there.

Internal links should be relative paths to `.md` files (mkdocs rewrites them at build time):

```markdown
See [the proxy boundary](../advanced/proxy-boundary.md).
```

## Conventions

- One H1 per page (the page title); use H2/H3 for sections.
- Code blocks declare a language for syntax highlighting (` ```bash`, ` ```rust`, ` ```python`).
- Use admonitions (`!!! note`, `!!! warning`) sparingly, for genuine asides.
- Prefer concrete examples over abstract description.
