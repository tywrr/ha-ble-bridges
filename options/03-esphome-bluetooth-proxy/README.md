# Option 3: ESPHome Bluetooth proxy on an ESP32-C6

A $6 to $10 ESP32-C6 board running ESPHome's
[`bluetooth_proxy`](https://esphome.io/components/bluetooth_proxy/) component.
Home Assistant's Bluetooth stack treats it as a remote adapter: scanning,
GATT connections and characteristic writes all travel over the ESPHome native
API, so the HA host itself needs no radio. The SwitchBot Bot then appears
through HA's **built-in SwitchBot integration**, with battery level, press
feedback and a real device page, exactly as if HA had a local dongle.

This is the option that replaced the Rust bridge and the one to pick unless
you have a specific reason not to (see the root README).

## Hardware

* Any ESP32-C6 dev board. The config was tested on an
  [M5Stack NanoC6](https://docs.m5stack.com/en/core/M5NanoC6) (USB-C, built-in
  blue status LED on GPIO7, about the size of a thumb). An ESP32-S3 or classic
  ESP32 also works; change `esp32.board`/`variant` and the LED pin.
* A USB power source near the Bot. The proxy needs no data connection after the
  first flash.
* 2.4 GHz Wi-Fi. The C6 has no 5 GHz radio. If your SSID is 5/6 GHz only you
  must add a 2.4 GHz SSID (most routers let you split the bands or add an IoT
  network).

## Files

| File                        | Purpose                                                          |
| --------------------------- | ---------------------------------------------------------------- |
| `ble-proxy-esp32c6.yaml`    | The ESPHome config. Every secret is a `!secret` reference.       |
| `secrets.example.yaml`      | Template. Copy to `secrets.yaml` (gitignored) and fill in.       |

Generate the API key with `openssl rand -base64 32`. HA asks for it when you
add the device.

## Flashing

### 1. Host prep (Linux)

The board enumerates as a USB CDC serial device with Espressif's vendor id
`303a`. Without a udev rule you will get `Permission denied` on `/dev/ttyACM0`.

```bash
# /etc/udev/rules.d/70-espressif.rules
SUBSYSTEMS=="usb", ATTRS{idVendor}=="303a", MODE="0660", TAG+="uaccess"
```

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

`uaccess` grants the logged-in desktop user access without group changes. On a
headless host, use `GROUP="dialout"` instead and add your user to that group.

On macOS no rule is needed; the device shows up as `/dev/cu.usbmodem*`.

### 2. ESPHome CLI

Pin the version so the generated firmware is reproducible:

```bash
# one-off, no install
uvx esphome==2026.10.1 version

# or a persistent install
pip install esphome==2026.10.1
```

The first compile pulls the ESP-IDF toolchain through PlatformIO (several
hundred MB, a few minutes).

### 3. First flash over USB

```bash
cd options/03-esphome-bluetooth-proxy
cp secrets.example.yaml secrets.yaml && $EDITOR secrets.yaml
uvx esphome==2026.10.1 run ble-proxy-esp32c6.yaml --device /dev/ttyACM0
```

If the board is not detected, hold its BOOT button while plugging it in to
force the ROM bootloader, then flash again.

### 4. Later updates over the air

Once the device is on Wi-Fi, drop `--device`; ESPHome resolves
`<name>.local` via mDNS and flashes OTA using `ota_password`:

```bash
uvx esphome==2026.10.1 run ble-proxy-esp32c6.yaml
```

`uvx esphome==2026.10.1 logs ble-proxy-esp32c6.yaml` streams the log over the
API, which is the quickest way to see BLE advertisements and RSSI.

## Pairing in Home Assistant

1. HA discovers the device by mDNS and offers it under **Settings > Devices &
   services > Discovered**. If it does not, add the **ESPHome** integration
   manually with the device's hostname or address.
2. Paste the `api_encryption_key` when asked.
3. Under **Settings > Devices & services > Bluetooth** the proxy now appears as
   an adapter. If the HA host has no local adapter you will see only the proxy,
   which is fine. HA may log a warning about missing `NET_ADMIN`/`NET_RAW`
   capabilities in a container; that concerns local-adapter recovery only and
   is harmless here.

## The Bot shows up by itself

With an active proxy in range, HA's Bluetooth discovery sees the Bot's
advertisements and offers the **SwitchBot** integration as a discovered device.
Accept it. If the Bot has a password set in the SwitchBot app, HA asks for it
here (the integration implements the passworded frame; see
[`docs/switchbot-bot-protocol.md`](../../docs/switchbot-bot-protocol.md)).

You get:

* `switch.<bot>`: in press mode `turn_on` is a press; in switch mode on/off
  map to the arm positions.
* `sensor.<bot>_battery`.
* RSSI and last-seen on the device page, which is how you find out whether the
  proxy is placed well.

Then build the automation in
[`docs/verification-pattern.md`](../../docs/verification-pattern.md). A press
through the proxy is more reliable than through the laptop bridge was, but it
is still a mechanical arm hitting a button, and nothing reports back whether
the button did anything.

## Placement and range

* The Bot is a coin cell device with a tiny antenna; the proxy, not the Bot,
  is what you move. Aim for RSSI better than about -75 dBm as read on the
  Bot's HA device page. At -80 dBm presses still worked but discovery took
  longer after each write.
* Keep the proxy off metal and away from the microwave, and ideally in the
  same room as the Bot, not just the same floor.
* More than one proxy is fine. HA uses whichever hears the device best.

## Troubleshooting

See [`docs/troubleshooting.md`](../../docs/troubleshooting.md). The short
list: it is on 5 GHz-only Wi-Fi, it is in the fallback AP, the udev rule is
missing, or the Bot was just written to and is not advertising yet.
