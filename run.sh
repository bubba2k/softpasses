if [[ $# -ne 1 ]]
then
	printf "Please supply an output path\n"
	exit
fi

cargo run --release $1; feh "${1}_albedo.png" 2> /dev/null; feh "${1}_normal.png" 2> /dev/null; feh "${1}_combined.png"; feh "${1}_denoise.png" 2> /dev/null;