SHELL := /bin/bash

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
DRAFT_DIR := $(ROOT)/drafts
OUTPUT_DIR := $(ROOT)/output
IMAGE_DIR := $(OUTPUT_DIR)/images
DIST_DIR := $(ROOT)/dist
TEMPLATE := $(ROOT)/templates/eisvogel.latex
SCRIPT_DIR := $(ROOT)/scripts
MERMAID_SCRIPT := $(SCRIPT_DIR)/mermaid.py
PUPPETEER_CONFIG ?= $(SCRIPT_DIR)/puppeteer.json

PANDOC ?= pandoc
LATEX ?= lualatex
MMDC ?= mmdc
PYTHON ?= python3

TITLE ?= Remlソースコード完全解説
AUTHOR ?= Reml Project
LANG ?= ja
DATE ?=

DRAFTS := $(shell find $(DRAFT_DIR) -maxdepth 1 -type f -name "*.md" ! -name "README.md" | sort)

.PHONY: all setup images concat pdf clean

all: pdf

setup:
	@mkdir -p $(OUTPUT_DIR) $(IMAGE_DIR) $(DIST_DIR)
	@command -v $(PANDOC) >/dev/null || (echo "pandoc not found: $(PANDOC)" && exit 1)
	@command -v $(LATEX) >/dev/null || (echo "lualatex not found: $(LATEX)" && exit 1)
	@command -v $(MMDC) >/dev/null || (echo "mmdc not found: $(MMDC)" && exit 1)
	@test -f $(TEMPLATE) || (echo "template not found: $(TEMPLATE)" && exit 1)

images: setup
	@$(PYTHON) $(MERMAID_SCRIPT) images \
		--mmdc $(MMDC) \
		--out-dir $(IMAGE_DIR) \
		--puppeteer-config $(PUPPETEER_CONFIG) \
		--drafts $(DRAFTS)

concat: setup
	@$(PYTHON) $(MERMAID_SCRIPT) concat \
		--out-file $(OUTPUT_DIR)/full_draft.md \
		--image-rel images \
		--title "$(TITLE)" \
		--author "$(AUTHOR)" \
		--lang "$(LANG)" \
		--date "$(DATE)" \
		--drafts $(DRAFTS)

pdf: images concat
	@$(PANDOC) $(OUTPUT_DIR)/full_draft.md \
		--pdf-engine=$(LATEX) \
		--template=$(TEMPLATE) \
		--filter=pandoc-crossref \
		--top-level-division=chapter \
		--toc \
		--resource-path=$(ROOT):$(OUTPUT_DIR):$(IMAGE_DIR) \
		--syntax-highlighting=monochrome \
		-V documentclass=bxjsbook \
		-V classoption=pandoc \
		-o $(DIST_DIR)/reml-commentary.pdf

clean:
	@rm -rf $(OUTPUT_DIR) $(DIST_DIR)
