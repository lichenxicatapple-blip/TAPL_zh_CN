PYTHON ?= python3

.PHONY: pdf split verify-splits init-review clean

pdf:
	@command -v latexmk >/dev/null || { echo "latexmk is required"; exit 1; }
	@command -v xelatex >/dev/null || { echo "xelatex is required"; exit 1; }
	mkdir -p build
	cd tex && latexmk -xelatex -interaction=nonstopmode -halt-on-error -outdir=../build main.tex

split:
	$(PYTHON) scripts/split_pdf.py

verify-splits:
	$(PYTHON) scripts/split_pdf.py --verify-only

init-review:
	@test -n "$(CHAPTER)" || { echo "CHAPTER is required"; exit 1; }
	@test -n "$(TARGET)" || { echo "TARGET is required"; exit 1; }
	$(PYTHON) scripts/init_review.py --chapter "$(CHAPTER)" --target "$(TARGET)"

clean:
	@if command -v latexmk >/dev/null; then \
		cd tex && latexmk -C -outdir=../build main.tex; \
	else \
		echo "latexmk not installed; nothing cleaned"; \
	fi
