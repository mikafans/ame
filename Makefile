.PHONY: help fmt fmt-check lint test test-engine test-db test-bank test-stats test-assess test-api e2e uiux check ci db-up db-down db-reset db-migrate db-shell db-seed simulate init-env stop dev hooks-install openapi

COMPOSE ?= $(shell command -v podman >/dev/null 2>&1 && echo "podman compose" || echo "docker compose")

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

lint: ## Clippy + tsc + api drift check + sqlfluff lint
	cd api && mise exec -- cargo clippy --all-targets -- -D warnings
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bun run type-check && mise exec -- bun run api:check; \
	else \
		echo "[web] skipping lint + type-check (web deps missing - run 'cd web && bun install' to enable)"; \
	fi
	mise exec -- uvx sqlfluff lint db/migrations || true

test: ## Backend + frontend unit / integration tests
	cd api && mise exec -- cargo test
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bun test src/ || true; \
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
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test exams -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test stats -- --nocapture

test-bank: ## Bank integration tests only (requires `make db-up`)
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test bank -- --nocapture

test-assess: ## Exam composition integration tests (requires `make db-up`)
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test exams -- --nocapture

test-stats: ## Stats, messages, keys integration tests (requires `make db-up`)
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test stats -- --nocapture

test-api: ## Black-box HTTP tests against a running API (requires `make db-up` + API running)
	uv run pytest api_tests -v

openapi: ## Regenerate api/openapi.yaml and web TypeScript schema
	cd api && mise exec -- cargo run --quiet --bin gen-openapi
	@if [ -x web/node_modules/.bin/openapi-typescript ]; then \
		cd web && mise exec -- bun run api:gen; \
	else \
		echo "[web] skipping schema regen (web deps missing - run 'cd web && bun install' to enable)"; \
	fi

e2e: ## Playwright (requires `make db-up`; auto-starts API + seeds if not running)
	@if [ -x web/node_modules/.bin/next ]; then \
		_api_owned=0; \
		if ! curl -sf http://localhost:8080/healthz > /dev/null 2>&1; then \
			echo "[e2e] API not running — starting..."; \
			mkdir -p .tmp; \
			DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame RUST_LOG=warn \
				mise exec -- cargo run --manifest-path api/Cargo.toml --bin ame-api >> .tmp/ame-api-e2e.log 2>&1 & \
			echo $$! > .tmp/ame-api-e2e.pid; \
			_api_owned=1; \
			echo "[e2e] Waiting for API on :8080..."; \
			until curl -sf http://localhost:8080/healthz > /dev/null 2>&1; do sleep 1; done; \
		fi; \
		echo "[e2e] Seeding..."; \
		uv run scripts/seed.py >> .tmp/ame-api-e2e.log 2>&1 || true; \
		cd web && mise exec -- bun run e2e; _exit=$$?; \
		if [ "$$_api_owned" = "1" ] && [ -f .tmp/ame-api-e2e.pid ]; then \
			kill $$(cat .tmp/ame-api-e2e.pid) 2>/dev/null || true; \
			rm -f .tmp/ame-api-e2e.pid; \
		fi; \
		exit $$_exit; \
	else \
		echo "[web] skipping e2e (web deps missing - run 'cd web && bun install' to enable)"; \
	fi

uiux: ## Focused Playwright UI/UX contract spec (requires API + seed data)
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && mise exec -- bunx playwright test e2e/uiux-spec.spec.ts --project=chromium; \
	else \
		echo "[web] skipping uiux (web deps missing - run 'cd web && bun install' to enable)"; \
	fi

check: fmt-check lint test ## Pre-commit gate (read-only)

ci: check test-db e2e ## Full CI gate: fmt + lint + unit + db tests + e2e

db-up: ## Start Postgres (docker or podman)
	$(COMPOSE) -f db/docker-compose.yml up -d

db-down: ## Stop Postgres
	$(COMPOSE) -f db/docker-compose.yml down

db-reset: db-down ## Wipe and recreate DB from scratch (local dev only)
	rm -rf db/data
	$(MAKE) db-up
	@echo "Waiting for Postgres to be ready..."
	@until DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame sqlx migrate info --source db/migrations > /dev/null 2>&1; do sleep 1; done
	$(MAKE) db-migrate

db-migrate: ## Run pending sqlx migrations
	DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame sqlx migrate run --source db/migrations

db-shell: ## Open interactive pgcli session to local Postgres
	uvx pgcli postgres://postgres:postgres@localhost:5432/ame

db-seed: ## Seed demo users, tags, questions, quizzes, and exams (requires API running)
	uv run scripts/seed.py

simulate: ## Run all three role simulation scripts against local API (requires make dev + make db-seed)
	uv run scripts/simulate/instructor.py
	uv run scripts/simulate/learner.py
	uv run scripts/simulate/agent.py

init-env: ## One-time setup: mise install + sqlx-cli + web deps + playwright
	mise install
	cargo install sqlx-cli --no-default-features --features postgres
	cd web && mise exec -- bun install
	cd web && mise exec -- bunx playwright install --with-deps
	@echo "init-env done — run 'make dev' to start the stack"

stop: ## Stop API, frontend, and Postgres
	@lsof -ti :8080 -ti :3000 | xargs kill -9 2>/dev/null || true
	$(COMPOSE) -f db/docker-compose.yml down

API_HOST ?= localhost

dev: db-up ## Kill stale processes, migrate, then start API + frontend. Override: make dev API_HOST=harus-macmini
	@lsof -ti :8080 -ti :3000 | xargs kill -9 2>/dev/null || true
	@sleep 1
	$(MAKE) db-migrate
	@mkdir -p .tmp
	@echo "Starting API on :8080  (logs → .tmp/ame-api.log)"
	@DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame \
		RUST_LOG=ame_api=debug,tower_http=info,sqlx=warn \
		mise exec -- cargo run --manifest-path api/Cargo.toml --bin ame-api 2>&1 | tee .tmp/ame-api.log &
	@echo "Starting frontend on :3000 targeting $(API_HOST):8080 (logs → .tmp/ame-web.log)"
	@cd web && NEXT_PUBLIC_API_URL=http://$(API_HOST):8080 NEXT_ALLOWED_ORIGINS=$(API_HOST) \
		mise exec -- bun run dev 2>&1 | tee $(CURDIR)/.tmp/ame-web.log

hooks-install: ## Point git at .githooks/
	git config core.hooksPath .githooks
	@echo "git hooks installed (.githooks/)"
