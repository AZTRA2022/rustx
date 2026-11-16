#!/bin/sh
# Delete the build output of every project on this machine.
#
# The POSIX half of `scripts/clean.ps1`, same rules and same words. See
# that file for why this exists; the short version is that one build of
# this framework is 13 GB, and a full disk makes cargo, tsc and
# rustdoc all report something other than the disk.
#
#   sh scripts/clean.sh                 # this project
#   sh scripts/clean.sh --all           # every project under $HOME
#   sh scripts/clean.sh --all --dry-run
#   sh scripts/clean.sh --all --caches  # also the registry downloads
#   sh scripts/clean.sh --root ~/work

set -eu

root=''
all=0
caches=0
dry=0
yes=0

while [ $# -gt 0 ]; do
	case "$1" in
	--all) all=1 ;;
	--caches) caches=1 ;;
	--dry-run) dry=1 ;;
	--yes | -y) yes=1 ;;
	--root)
		shift
		root="${1:-}"
		;;
	-h | --help)
		sed -n '2,16p' "$0"
		exit 0
		;;
	*)
		echo "clean.sh: $1 is not an option. --help says what is." >&2
		exit 2
		;;
	esac
	shift
done

if [ -z "$root" ]; then
	if [ "$all" -eq 1 ]; then
		root="$HOME"
	else
		root=$(cd "$(dirname "$0")/.." && pwd)
	fi
fi
[ -d "$root" ] || {
	echo "clean.sh: $root is not there." >&2
	exit 2
}

# `du -sh` on every candidate is one stat walk per folder, which is the
# same walk the delete does -- so it is measured once, here, and the
# number is what gets printed and summed.
size_kb() { du -sk "$1" 2>/dev/null | awk '{print $1}'; }

gb() { awk -v kb="$1" 'BEGIN { printf "%8.2f GB", kb / 1048576 }'; }

echo
echo "rustx clean"
echo "  looking under $root"
echo

# One `find` that prunes what it matches, so it never descends into a
# `node_modules` to look for a `target` inside it.
#
# `dist` and `build` are not here: they are also ordinary source
# directory names, and the PowerShell half only takes them when a
# manifest sits beside them. A `find` cannot ask that cheaply, so this
# half leaves them alone rather than guess.
candidates=$(
	find "$root" \
		\( -name .git -o -name .cargo \) -prune -o \
		-type d \( \
		-name target -o \
		-name node_modules -o \
		-name .next -o \
		-name .turbo -o \
		-name .svelte-kit -o \
		-name .parcel-cache -o \
		-name .rustx -o \
		-name __pycache__ -o \
		-name .pytest_cache -o \
		-name .mypy_cache \
		\) -print -prune 2>/dev/null || true
)

# A `target` is only a build directory when a manifest sits beside it.
keep=''
for path in $candidates; do
	case "$(basename "$path")" in
	target)
		[ -f "$(dirname "$path")/Cargo.toml" ] || continue
		;;
	esac
	keep="$keep
$path"
done

# The framework's own temporary folders, and the caches when asked.
for path in "${TMPDIR:-/tmp}"/rustx-* "$HOME/.cache/rustx"; do
	[ -e "$path" ] && keep="$keep
$path"
done || true

if [ "$caches" -eq 1 ]; then
	for path in "$HOME/.cargo/registry/cache" "$HOME/.cargo/registry/src" \
		"$HOME/.cargo/git/checkouts" "$HOME/.npm/_cacache"; do
		[ -d "$path" ] && keep="$keep
$path"
	done || true
fi

keep=$(printf '%s\n' "$keep" | sed '/^$/d' | sort -u)
[ -n "$keep" ] || {
	echo "  nothing to delete."
	echo
	exit 0
}

total=0
count=0
printf '%s\n' "$keep" | while read -r path; do
	[ -n "$path" ] || continue
	kb=$(size_kb "$path")
	[ -n "$kb" ] || kb=0
	printf '  %s  %s\n' "$(gb "$kb")" "$path"
done

total=$(printf '%s\n' "$keep" | while read -r path; do
	[ -n "$path" ] && size_kb "$path"
done | awk '{ s += $1 } END { print s + 0 }')
count=$(printf '%s\n' "$keep" | wc -l | tr -d ' ')

echo
echo "  $count folder(s), $(gb "$total") in all"

if [ "$dry" -eq 1 ]; then
	echo "  --dry-run: nothing was deleted."
	echo
	exit 0
fi

if [ "$yes" -eq 0 ] && [ "$total" -gt 1048576 ]; then
	echo
	printf '  Delete all of it? Every build will be from scratch. [y/N] '
	read -r answer || answer=n
	case "$answer" in
	y | Y | yes) ;;
	*)
		echo "  nothing was deleted."
		echo
		exit 0
		;;
	esac
fi

held=''
printf '%s\n' "$keep" | while read -r path; do
	[ -n "$path" ] || continue
	rm -rf "$path" 2>/dev/null || true
	[ -e "$path" ] && echo "    $path"
done >"${TMPDIR:-/tmp}/rustx-clean-held.$$" || true

held=$(cat "${TMPDIR:-/tmp}/rustx-clean-held.$$" 2>/dev/null || true)
rm -f "${TMPDIR:-/tmp}/rustx-clean-held.$$"

echo
echo "  done."
if [ -n "$held" ]; then
	echo
	echo "  still there, held by something that is running:"
	printf '%s\n' "$held"
	echo "  close the editor, the window or \`rustx dev\`, and run this again."
fi
echo
