.PHONY: help fmt fmt-check lint test test-db test-api e2e uiux local-uiux preview check ci db-up db-down db-reset db-migrate db-backup db-restore init-env stop dev dev-reset hooks-install openapi public-docs docker-build docker-up docker-down docker-logs docker-shell docker-admin db-shell db-admin local-seed local-logs local-api-contracts local-haru-simulation local-reference-courses local-learner-workspace local-persistence-check

API_HOST ?= localhost
API_PORT ?= 28080
WEB_PORT ?= 23000
PREVIEW_PORT ?= 28900
BACKUP_FILE ?= backup.dump
RESTORE_DB ?= ame_scratch
ADMIN_EMAIL ?= admin@example.com
ADMIN_NAME ?= Carol Admin
ADMIN_PASSWORD ?= password123

export API_HOST API_PORT WEB_PORT PREVIEW_PORT BACKUP_FILE RESTORE_DB ADMIN_EMAIL ADMIN_NAME ADMIN_PASSWORD

help:
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z0-9_.-]+:.*## / {printf "%-16s %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort

# Quality and test gates
fmt: ## Format Rust, frontend, and SQL
	uv run scripts/tasks.py fmt
fmt-check: ## Verify Rust, frontend, and SQL formatting
	uv run scripts/tasks.py fmt-check
lint: ## Run Rust, TypeScript, OpenAPI, and SQL checks
	uv run scripts/tasks.py lint
test: ## Run backend and frontend unit tests
	uv run scripts/tasks.py test
check: fmt-check lint test ## Run the pre-commit gate
ci: check test-db db-reset e2e ## Run the full local CI gate

test-db: db-up ## Run all DB-backed integration tests
	uv run scripts/tasks.py test-db
test-api: ## Run black-box HTTP tests against the local API
	uv run pytest api_tests -v
bench-load: ## Run the answer load benchmark against a running API
	uv run scripts/perf.py load
bench-soak: ## Run the duration benchmark against a running API
	uv run scripts/perf.py soak

# Application and API artifacts
openapi: ## Regenerate the OpenAPI and generated frontend schema
	uv run scripts/tasks.py openapi

public-docs: openapi ## Regenerate generated documents in docs/public
	uv run scripts/tasks.py public-docs
e2e: db-up ## Run the seeded Playwright suite
	uv run scripts/e2e.py
uiux: ## Run the focused UI/UX contract against the host stack
	uv run scripts/uiux.py
local-uiux: ## Run the focused UI/UX contract through Caddy
	uv run scripts/local_stack.py uiux
preview: ## Serve design previews
	uv run scripts/tasks.py preview

# Database (the bare test Postgres backing make test-db/make ci; not make dev's)
db-up: ## Start Postgres and Valkey
	uv run scripts/tasks.py db-up
db-down: ## Stop Postgres and Valkey
	uv run scripts/tasks.py db-down
db-reset: ## Recreate the local database and cache
	uv run scripts/tasks.py db-reset
db-migrate: ## Run pending SQL migrations
	uv run scripts/tasks.py db-migrate
db-backup: db-up ## Back up the bare test Postgres (make test-db's database; not make dev's)
	uv run scripts/db.py backup
db-restore: db-up ## Restore the bare test Postgres (make test-db's database; not make dev's)
	uv run scripts/db.py restore

# Development
init-env: ## Install project toolchains, dependencies, and browsers
	uv run scripts/tasks.py init-env
dev: ## Start the one local stack: Postgres, Valkey, API, and web behind Caddy at :28800
	uv run scripts/local_stack.py up
stop: ## Stop the local stack; keeps the named Postgres volume
	uv run scripts/local_stack.py down
dev-reset: ## Wipe the local stack's Postgres volume and restart clean
	uv run scripts/local_stack.py reset

# Container workflows — docker-* targets the production-shaped stack
# (deploy/docker-compose.prod.yml); local-*/db-shell/db-admin target the
# make dev stack (deploy/docker-compose.local.yml).
docker-build: ## Build production-shaped images
	uv run scripts/tasks.py docker-build
docker-up: ## Start the production-shaped stack
	uv run scripts/tasks.py docker-up
docker-down: ## Stop the production-shaped stack
	uv run scripts/tasks.py docker-down
docker-logs: ## Tail production-shaped stack logs
	uv run scripts/tasks.py docker-logs
docker-shell: ## Open an interactive psql shell in the production-shaped stack
	uv run scripts/tasks.py docker-shell
docker-admin: ## Promote the configured admin account in the production-shaped stack
	uv run scripts/tasks.py docker-admin
db-shell: ## Open an interactive psql shell in the running make dev stack
	uv run scripts/local_stack.py shell
db-admin: ## Promote the configured admin account in the running make dev stack
	uv run scripts/local_stack.py admin
local-seed: ## Seed the containerized stack through Caddy
	uv run scripts/local_stack.py seed
local-logs: ## Tail containerized stack logs
	uv run scripts/local_stack.py logs
local-api-contracts: ## Run the api_tests black-box contract suite through Caddy
	uv run scripts/local_stack.py api-contracts
local-haru-simulation: ## Create a disposable three-round Haru journey; requires HARU_SIM_PASSWORD
	uv run scripts/haru_simulation.py
local-reference-courses: ## Create delegated Flink and Netty courses for one learner; requires HARU_REFERENCE_PASSWORD
	uv run scripts/seed_reference_courses.py
local-learner-workspace: ## Seed active/completed delegated courses and smoke the learner workspace through Caddy
	uv run scripts/local_stack.py learner-workspace
local-persistence-check: ## Rehearse learner persistence across a non-destructive local-stack restart
	uv run scripts/test_local_stack_persistence.py

hooks-install: ## Configure the repository git hooks
	git config core.hooksPath .githooks
