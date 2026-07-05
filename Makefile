# make make sane
.DELETE_ON_ERROR:
.SHELLFLAGS := --norc -euo pipefail -c
SHELL := /bin/bash

.PHONY: pegon
pegon: ## Create binary
	cargo build --release

.PHONY: wheel
wheel: ## Create python package
	uv build

.PHONY: lint
lint: ## Lint, format, test
	uv run --frozen --only-dev prek --all-files --stage pre-push

.PHONY: bench
bench: ## Run micro-benchmarks
	# run benchmark suite
	cargo bench

.PHONY: profile-queries
profile-queries: ## Profile queries
	ts_query_ls profile

export PERF_CONFIG ?= .perfconfig

.PHONY: profile
profile: ## Profile lint run with perf
	RUSTFLAGS="-C force-frame-pointers=yes" cargo build --profile profiling
	perf record -g target/profiling/pegon check ~/workspace/lucene > out.txt || true
	perf report

export LLVM_COV ?= llvm-cov
export LLVM_PROFDATA ?= llvm-profdata

.PHONY: test
test: ## Run tests with coverage report
	cargo llvm-cov --text
	cargo llvm-cov report --summary-only

.PHONY: help
help: ## Display this help screen
	@grep -E '^[a-z.A-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
