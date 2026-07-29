PYTHON ?= $(if $(wildcard .venv/bin/python),.venv/bin/python,python3)

.PHONY: setup pdf review-preface review-part-01 preface-figures split verify-splits preface-assets init-review rust-check clean

setup:
	python3 -m venv .venv
	.venv/bin/python -m pip install --upgrade pip
	.venv/bin/python -m pip install -r requirements.txt

pdf: preface-figures
	mkdir -p build
	@if command -v latexmk >/dev/null && command -v xelatex >/dev/null; then \
		cd tex && latexmk -xelatex -interaction=nonstopmode -halt-on-error \
			-outdir=../build main.tex; \
	elif command -v tectonic >/dev/null; then \
		cd tex && tectonic --keep-logs --keep-intermediates \
			--outdir ../build main.tex; \
	else \
		echo "A XeLaTeX toolchain (latexmk + xelatex) or tectonic is required"; \
		exit 1; \
	fi

review-preface: preface-figures
	mkdir -p build output/pdf
	@if command -v latexmk >/dev/null && command -v xelatex >/dev/null; then \
		cd tex && latexmk -xelatex -interaction=nonstopmode -halt-on-error \
			-outdir=../build review-preface.tex; \
	elif command -v tectonic >/dev/null; then \
		cd tex && tectonic --keep-logs --keep-intermediates \
			--outdir ../build review-preface.tex; \
	else \
		echo "A XeLaTeX toolchain (latexmk + xelatex) or tectonic is required"; \
		exit 1; \
	fi
	cp build/review-preface.pdf output/pdf/preface-review.pdf

review-part-01: preface-figures
	mkdir -p build output/pdf
	@if command -v latexmk >/dev/null && command -v xelatex >/dev/null; then \
		cd tex && latexmk -xelatex -interaction=nonstopmode -halt-on-error \
			-outdir=../build review-part-01.tex; \
	elif command -v tectonic >/dev/null; then \
		cd tex && tectonic --keep-logs --keep-intermediates \
			--outdir ../build review-part-01.tex; \
	else \
		echo "A XeLaTeX toolchain (latexmk + xelatex) or tectonic is required"; \
		exit 1; \
	fi
	cp build/review-part-01.pdf output/pdf/part-01-review.pdf

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

rust-check:
	cd code && cargo fmt --all --check
	cd code && cargo clippy --workspace --all-targets -- -D warnings
	cd code && cargo test --workspace

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
