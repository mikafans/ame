.PHONY: help fmt fmt-check lint test test-db e2e check validate db-up db-down hooks-install

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN{FS=":.*?## "}{printf "%-16s %s\n", $$1, $$2}'

fmt: ## Format Rust + frontend + SQL (mutates files)
	cd api && mise exec -- cargo fmt
	cd web && mise exec -- bunx prettier --write .
	mise exec -- uvx sqlfluff format db/migrations || true

fmt-check: ## Verify formatting without mutating
	cd api && mise exec -- cargo fmt --check
	cd web && mise exec -- bunx prettier --check .

lint: ## Clippy + eslint + tsc + sqlfluff lint
	cd api && mise exec -- cargo clippy --all-targets -- -D warnings
	cd web && mise exec -- bun run lint
	cd web && mise exec -- bun run type-check
	mise exec -- uvx sqlfluff lint db/migrations || true

test: ## Backend + frontend unit / integration tests
	cd api && mise exec -- cargo test
	cd web && mise exec -- bun test --if-present || true

test-db: ## DB-backed backend integration tests (requires `make db-up`)
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test auth -- --nocapture

e2e: ## Playwright (requires `make db-up`)
	cd web && mise exec -- bun run e2e --if-present || true

check: fmt-check lint test ## Pre-commit gate (read-only)

validate: check e2e ## Pre-PR gate

db-up: ## Start Postgres in docker
	docker compose -f db/docker-compose.yml up -d

db-down: ## Stop Postgres
	docker compose -f db/docker-compose.yml down

hooks-install: ## Point git at .githooks/
	git config core.hooksPath .githooks
	@echo "git hooks installed (.githooks/)"
