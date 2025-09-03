if [[ $# -ne 1 ]]
then
	printf "Please supply an output path\n"
	exit
fi

cargo run --release $1; feh "${1}_albedo.ppm" 2> /dev/null; feh "${1}_normal.ppm" 2> /dev/null; feh "${1}_combined.ppm"; 