#!/bin/bash

# this bash script will run the admin for each terminal and redirect the output to a file,
# the output file will be named "output_$PORT.txt.
# THESE TERMINALS WILL PRINT LOGS RELATED TO ELECTIONS BETWEEN ADMINS

# EXECUTE BEFORE THIS!!!:
# chmod +x election_admins.sh

N_TERMINALS=5

STARTING_PORT=8080

COMMANDS=(
    "cargo run --features election_logs --bin admin 8080"
    "cargo run --features election_logs --bin admin 8081"
    "cargo run --features election_logs --bin admin 8082"
    "cargo run --features election_logs --bin admin 8083"
    "cargo run --features election_logs --bin admin 8084"
)

TERMINAL="gnome-terminal" # CAN BE CHANGED TO "xterm"

if ! command -v $TERMINAL &> /dev/null; then
    echo "$TERMINAL it's not installed. Please install it or change the terminal you have in TERMINAL parameter in this .sh."
    exit 1
fi

for i in $(seq $((N_TERMINALS - 1)) -1 0); do
    if [ $i -lt ${#COMMANDS[@]} ]; then

        PORT=$((STARTING_PORT + $i))
        OUTPUT="output_$PORT.txt"


        $TERMINAL --tab --title=Admin_$PORT -- bash -c "${COMMANDS[$i]} | tee $OUTPUT; exec bash"
    else
        echo "Commands missing for terminals given."
    fi
done
