PYTHON ?= python3

.PHONY: pdf split verify-splits clean

pdf:
	@command -v latexmk >/dev/null || { echo "latexmk is required"; exit 1; }
	@command -v xelatex >/dev/null || { echo "xelatex is required"; exit 1; }
	mkdir -p build
	cd tex && latexmk -xelatex -interaction=nonstopmode -halt-on-error -outdir=../build main.tex

split:
	$(PYTHON) scripts/split_pdf.py

verify-splits:
	$(PYTHON) scripts/split_pdf.py --verify-only

clean:
	@if command -v latexmk >/dev/null; then \
		cd tex && latexmk -C -outdir=../build main.tex; \
	else \
		echo "latexmk not installed; nothing cleaned"; \
	fi
