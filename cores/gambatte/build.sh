#!/bin/sh
set -eu

device_cflags="-mcpu=cortex-a53 -mtune=cortex-a53"

usage() {
	echo "usage: $0 stamp COMMIT | build COMMIT TARBALL WORKDIR OUT PLATFORM" >&2
	exit 2
}

stamp() {
	echo "commit=$1"
	echo "source=https://github.com/libretro/gambatte-libretro/archive/$1.tar.gz"
	echo "device_platform=rpi3_64"
	echo "device_cflags=$device_cflags"
}

build() {
	commit="$1" tarball="$2" work="$3" out="$4" platform="$5"
	src="$work/gambatte-libretro-$commit"

	rm -rf "$work"
	mkdir -p "$work"
	tar -xzf "$tarball" -C "$work"
	if [ ! -f "$src/Makefile" ] || [ ! -f "$src/COPYING" ]; then
		echo "$tarball does not hold the complete gambatte-libretro-$commit source archive" >&2
		exit 1
	fi

	CFLAGS="$device_cflags" make -C "$src" platform="$platform" \
		GIT_VERSION="\"$(printf %s "$commit" | cut -c1-7)\"" \
		-j"$(getconf _NPROCESSORS_ONLN)"

	case "$platform" in
		osx) artifact=gambatte_libretro.dylib ;;
		*) artifact=gambatte_libretro.so ;;
	esac
	if [ ! -f "$src/$artifact" ]; then
		echo "make finished without producing $artifact" >&2
		exit 1
	fi
	mkdir -p "$(dirname "$out")"
	cp "$src/$artifact" "$out"
	stamp "$commit" >"$out.meta"
}

case "${1:-}" in
stamp)
	[ $# -eq 2 ] && [ -n "$2" ] || usage
	stamp "$2"
	;;
build)
	[ $# -eq 6 ] && [ -n "$2" ] && [ -n "$3" ] && [ -n "$4" ] && [ -n "$5" ] && [ -n "$6" ] || usage
	build "$2" "$3" "$4" "$5" "$6"
	;;
*)
	usage
	;;
esac
