# This simply recompiles (if necessary), runs the program and then opens the output files in GIMP.
# Useful for dev cycle

if [[ $# -ne 1 ]]
then
	printf "Please supply an output path\n"
	exit
fi

cargo run --release $1
gimp ${1}/*.png & 2> /dev/null
gimp ${1}/*.exr & 2> /dev/null

