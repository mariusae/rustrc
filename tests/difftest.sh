#!/bin/sh
# Differential test runner.
#
#   tests/difftest.sh diff [case]   compare our rc with the reference rc ($RC_REF)
#   tests/difftest.sh regen [case]  regenerate tests/expected from the reference rc
#   tests/difftest.sh check [case]  compare our rc with tests/expected
#
# Each test case is tests/cases/NAME.sh, a POSIX shell script that invokes
# `rc` (found via $PATH) however it likes.  Cases run in a scratch directory
# containing a fixed set of files, with a minimal environment.

set -u
mode=${1:-check}
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
RC_OURS=${RC_OURS:-$root/target/debug/rc}
RC_REF=${RC_REF:-${PLAN9:-/usr/local/plan9}/bin/rc}
PLAN9=${PLAN9:-/usr/local/plan9}
export PLAN9

normalize() {
	# process ids and other nondeterministic bits
	sed -e 's/^[0-9][0-9]*: signal:/PID: signal:/' \
	    -e 's/^apid=[0-9][0-9]*$/apid=PID/' \
	    -e 's/^pid=[0-9][0-9]*$/pid=PID/' \
	    -e 's|/tmp/here[0-9a-f]*\.[0-9a-f]*|/tmp/hereXXXX.XXXX|g' \
	    -e "s|$PLAN9/rcmain|RCMAIN|g" \
	    -e 's|[^ ]*/rcmain\.[0-9][0-9]*|RCMAIN|g'
}

# run CASE RC OUTDIR
run_case() {
	case_=$1; rc=$2; out=$3
	work=$(mktemp -d "${TMPDIR:-/tmp}/rctest.XXXXXX")
	realwork=$(cd "$work" && pwd -P)
	bindir=$work/bin
	mkdir -p "$bindir" "$work/d" "$work/d/sub" "$work/e"
	ln -s "$rc" "$bindir/rc"
	# a fixed set of files for globbing tests
	for f in a.c b.c c.h .hidden 'sp ace' d/x.c d/y.h d/sub/z.c e/q; do
		: > "$work/$f"
	done
	printf 'echo script args: $*\necho zero=$0\n' > "$work/script.rc"
	printf '#!/usr/bin/env rc\necho shebang $1\n' > "$bindir/shebang"
	chmod +x "$bindir/shebang"
	printf 'echo sourced $#* $1\nx=fromfile\n' > "$work/src.rc"
	name=$(basename "$case_" .sh)
	(
		cd "$work" || exit 99
		env -i PATH="$bindir:/usr/bin:/bin" HOME="$work" PLAN9="$PLAN9" TMPDIR="$work" \
			WORK="$work" CASES="$here/cases" \
			"$here/timeout.pl" "${CASE_TIMEOUT:-120}" /bin/sh "$case_" >"$out/$name.out" 2>"$out/$name.err" </dev/null
		echo $? > "$out/$name.code"
	)
	normalize < "$out/$name.out" > "$out/$name.out.n" && mv "$out/$name.out.n" "$out/$name.out"
	normalize < "$out/$name.err" > "$out/$name.err.n" && mv "$out/$name.err.n" "$out/$name.err"
	# the scratch directory name is nondeterministic
	sed -i.bak -e "s|$realwork|WORK|g" -e "s|$work|WORK|g" "$out/$name.out" "$out/$name.err" && rm -f "$out/$name".*.bak
	rm -rf "$work"
}

fail=0
total=0
cases=$(ls "$here"/cases/*.sh)
[ $# -ge 2 ] && cases="$here/cases/$2.sh"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/rcdiff.XXXXXX")
mkdir -p "$tmp/ours" "$tmp/ref"
for c in $cases; do
	name=$(basename "$c" .sh)
	total=$((total+1))
	case $mode in
	regen)
		run_case "$c" "$RC_REF" "$here/expected"
		;;
	diff)
		run_case "$c" "$RC_OURS" "$tmp/ours"
		run_case "$c" "$RC_REF" "$tmp/ref"
		for ext in out err code; do
			if ! cmp -s "$tmp/ours/$name.$ext" "$tmp/ref/$name.$ext"; then
				echo "FAIL $name ($ext)"
				diff "$tmp/ref/$name.$ext" "$tmp/ours/$name.$ext" | head -${DIFFLINES:-20}
				fail=$((fail+1))
			fi
		done
		;;
	check)
		run_case "$c" "$RC_OURS" "$tmp/ours"
		for ext in out err code; do
			if ! cmp -s "$tmp/ours/$name.$ext" "$here/expected/$name.$ext"; then
				echo "FAIL $name ($ext)"
				diff "$here/expected/$name.$ext" "$tmp/ours/$name.$ext" | head -${DIFFLINES:-20}
				fail=$((fail+1))
			fi
		done
		;;
	esac
done
rm -rf "$tmp"
[ "$mode" = regen ] && { echo "regenerated $total cases"; exit 0; }
echo "$total cases, $fail failures"
[ $fail -eq 0 ]
