if [[ $# -ne 1 ]]
then
	printf "Supply an output number\n"
	exit
fi

cargo run --release > "./images/out${1}.ppm" && feh "./images/out${1}.ppm"
