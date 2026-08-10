PYTHON ?= $(if $(wildcard .venv/bin/python),.venv/bin/python,python3)
PYGMENTS_PATH := $(abspath .venv/bin)

.PHONY: setup pdf preface-figures split verify-splits preface-assets init-review rust-snippets rust-check ocaml-check check-terms check-links clean

setup:
	python3 -m venv .venv
	.venv/bin/python -m pip install --upgrade pip
	.venv/bin/python -m pip install -r requirements.txt

pdf: check-terms check-links rust-snippets rust-check ocaml-check preface-figures
	mkdir -p build output/pdf
	@if command -v latexmk >/dev/null && command -v xelatex >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" latexmk -xelatex -shell-escape \
			-interaction=nonstopmode -halt-on-error \
			main.tex && \
			mv -f main.pdf ../build/main.pdf && \
			latexmk -C main.tex; \
	elif command -v tectonic >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" tectonic \
			-Z shell-escape-cwd="$$(pwd)" --keep-logs --keep-intermediates \
			--outdir ../build main.tex; \
	else \
		echo "A XeLaTeX toolchain (latexmk + xelatex) or tectonic is required"; \
		exit 1; \
	fi
	cp build/main.pdf output/pdf/tapl-zh.pdf

preface-figures:
	$(PYTHON) scripts/verify_preface_dependency_figure.py
	mkdir -p build/figures/preface figures/redrawn/preface
	@if command -v xelatex >/dev/null; then \
		SOURCE_DATE_EPOCH=1012521600 xelatex \
			-interaction=nonstopmode -halt-on-error \
			-output-directory=build/figures/preface \
			figures/redrawn/preface/chapter-dependencies.tex; \
	elif command -v tectonic >/dev/null; then \
		SOURCE_DATE_EPOCH=1012521600 tectonic \
			--outdir build/figures/preface \
			figures/redrawn/preface/chapter-dependencies.tex; \
	else \
		echo "XeLaTeX or tectonic is required to build the redrawn figures"; \
		exit 1; \
	fi
	cp build/figures/preface/chapter-dependencies.pdf \
		figures/redrawn/preface/chapter-dependencies.pdf

split:
	$(PYTHON) scripts/split_pdf.py

verify-splits:
	$(PYTHON) scripts/split_pdf.py --verify-only

rust-snippets:
	$(PYTHON) scripts/extract_rust_snippets.py

rust-check: rust-snippets
	cd code && cargo fmt --all --check
	cd code && cargo clippy --workspace --all-targets -- -D warnings
	cd code && cargo test --workspace

ocaml-check:
	@command -v ocamlc >/dev/null || { echo "ocamlc is required"; exit 1; }
	mkdir -p build/ocaml/chapter17
	cp code/ocaml/chapter17/diagnostics.ml build/ocaml/chapter17/
	cp code/ocaml/chapter17/coercion.ml build/ocaml/chapter17/
	cd build/ocaml/chapter17 && ocamlc -o diagnostics.byte diagnostics.ml && ./diagnostics.byte
	cd build/ocaml/chapter17 && ocamlc -o coercion.byte coercion.ml && ./coercion.byte

check-terms:
	$(PYTHON) scripts/check_term_first_use.py

check-links:
	$(PYTHON) scripts/check_reference_links.py

preface-assets:
	$(PYTHON) scripts/extract_preface_assets.py

init-review:
	@test -n "$(TARGET)" || { echo "TARGET is required"; exit 1; }
	@if test -n "$(UNIT)"; then \
		$(PYTHON) scripts/init_review.py --unit "$(UNIT)" --target "$(TARGET)"; \
	elif test -n "$(CHAPTER)"; then \
		$(PYTHON) scripts/init_review.py --chapter "$(CHAPTER)" --target "$(TARGET)"; \
	else \
		echo "UNIT or CHAPTER is required"; \
		exit 1; \
	fi

clean:
	@if command -v latexmk >/dev/null; then \
		cd tex && latexmk -C -outdir=../build main.tex; \
	else \
		echo "latexmk not installed; nothing cleaned"; \
	fi
