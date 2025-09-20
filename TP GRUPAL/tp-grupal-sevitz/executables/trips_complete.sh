#!/bin/bash

# this bash script will run the admin for each terminal and redirect the output to a file,
# the output file will be named "output_$PORT.txt.
# THESE TERMINALS WILL PRINT LOGS RELATED TO TRIPS
# also will launch 6 driver terminals and 10 passenger terminals and 1 payment gateway terminal

# EXECUTE BEFORE THIS!!!:
# chmod +x trips_complete.sh

N_TERMINALS=5

STARTING_PORT=8080


ADMIN_COMMANDS=(
    "cargo run --features trip_logs --bin admin 8080"
    "cargo run --features trip_logs --bin admin 8081"
    "cargo run --features trip_logs --bin admin 8082"
    "cargo run --features trip_logs --bin admin 8083"
    "cargo run --features trip_logs --bin admin 8084"
)
GATEWAY_COMMAND="cargo run --bin payment"

DRIVER_COMMAND="cargo run --bin driver"

PASSENGER_COMMAND="cargo run --bin passenger"

TERMINAL="gnome-terminal" # CAN BE CHANGED TO "xterm"

GATEWAY_OUTPUT="payment_gateway.txt"

if ! command -v $TERMINAL &> /dev/null; then
    echo "$TERMINAL it's not installed. Please install it or change the terminal you have in TERMINAL parameter in this .sh."
    exit 1
fi

$TERMINAL -- bash -c "$GATEWAY_COMMAND | tee $GATEWAY_OUTPUT; exec bash"

for i in $(seq $((N_TERMINALS - 1)) -1 0); do
    if [ $i -lt ${#ADMIN_COMMANDS[@]} ]; then
        PORT=$((STARTING_PORT + $i))
        OUTPUT="output_admin_$PORT.txt"

        $TERMINAL -- bash -c "${ADMIN_COMMANDS[$i]} | tee $OUTPUT; exec bash"
    else
        echo "Commands missing for terminals given."
    fi
done

# run apps for 8 secs
sleep 8

# launch 6 drivers and 10 passengers
for i in $(seq 0 6); do
    $TERMINAL -- bash -c "${DRIVER_COMMAND} | tee output_driver_$i.txt; exec bash"
done

for i in $(seq 0 10); do
    $TERMINAL -- bash -c "${PASSENGER_COMMAND} | tee output_passenger_$i.txt; exec bash"
done
