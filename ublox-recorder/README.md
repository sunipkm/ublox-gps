# GPS and TEC data recorder with u-blox EVK-F9P

### `udev` rules for u-blox EVK-F9P
1. Disconnect the USB cable.
2. Copy the `99-ublox-load.rules` file to `/etc/udev/rules.d`:
   ```sh
   $ sudo cp 99-ublox-load.rules /etc/udev/rules.d
   ```
3. Reload the `udev` daemon:
   ```sh
   $ sudo udevadm control --reload-rules
   ```
4. Copy the `ublox_load.sh` script to the root directory, and add execute permission:
   ```sh
   $ sudo cp ublox_load.sh / && sudo chmod +x /ublox_load.sh
   ```
5. Plug in the device, and make sure `/dev/ttyUSB` device is showing up.
