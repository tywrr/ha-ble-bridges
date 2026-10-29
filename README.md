# ha-ble-bridges

**Four ways to press a SwitchBot Bot from a Home Assistant that has no Bluetooth radio.**

A SwitchBot Bot is a battery-powered finger that pushes a button. It speaks
Bluetooth Low Energy and nothing else. Home Assistant drives it beautifully,
as long as HA can hear it. This repo is what happened when HA moved to a
machine that could not, written up so you can skip straight to the answer.

```
options/01-local-adapter/           HAOS onboard radio, or a USB dongle into a container/pod
options/02-rust-rest-bridge/        Rust btleplug + axum bridge on a Mac/Linux box with a radio
options/03-esphome-bluetooth-proxy/ ESP32-C6 running ESPHome as a Bluetooth proxy  <- pick this
docs/                               Bot protocol, the verification pattern, troubleshooting
```

## The story

1. **HAOS on an Intel NUC.** The NUC had an onboard Bluetooth adapter. HA's
   Bluetooth integration found it, the SwitchBot integration found the Bot,
   done. It ran like that for a few months. The only lesson from this period
   was that a press at the edge of range sometimes did not land, which is
   where the [verification pattern](docs/verification-pattern.md) comes from.
2. **HA in a container on a Kubernetes node, no radio.** The move off HAOS put
   HA in a pod on an immutable Kubernetes OS with no BlueZ (the onboard radio
   was unusable), then on an ARM board with no radio at all. Either way the
   host was a floor away from the Bot. Passing a USB dongle into the pod was
   [researched](options/01-local-adapter/README.md#1b-usb-dongle-passed-into-a-container-or-pod-researched-not-tested)
   and rejected: it needs `hostNetwork`, privileged, the host's D-Bus socket,
   pins the pod to one node, and the dongle would still have been a floor
   away.
3. **A laptop as a REST bridge.** A MacBook that lived within range got a
   [small Rust service](options/02-rust-rest-bridge/): `POST /press` over
   HTTP in, a three-byte BLE write out, with retries. HA called it through
   `rest_command`. It worked for six weeks and taught a lot about macOS TCC.
   It was also a laptop, so it slept, updated, and got unplugged.
4. **A $6 ESP32-C6 running ESPHome as a Bluetooth proxy.** Powered from the
   HA host's USB port (still a floor from the Bot, about -80 dBm), it shows up
   in HA as a Bluetooth adapter. Put it next to the Bot if you can. HA's
   native SwitchBot integration comes back, with battery and RSSI, and the
   laptop goes back to being a laptop. [This is the one.](options/03-esphome-bluetooth-proxy/)

## Comparison

| | 1a. HAOS onboard adapter | 1b. USB dongle in container/pod | 2. Rust REST bridge | 3. ESPHome proxy (ESP32-C6) |
| --- | --- | --- | --- | --- |
| Press latency (request to arm moving) | ~1 s | ~1 s (expected) | 2 to 6 s, up to 25 s with retries | 1 to 2 s |
| Reliability in this setup | Good when in range; range is the server's location | Untested | Good while the host is awake; host sleep/updates were the failure mode | Best so far, even a floor away; a proxy can go wherever there is USB power |
| Extra hardware cost | $0 if the host has a radio | $10 to $20 dongle (+ extension cable) | $0 if you already own a Mac/Linux box near the Bot | $6 to $10 |
| Setup effort | None | Medium: BlueZ on host, D-Bus mount, privileged, hostNetwork, node pinning | Medium: build, TCC dance, launchd/systemd, `rest_command` | Low: flash once over USB, accept in HA |
| Works without cloud | Yes | Yes | Yes | Yes |
| Keeps HA's native SwitchBot integration | Yes | Yes | **No** (HA sees only a `rest_command`; no battery, no feedback) | Yes |
| Needs the HA pod/container changed | No | Yes | No | No |
| Something else to keep alive | No | The dongle and host BlueZ | A general-purpose OS | A microcontroller with no OS |
| Status in this repo | Tested (a few months) | Researched, not tested | Tested (six weeks), retired | Tested, in production since late October 2026 |

Latency figures are rough observations from a handful of presses in this
installation, with the Bot at -70 to -80 dBm, not measurements. Yours will
differ; the ordering probably will not.

## Which should I pick

* **HA has a radio and the Bot is within range of it:** option 1a. Do
  nothing. Add the verification pattern if the button matters.
* **HA is in a container or pod on a box with a radio, and the Bot is in
  range of that box:** option 1b is legitimate. Expect to make the pod
  privileged and `hostNetwork`. If you are already doing that for Matter or
  mDNS, the marginal cost is low.
* **HA is nowhere near the Bot, or has no radio:** option 3. An ESP32-C6
  board, ESPHome, ten minutes. This is the default answer.
* **You cannot put another device on your Wi-Fi, or you want a plain HTTP
  interface for something that is not HA, and you have an always-on Mac or
  Linux machine near the Bot:** option 2. Read its
  [retirement notes](options/02-rust-rest-bridge/README.md#why-it-was-retired)
  first.
* **The Bot has a password set:** options 1 and 3 handle it through HA's
  integration; option 2 does not implement the passworded frame.

## Do not trust the press

Every option above reports success when the Bot's arm moved. None of them
knows whether the button did anything. If the appliance matters, put a
power-monitoring plug on its supply and have the automation wait for the
current to change:

```yaml
- action: switch.turn_on          # press
  target: { entity_id: switch.bot_xxxx }
  continue_on_error: true
- wait_for_trigger:               # confirm
    - trigger: numeric_state
      entity_id: sensor.appliance_plug_current
      above: 1
  timeout: "00:00:08"
  continue_on_timeout: true
- if: "{{ wait.trigger is none }}"
  then: ...raise an error the user can see...
```

The full automation, with the pre-power step, the unavailable check and the
on/off direction logic, is in [`docs/verification-pattern.md`](docs/verification-pattern.md).

## Docs

* [`docs/switchbot-bot-protocol.md`](docs/switchbot-bot-protocol.md): GATT
  UUIDs, the `57 01 xx` frames, the password CRC, advertisement bytes.
* [`docs/verification-pattern.md`](docs/verification-pattern.md): the
  automation above in full, plus the dashboard that goes with it.
* [`docs/troubleshooting.md`](docs/troubleshooting.md): RSSI, 2.4 GHz-only
  boards, BLE over SSH on macOS, discovery flakiness, neighbours' devices.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at
your option. The ESPHome config is derived from the
[esphome/bluetooth-proxies](https://github.com/esphome/bluetooth-proxies)
templates (Apache-2.0).
