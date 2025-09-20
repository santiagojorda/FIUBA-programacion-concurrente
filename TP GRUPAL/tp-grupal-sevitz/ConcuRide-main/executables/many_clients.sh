#!/bin/bash

TERMINAL="gnome-terminal"
DRIVER_COMMAND="cargo run --bin driver"
PASSENGER_COMMAND="cargo run --bin passenger"

# launch 6 drivers and 10 passengers
for i in $(seq 0 6); do
    $TERMINAL --tab --title=Driver_$i -- bash -c "${DRIVER_COMMAND} | tee output_driver_$i.txt; exec bash"
done

for i in $(seq 0 10); do
    $TERMINAL --tab --title=Passenger_$i -- bash -c "${PASSENGER_COMMAND} | tee output_passenger_$i.txt; exec bash"
done