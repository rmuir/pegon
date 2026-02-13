# make make sane
.DELETE_ON_ERROR:
SHELL := /bin/bash
.SHELLFLAGS := --norc -euo pipefail -c

.PHONY: all
all: ## Create binary
	# create target/release/pegon
	cargo build --release

.PHONY: wheel
wheel: ## Create python package
	# build python package with maturin
	uv build

.PHONY: lint
lint: ## Lint and format sources
	# run checks on all files
	uv run --frozen --only-dev prek --all-files

.PHONY: help
help: ## Display this help screen
	@grep -E '^[a-z.A-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
