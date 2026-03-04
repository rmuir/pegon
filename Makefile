# make make sane
.DELETE_ON_ERROR:
SHELL := /bin/bash
.SHELLFLAGS := --norc -euo pipefail -c

.PHONY: pegon
pegon: ## Create binary
	# create target/release/pegon
	cargo build --release

.PHONY: wheel
wheel: ## Create python package
	# build python package with maturin
	uv build

.PHONY: lint
lint: ## Lint, format, test
	# run checks on all files
	uv run --frozen --only-dev prek --all-files --stage pre-push

.PHONY: bench
bench: ## Run micro-benchmarks
	# run benchmark suite
	cargo bench

.PHONY: profile-queries
profile-queries: ## Profile queries
	ts_query_ls profile

.PHONY: profile
profile: ## Profile lint run with perf
	RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release
	perf record --call-graph fp -c 10000 target/release/pegon check ~/workspace/lucene > out.txt || true
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
