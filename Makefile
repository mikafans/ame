.PHONY: help fmt fmt-check lint test test-engine test-db test-bank test-stats test-assess test-api bench bench-load e2e uiux check ci db-up db-down db-reset db-migrate db-shell db-admin db-seed db-bulk db-heavy simulate init-env stop dev hooks-install openapi docker-build docker-up docker-down docker-logs

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

bench: ## Criterion micro-benchmarks for hot paths (token verify, grader, elo)
	cd api && mise exec -- cargo bench --bench hot_paths

bench-load: ## k6 load test against a running API (requires `make dev` + `make db-seed`)
	@command -v k6 >/dev/null 2>&1 || { echo "k6 not found — install it (https://k6.io/docs/get-started/installation/)"; exit 1; }
	BASE_URL=http://localhost:$(API_PORT) k6 run api_tests/perf/answer_load.js

test-engine: ## Engine unit and integration-test compile gate
	cd api && mise exec -- cargo test engine
	cd api && mise exec -- cargo test --test planner

test-db: ## DB-backed backend integration tests (requires `make db-up`)
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test auth -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test bank -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test me -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test agent_tools -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test cross_owner -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test sessions -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test exams -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test quota -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test stats -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test planner -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test agent_manifest -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test export -- --nocapture
	cd api && AME_RUN_DB_TESTS=1 mise exec -- cargo test --test admin -- --nocapture

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
		mkdir -p .tmp; \
		if ! DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame sqlx migrate info --source db/migrations > /dev/null 2>&1; then \
			echo "[e2e] FAILED: Postgres is not reachable on :5432. Run 'make db-up' first."; \
			exit 1; \
		fi; 		if ! curl -sf http://$(API_HOST):$(API_PORT)/healthz > /dev/null 2>&1; then \
			echo "[e2e] API not running — starting..."; \
			DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame RUST_LOG=warn \
			AME_PORT=$(API_PORT) AME_CORS_ORIGINS=http://$(API_HOST):$(WEB_PORT) \
			AME_GLOBAL_RATELIMIT_BURST=20000 AME_RATELIMIT_BURST=100 AME_AGENT_ACCESS_CODE=e2e-access-code \
				mise exec -- cargo run --manifest-path api/Cargo.toml --bin ame-api >> .tmp/ame-api-e2e.log 2>&1 & \
			echo $$! > .tmp/ame-api-e2e.pid; \
			_api_owned=1; \
			echo "[e2e] Waiting for API on :$(API_PORT)..."; \
			until curl -sf http://$(API_HOST):$(API_PORT)/healthz > /dev/null 2>&1; do sleep 1; done; \
		fi; \
		echo "[e2e] Promoting admin (db-admin)..."; \
		if ! $(MAKE) --no-print-directory db-admin >> .tmp/ame-seed-e2e.log 2>&1; then \
			echo "[e2e] FAILED: db-admin errored — last 20 lines of .tmp/ame-seed-e2e.log:"; \
			tail -20 .tmp/ame-seed-e2e.log; \
			if [ "$$_api_owned" = "1" ] && [ -f .tmp/ame-api-e2e.pid ]; then \
				kill $$(cat .tmp/ame-api-e2e.pid) 2>/dev/null || true; rm -f .tmp/ame-api-e2e.pid; \
			fi; \
			exit 1; \
		fi; \
		echo "[e2e] Seeding..."; \
		if ! uv run scripts/seed.py --api http://$(API_HOST):$(API_PORT) >> .tmp/ame-seed-e2e.log 2>&1; then \
			echo "[e2e] FAILED: seeding errored — last 20 lines of .tmp/ame-seed-e2e.log:"; \
			tail -20 .tmp/ame-seed-e2e.log; \
			if [ "$$_api_owned" = "1" ] && [ -f .tmp/ame-api-e2e.pid ]; then \
				kill $$(cat .tmp/ame-api-e2e.pid) 2>/dev/null || true; rm -f .tmp/ame-api-e2e.pid; \
			fi; \
			exit 1; \
		fi; \
		echo "[e2e] Running playwright tests..."; \
		cd web && PORT=$(WEB_PORT) NEXT_PUBLIC_API_URL=http://$(API_HOST):$(API_PORT) \
			E2E_API_URL=http://$(API_HOST):$(API_PORT) E2E_BASE_URL=http://$(API_HOST):$(WEB_PORT) \
			mise exec -- bun run e2e > ../.tmp/playwright-e2e.log 2>&1; _exit=$$?; cd ..; \
		if [ "$$_api_owned" = "1" ] && [ -f .tmp/ame-api-e2e.pid ]; then \
			kill $$(cat .tmp/ame-api-e2e.pid) 2>/dev/null || true; \
			rm -f .tmp/ame-api-e2e.pid; \
		fi; \
		if [ $$_exit -ne 0 ]; then \
			echo "[e2e] Tests failed — last 40 lines of .tmp/playwright-e2e.log:"; \
			tail -40 .tmp/playwright-e2e.log; \
			echo "[e2e] (full logs: .tmp/{ame-api-e2e.log,ame-seed-e2e.log,playwright-e2e.log})"; \
		else \
			echo "[e2e] Tests passed."; \
		fi; \
		exit $$_exit; \
	else \
		echo "[web] skipping e2e (web deps missing - run 'cd web && bun install' to enable)"; \
	fi
 
