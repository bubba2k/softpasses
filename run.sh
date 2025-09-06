if [[ $# -ne 1 ]]
then
	printf "Please supply an output path\n"
	exit
fi

cargo run --release $1