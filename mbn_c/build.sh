#!/bin/bash

options() {
	echo "1 - Build"
	echo "2 - Run"
	echo "3 - Build & Test"
	echo "4 - Build & Run"
}

build() {
	# cd ../
	cargo build

	# cd mbn_c
	# Build artifacts
	cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -B build

	# Build
	cmake --build ./build

}

run() {
	# Run binary
	./bin/mbn
}

tester() {
	if cd build; then
		ctest --verbose
	fi
}

# Main
while true; do
	options
	read -r option

	case $option in
	1)
		build
		break
		;;
	2)
		run
		break
		;;
	3)
		build
		tester
		break
		;;
	4)
		build
		run
		break
		;;
	*) echo "Choose a different one." ;;
	esac
done
