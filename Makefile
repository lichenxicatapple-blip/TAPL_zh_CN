PYTHON ?= $(if $(wildcard .venv/bin/python),.venv/bin/python,python3)

.PHONY: setup pdf split verify-splits preface-assets init-review clean

setup:
	python3 -m venv .venv
	.venv/bin/python -m pip install --upgrade pip
	.venv/bin/python -m pip install -r requirements.txt

pdf:
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

split:
	$(PYTHON) scripts/split_pdf.py

verify-splits:
	$(PYTHON) scripts/split_pdf.py --verify-only

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