uiux: ## Focused Playwright UI/UX contract spec (requires API + seed data)
	@if [ -x web/node_modules/.bin/next ]; then \
		cd web && PORT=$(WEB_PORT) NEXT_PUBLIC_API_URL=http://$(API_HOST):$(API_PORT) \
			E2E_API_URL=http://$(API_HOST):$(API_PORT) E2E_BASE_URL=http://$(API_HOST):$(WEB_PORT) \
			mise exec -- bunx playwright test e2e/uiux.spec.ts --project=chromium; \
	fi

check: fmt-check lint test ## Pre-commit gate (read-only)

# db-reset between test-db and e2e: test-db writes users/sessions into the shared
# dev DB, which pollutes the seeded state the e2e UI assertions depend on.
ci: check test-db db-reset e2e ## Full CI gate: fmt + lint + unit + db tests + e2e

db-up: ## Start Postgres (docker or podman)
	$(COMPOSE) -f db/docker-compose.yml up -d

db-down: ## Stop Postgres
	$(COMPOSE) -f db/docker-compose.yml down

db-reset: db-down ## Wipe and recreate DB from scratch (local dev only)
	@# Rootless podman writes db/data as a subordinate uid the host user can't
	@# rm directly, so fall back to `podman unshare` to delete inside the userns.
	@rm -rf db/data 2>/dev/null || podman unshare rm -rf db/data 2>/dev/null || true
	@if [ -d db/data ]; then echo "[db-reset] could not remove db/data (try: sudo rm -rf db/data)"; exit 1; fi
	$(MAKE) db-up
	@echo "Waiting for Postgres to be ready..."
	@until DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame sqlx migrate info --source db/migrations > /dev/null 2>&1; do sleep 1; done
	$(MAKE) db-migrate

db-migrate: ## Run pending sqlx migrations
	DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame sqlx migrate run --source db/migrations

db-shell: ## Open interactive pgcli session to local Postgres
	uvx pgcli postgres://postgres:postgres@localhost:5432/ame

db-admin: db-up ## Create/grant ADMIN_EMAIL (default admin@example.com) as admin directly in the DB
	@until $(COMPOSE) -f db/docker-compose.yml exec -T postgres pg_isready -U postgres -d ame >/dev/null 2>&1; do sleep 1; done
	@hash="$$(uvx --quiet --from argon2-cffi python -c \
		"from argon2 import PasswordHasher; print(PasswordHasher().hash('$(ADMIN_PASSWORD)'))")"; \
	$(COMPOSE) -f db/docker-compose.yml exec -T postgres \
		psql -U postgres -d ame -v ON_ERROR_STOP=1 -c \
		"INSERT INTO tb_users (email, display_name, role, password_hash) \
		 VALUES ('$(ADMIN_EMAIL)', '$(ADMIN_NAME)', 'admin', '$$hash') \
		 ON CONFLICT (email) DO UPDATE SET role = 'admin';"
	@echo "[db-admin] $(ADMIN_EMAIL) is admin — new users log in with password '$(ADMIN_PASSWORD)'; existing accounts keep theirs."

