.PHONY: dev build test fmt lint docker run clean seed seed-cathodic

# Local development: Rust API on :8080, Vite dev server on :5173 with /api proxy.
dev:
	@echo "Run in two terminals:"
	@echo "  make run     # backend on :8080"
	@echo "  make vite    # frontend on :5173"

vite:
	cd frontend && npm run dev

run:
	cd backend && STATIC_DIR=../frontend/dist cargo run

build:
	cd frontend && npm ci --no-audit --no-fund && npm run build
	cd backend && cargo build --release --locked

test:
	cd backend && cargo test --all-targets
	cd frontend && npm run typecheck && npm test

fmt:
	cd backend && cargo fmt

lint:
	cd backend && cargo clippy --all-targets -- -D warnings

docker:
	docker build -t electricacl:local .

image:
	docker run --rm -p 8080:8080 -v electricacl-data:/app/data electricacl:local

# Re-extract the seed data from the source workbook (needs python + openpyxl).
seed:
	python tools/extract_seed.py

seed-cathodic:
	python tools/extract_cathodic_seed.py

clean:
	rm -rf frontend/dist backend/target
