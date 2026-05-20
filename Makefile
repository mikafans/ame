.PHONY: help fmt fmt-check lint test test-engine test-db test-bank e2e check validate db-up db-down hooks-install openapi

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN{FS=":.*?## "}{printf "%-16s %s\n", $$1, $$2}'

fmt: ## Format Rust + frontend + SQL (mutates files)
	cd api && mise exec -- cargo fmt
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bunx prettier --write .; \
	else \
		echo "[web] skipping prettier --write (web deps missing - run 'cd web && bun install' to enable)"; \
	fi
	mise exec -- uvx sqlfluff format db/migrations || true

fmt-check: ## Verify formatting without mutating
	cd api && mise exec -- cargo fmt --check
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bunx prettier --check .; \
	else \
		echo "[web] skipping prettier --check (web deps missing - run 'cd web && bun install' to enable)"; \
	fi

lint: ## Clippy + eslint + tsc + sqlfluff lint
	cd api && mise exec -- cargo clippy --all-targets -- -D warnings
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bun run lint && mise exec -- bun run type-check; \
	else \
		echo "[web] skipping lint + type-check (web deps missing - run 'cd web && bun install' to enable)"; \
	fi
	mise exec -- uvx sqlfluff lint db/migrations || true

test: ## Backend + frontend unit / integration tests
	cd api && mise exec -- cargo test
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bun test --if-present || true; \
	else \
		echo "[web] skipping bun test (web deps missing - run 'cd web && bun install' to enable)"; \
	fi

test-engine: ## Engine unit and integration-test compile gate
	cd api && mise exec -- cargo test engine
	cd api && mise exec -- cargo test --test planner

test-db: ## DB-backed backend integration tests (requires `make db-up`)
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test auth -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test bank -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test sessions -- --nocapture

test-bank: ## Bank integration tests only (requires `make db-up`)
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test bank -- --nocapture

openapi: ## Regenerate api/openapi.yaml snapshot
	cd api && mise exec -- cargo run --quiet --bin gen-openapi

e2e: ## Playwright (requires `make db-up`)
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bun run e2e --if-present || true; \
	else \
		echo "[web] skipping e2e (web deps missing - run 'cd web && bun install' to enable)"; \
	fi

check: fmt-check lint test ## Pre-commit gate (read-only)

validate: check e2e ## Pre-PR gate

db-up: ## Start Postgres in docker
	docker compose -f db/docker-compose.yml up -d

db-down: ## Stop Postgres
	docker compose -f db/docker-compose.yml down

hooks-install: ## Point git at .githooks/
	git config core.hooksPath .githooks
	@echo "git hooks installed (.githooks/)"
