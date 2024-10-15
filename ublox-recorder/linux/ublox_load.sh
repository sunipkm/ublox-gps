#!/usr/bin/env bash
modprobe ftdi_sio
# echo "1546 0508" > /sys/bus/usb-serial/drivers/ftdi_sio/new_id
echo "1546 0507" > /sys/bus/usb-serial/drivers/ftdi_sio/new_id
