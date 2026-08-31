PYTHON ?= $(if $(wildcard .venv/bin/python),.venv/bin/python,python3)
PYGMENTS_PATH := $(abspath .venv/bin)
PDF_OUTPUT := TAPL_zh_CN.pdf

.PHONY: setup pdf preface-figures split verify-splits preface-assets rust-snippets rust-check ocaml-check check-terms check-links clean distclean

setup:
	python3 -m venv .venv
	.venv/bin/python -m pip install --upgrade pip
	.venv/bin/python -m pip install -r requirements.txt

pdf: check-terms check-links rust-snippets rust-check ocaml-check preface-figures
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
	cp build/main.pdf $(PDF_OUTPUT)

preface-figures:
	$(PYTHON) scripts/verify_preface_dependency_figure.py
	mkdir -p build/figures/preface
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
	@command -v ocamllex >/dev/null || { echo "ocamllex is required"; exit 1; }
	@command -v ocamlyacc >/dev/null || { echo "ocamlyacc is required"; exit 1; }
	mkdir -p build/ocaml/chapter04 build/ocaml/chapter07
	cp code/ocaml/chapter04/evaluators.ml build/ocaml/chapter04/
	cp code/ocaml/chapter07/evaluators.ml build/ocaml/chapter07/
	cd build/ocaml/chapter04 && ocamlc -o evaluators.byte evaluators.ml && ./evaluators.byte
	cd build/ocaml/chapter07 && ocamlc -o evaluators.byte evaluators.ml && ./evaluators.byte
	mkdir -p build/ocaml/chapter17
	cp code/ocaml/chapter17/core.ml build/ocaml/chapter17/
	cp code/ocaml/chapter17/join.ml build/ocaml/chapter17/
	cp code/ocaml/chapter17/diagnostics.ml build/ocaml/chapter17/
	cp code/ocaml/chapter17/coercion.ml build/ocaml/chapter17/
	cd build/ocaml/chapter17 && ocamlc -o core.byte core.ml && ./core.byte
	cd build/ocaml/chapter17 && ocamlc -o join.byte join.ml && ./join.byte
	cd build/ocaml/chapter17 && ocamlc -o diagnostics.byte diagnostics.ml && ./diagnostics.byte
	cd build/ocaml/chapter17 && ocamlc -o coercion.byte coercion.ml && ./coercion.byte
	scripts/check_official_ocaml.sh

check-terms:
	$(PYTHON) scripts/check_term_first_use.py

check-links:
	$(PYTHON) scripts/check_reference_links.py

preface-assets:
	$(PYTHON) scripts/extract_preface_assets.py

clean:
	rm -rf build tmp scripts/__pycache__
	rm -f tex/*.aux tex/*.bbl tex/*.bcf tex/*.blg tex/*.fdb_latexmk \
		tex/*.fls tex/*.idx tex/*.ilg tex/*.ind tex/*.lof tex/*.log \
		tex/*.lot tex/*.out tex/*.run.xml tex/*.synctex.gz tex/*.toc \
		tex/*.xdv

distclean: clean
	rm -rf code/target
