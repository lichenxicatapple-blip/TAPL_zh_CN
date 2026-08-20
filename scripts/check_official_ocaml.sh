#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
build_root="$repo_root/build/ocaml/official"

rm -rf "$build_root"
mkdir -p "$build_root"

check_project() {
  name=$1
  source_relative=$2
  source_dir="$repo_root/source/official-code/extracted/$source_relative"
  build_dir="$build_root/$name"

  mkdir -p "$build_dir"
  cp -R "$source_dir/." "$build_dir/"
  printf '\nChecking official OCaml project: %s\n' "$name"
  make -C "$build_dir" test

  extra_test="$repo_root/code/ocaml/official-tests/$name.f"
  if [ -f "$extra_test" ]; then
    printf 'Running project smoke test: %s\n' "$extra_test"
    "$build_dir/f" "$extra_test"
  fi
}

check_project arith arith
check_project untyped untyped
check_project fulluntyped fulluntyped
check_project tyarith tyarith/tyarith
check_project simplebool simplebool/simplebool
check_project fullsimple fullsimple/fullsimple
check_project fullref fullref/fullref
check_project fullerror fullerror/fullerror
check_project fullsub fullsub/fullsub
check_project bot bot/bot
check_project rcdsubbot rcdsubbot/rcdsubbot
check_project recon recon
check_project reconbase reconbase
check_project fullrecon fullrecon
check_project fullpoly fullpoly
check_project fullfsub fullfsub
check_project fullfomsub fullfomsub
check_project purefsub purefsub

printf '\nAll official OCaml projects compiled and ran their bundled tests.\n'
