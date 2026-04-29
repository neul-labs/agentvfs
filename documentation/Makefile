.PHONY: install serve build clean help

# Use uv if available (fast, isolated); fall back to pip in a venv otherwise.
UV := $(shell command -v uv 2>/dev/null)

help:
	@echo "Targets:"
	@echo "  make install   Create/update the docs build environment"
	@echo "  make serve     Run the docs locally on http://127.0.0.1:8000 with live reload"
	@echo "  make build     Build the static site into ./site"
	@echo "  make clean     Remove the built site"

install:
ifeq ($(UV),)
	python3 -m venv .venv
	./.venv/bin/pip install -U pip
	./.venv/bin/pip install -r requirements.txt
else
	uv venv .venv
	uv pip install --python .venv/bin/python -r requirements.txt
endif

serve:
	./.venv/bin/mkdocs serve

build:
	./.venv/bin/mkdocs build --strict

clean:
	rm -rf site
