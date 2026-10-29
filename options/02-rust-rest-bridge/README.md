# Option 2: Rust REST bridge on a machine that has a radio

A small Rust binary that runs on any Mac or Linux box with Bluetooth, talks to a
SwitchBot Bot over BLE with [`btleplug`](https://crates.io/crates/btleplug)
(CoreBluetooth on macOS, BlueZ on Linux), and exposes `POST /press`, `/on`, `/off`
over HTTP with [`axum`](https://crates.io/crates/axum) so a Home Assistant without
a radio can call it through `rest_command`.

This is the option that was in production for about a month on a laptop that
happened to sit within range of the Bot. It worked, and it was retired. See
[Why it was retired](#why-it-was-retired) before you build on it.

## Build

Requires Rust 1.75 or newer.

```bash
cd options/02-rust-rest-bridge
cargo build --release
./target/release/switchbot-bridge --help
```

On Linux you also need the BlueZ development headers for the `dbus` crate
(`libdbus-1-dev` on Debian/Ubuntu, `dbus` on Arch) and a running `bluetoothd`.

## CLI

```bash
# Find Bots nearby (default 5 s). --all lists every BLE peripheral the OS sees.
switchbot-bridge scan
switchbot-bridge scan --duration 10 --all

# Press the first Bot found, or one matched by a name/ID substring.
switchbot-bridge press
switchbot-bridge press --device <NAME_OR_ID_SUBSTRING>

# Switch mode only: the Bot's arm stays down (on) or retracts (off).
switchbot-bridge on  --device <NAME_OR_ID_SUBSTRING>
switchbot-bridge off --device <NAME_OR_ID_SUBSTRING>
```

The ID shown by `scan` is whatever the OS hands out. On macOS it is a
CoreBluetooth UUID that is stable per Mac but differs between Macs; on Linux it
is derived from the Bluetooth address. Matching is a case-insensitive substring
of either the name or that ID.

## REST server

```bash
switchbot-bridge serve --host 0.0.0.0 --port 8080
```

| Method | Path      | Effect                                           |
| ------ | --------- | ------------------------------------------------ |
| `GET`  | `/health` | `{"status":"ok","service":..., "version":...}`    |
| `POST` | `/press`  | Press (Bot in press mode)                        |
| `POST` | `/on`     | Turn on (Bot in switch mode)                     |
| `POST` | `/off`    | Turn off (Bot in switch mode)                    |

All three POSTs accept `?device=<substring>`. The response is
`{"success": bool, "message": string, "device"?: string}` with `200`, `404`
(no Bot found after retries) or `500` (found it, write failed).

Each request does a fresh scan, connect, service discovery, write, 2 s dwell,
and disconnect, and retries that whole sequence up to three times with a 1.5 s
pause. A Bot that was just written to can take a few seconds to advertise
again, so a request can take 20 to 25 s in the worst case. Set the HA
`rest_command` timeout accordingly (see
[`ha/rest_command.example.yaml`](ha/rest_command.example.yaml)).

There is no authentication. Keep it on your LAN and firewall the port.

## Run it as a service

* macOS: [`packaging/launchd.plist.example`](packaging/launchd.plist.example),
  installed as a user agent (`gui/<uid>`), because CoreBluetooth needs a
  logged-in session.
* Linux: [`packaging/systemd.service.example`](packaging/systemd.service.example).

## macOS: TCC and CoreBluetooth gotchas

These cost more time than the Bluetooth code did.

* **The binary must carry an `Info.plist`.** Since macOS 11, any process that
  touches CoreBluetooth must declare `NSBluetoothAlwaysUsageDescription`, or the
  call fails silently (`adapter_state` stays `Unknown`, scans return nothing).
  A bare command-line binary has no bundle, so [`build.rs`](build.rs) embeds
  [`Info.plist`](Info.plist) into the `__TEXT,__info_plist` section at link
  time. TCC then keys the permission on `CFBundleIdentifier`
  (`io.github.tywrr.switchbot-bridge`).
* **The first run must happen in a GUI session.** The permission prompt is a
  dialog; it never appears over SSH. Run `switchbot-bridge scan` once from
  Terminal.app, click Allow, and from then on launchd and SSH sessions work.
  If you rebuild and the bundle id changes, you get prompted again.
* **Reset a stuck permission:** `tccutil reset Bluetooth io.github.tywrr.switchbot-bridge`.
* **Do not run it as a LaunchDaemon.** System daemons have no TCC context; the
  adapter never reaches `PoweredOn`.
* **Sleep.** A laptop lid closed or a Mac asleep will not scan. Disable sleep
  while plugged in (`sudo pmset -c sleep 0`) or you will debug a bridge that is
  fine and simply asleep.

## Protocol notes

The full write-up is in [`docs/switchbot-bot-protocol.md`](../../docs/switchbot-bot-protocol.md).
Short version:

* Service `cba20d00-224d-11e6-9fb8-0002a5d5c51b`, command characteristic
  `cba20002-...`, notify characteristic `cba20003-...`.
* Press is the three-byte write `57 01 00`; on is `57 01 01`; off is `57 01 02`.
  The bridge writes without response when the characteristic allows it.
* **Password / encryption caveat.** If you set a password on the Bot in the
  SwitchBot app, the plain `57 01 xx` frames are rejected. The passworded
  frame is `57 11 <crc32 of the password, 4 bytes> <action>`. This bridge does
  not implement that; remove the password or extend `bot.rs`. Newer SwitchBot
  devices (Lock, some Plug Minis) use a separate encrypted session protocol
  that this code does not speak at all. Bots work fine without a password.

## Why it was retired

It did the job, but every weakness was structural:

* A general-purpose computer was now infrastructure. Sleep, OS updates, a
  reboot that lands on the login screen before TCC is active, and a laptop
  that someone unplugs all turned into "the switch does nothing".
* Range was whatever the laptop's position gave. Around -80 dBm it started
  needing all three retries.
* HA's native SwitchBot integration was bypassed, so battery level, press
  feedback and the device page were gone; HA saw a `rest_command`, nothing
  more.
* A $6 ESP32-C6 running ESPHome as a Bluetooth proxy
  ([option 3](../03-esphome-bluetooth-proxy/)) gives all of that back, sits
  wherever a USB charger is, and has no operating system to babysit.

Keep this option if you already have an always-on Mac or Linux host with
Bluetooth next to the Bot, want a plain HTTP interface for something other than
HA, or cannot add devices to your Wi-Fi.

## License

MIT or Apache-2.0, at your option.
