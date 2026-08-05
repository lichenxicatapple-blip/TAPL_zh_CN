PYTHON ?= $(if $(wildcard .venv/bin/python),.venv/bin/python,python3)
PYGMENTS_PATH := $(abspath .venv/bin)

.PHONY: setup pdf review-preface review-part-01 review-part-02 preface-figures split verify-splits preface-assets init-review rust-snippets rust-check check-terms check-links clean

setup:
	python3 -m venv .venv
	.venv/bin/python -m pip install --upgrade pip
	.venv/bin/python -m pip install -r requirements.txt

pdf: check-terms check-links rust-snippets preface-figures
	mkdir -p build
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

review-preface: check-terms check-links preface-figures
	mkdir -p build output/pdf
	@if command -v latexmk >/dev/null && command -v xelatex >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" latexmk -xelatex -shell-escape \
			-interaction=nonstopmode -halt-on-error \
			review-preface.tex && \
			mv -f review-preface.pdf ../build/review-preface.pdf && \
			latexmk -C review-preface.tex; \
	elif command -v tectonic >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" tectonic \
			-Z shell-escape-cwd="$$(pwd)" --keep-logs --keep-intermediates \
			--outdir ../build review-preface.tex; \
	else \
		echo "A XeLaTeX toolchain (latexmk + xelatex) or tectonic is required"; \
		exit 1; \
	fi
	cp build/review-preface.pdf output/pdf/preface-review.pdf

review-part-01: check-terms check-links rust-snippets preface-figures
	mkdir -p build output/pdf
	@if command -v latexmk >/dev/null && command -v xelatex >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" latexmk -xelatex -shell-escape \
			-interaction=nonstopmode -halt-on-error \
			review-part-01.tex && \
			mv -f review-part-01.pdf ../build/review-part-01.pdf && \
			latexmk -C review-part-01.tex; \
	elif command -v tectonic >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" tectonic \
			-Z shell-escape-cwd="$$(pwd)" --keep-logs --keep-intermediates \
			--outdir ../build review-part-01.tex; \
	else \
		echo "A XeLaTeX toolchain (latexmk + xelatex) or tectonic is required"; \
		exit 1; \
	fi
	cp build/review-part-01.pdf output/pdf/part-01-review.pdf

review-part-02: check-terms check-links rust-snippets preface-figures
	mkdir -p build output/pdf
	@if command -v latexmk >/dev/null && command -v xelatex >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" latexmk -xelatex -shell-escape \
			-interaction=nonstopmode -halt-on-error \
			review-part-02.tex && \
			mv -f review-part-02.pdf ../build/review-part-02.pdf && \
			latexmk -C review-part-02.tex; \
	elif command -v tectonic >/dev/null; then \
		cd tex && PATH="$(PYGMENTS_PATH):$$PATH" tectonic \
			-Z shell-escape-cwd="$$(pwd)" --keep-logs --keep-intermediates \
			--outdir ../build review-part-02.tex; \
	else \
		echo "A XeLaTeX toolchain (latexmk + xelatex) or tectonic is required"; \
		exit 1; \
	fi
	cp build/review-part-02.pdf output/pdf/part-02-review.pdf

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