db-seed: db-admin ## Seed demo users, tags, questions, assessments, and exams (requires API running)
	uv run scripts/seed.py --api http://localhost:$(API_PORT)

db-bulk: ## Mint 10k questions and 1k exams via agent surface (requires API running)
	uv run scripts/mint_bulk.py --api http://localhost:$(API_PORT)

db-heavy: db-reset ## Wipe, migrate, seed, and mint 10k+1k (requires API running)
	@if ! curl -sf http://localhost:$(API_PORT)/healthz > /dev/null 2>&1; then \
		echo "API not running — please start it with 'make dev' in another terminal"; \
		exit 1; \
	fi
	$(MAKE) db-seed
	$(MAKE) db-bulk

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
	@fuser -k -9 $(API_PORT)/tcp $(WEB_PORT)/tcp 2>/dev/null || true
	$(COMPOSE) -f db/docker-compose.yml down

API_HOST ?= localhost
API_PORT ?= 28080
WEB_PORT ?= 23000

# Admin bootstrap (db-admin): registration only grants `user`, so the admin
# role is set out-of-band in the DB. Override ADMIN_EMAIL to promote yourself.
ADMIN_EMAIL ?= admin@example.com
ADMIN_NAME ?= Carol Admin
ADMIN_PASSWORD ?= password123

dev: db-up ## Kill stale processes, migrate, then start API + frontend. Override: make dev API_HOST=harus-mini
	@fuser -k -9 $(API_PORT)/tcp $(WEB_PORT)/tcp 2>/dev/null || true
	@sleep 1
	$(MAKE) db-migrate
	@mkdir -p .tmp
	@echo "Starting API on :$(API_PORT)  (logs → .tmp/ame-api.log)"
	@DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame \
		AME_PORT=$(API_PORT) \
		AME_CORS_ORIGINS=http://$(API_HOST):$(WEB_PORT) \
		AME_GLOBAL_RATELIMIT_BURST=50000 \
		RUST_LOG=ame_api=debug,tower_http=info,sqlx=warn \
		mise exec -- cargo run --manifest-path api/Cargo.toml --bin ame-api 2>&1 | tee .tmp/ame-api.log &
	@echo "Starting frontend on :$(WEB_PORT) targeting $(API_HOST):$(API_PORT) (logs → .tmp/ame-web.log)"
	@cd web && PORT=$(WEB_PORT) NEXT_PUBLIC_API_URL=http://$(API_HOST):$(API_PORT) NEXT_ALLOWED_ORIGINS=$(API_HOST) \
		mise exec -- bun run dev --port $(WEB_PORT) 2>&1 | tee $(CURDIR)/.tmp/ame-web.log

hooks-install: ## Point git at .githooks/
	git config core.hooksPath .githooks
	@echo "git hooks installed (.githooks/)"

# ── Containers ────────────────────────────────────────────────────────────
# Production-shaped stack. See deploy/README.md.

docker-build: ## Build api + web images via docker-compose.prod.yml
	$(COMPOSE) -f docker-compose.prod.yml build

docker-up: ## Start the prod-shaped stack (requires .env with POSTGRES_PASSWORD)
	$(COMPOSE) -f docker-compose.prod.yml up -d

docker-down: ## Stop the prod-shaped stack
	$(COMPOSE) -f docker-compose.prod.yml down

docker-logs: ## Tail logs from the prod-shaped stack
	$(COMPOSE) -f docker-compose.prod.yml logs -f
