# Option 1: a Bluetooth adapter on the Home Assistant host

The boring option, and the right one when it is available: give the machine
running Home Assistant a radio and let HA's own Bluetooth stack and SwitchBot
integration do everything. No bridge, no second device, nothing to maintain.

This folder has no code because there is nothing to write. It documents the
two shapes this takes and why the second one was researched and rejected for
the setup that produced this repo.

## 1a. Home Assistant OS with an onboard adapter (tested, for a year)

Most Intel NUCs and many mini PCs ship with a combined Wi-Fi/Bluetooth card.
HAOS detects it, the **Bluetooth** integration appears by itself, and a
SwitchBot Bot in range is offered as a discovered device within a minute.

What worked:

* Press, battery, RSSI, everything the SwitchBot integration offers.
* Zero configuration beyond clicking "Add" twice.

What to know:

* Range is wherever the server lives, and the Bot's coin-cell radio is the
  weak end. The verification pattern in
  [`docs/verification-pattern.md`](../../docs/verification-pattern.md) was
  first written for this setup, because presses at the edge of range
  sometimes did not register.
* Intel combo cards share the antenna between Wi-Fi and BLE. If the host
  also uses Wi-Fi heavily, BLE scans get patchy. Wired Ethernet for the host
  fixes it.
* When HA moves off HAOS (to a container, a VM, Kubernetes), this option goes
  with it unless you can pass the adapter through. That is what the rest of
  this repo is about.

## 1b. USB dongle passed into a container or pod (researched, not tested)

If HA runs as a container on a Linux host, a USB Bluetooth dongle on that
host can in principle be used by HA. It was the first thing considered after
leaving HAOS. It was not deployed, so treat the following as a reading of
the documentation and issue trackers rather than as experience.

### What HA needs

HA's Bluetooth integration talks to BlueZ over D-Bus and, for adapter
recovery, uses raw HCI sockets. In a container that means:

1. **The host runs `bluetoothd`** (BlueZ) with the dongle attached, and the
   container gets the host's system D-Bus socket:
   `-v /run/dbus:/run/dbus:ro`.
2. **`network_mode: host`** (or `hostNetwork: true` in Kubernetes). BlueZ
   HCI sockets are `AF_BLUETOOTH` sockets in the host's network namespace;
   a bridged container cannot open them.
3. **`privileged: true`**, or at minimum the `NET_ADMIN` and `NET_RAW`
   capabilities, for the adapter reset path. Without them HA logs a
   `habluetooth` warning on startup and cannot recover a wedged adapter,
   though normal operation can still work.
4. The host kernel must not have another consumer of the adapter (a desktop
   session's Bluetooth applet will fight HA for scans).

HA's own documentation describes this for Docker; it is the supported path
for "Container" installs. The example Compose shape:

```yaml
services:
  homeassistant:
    image: ghcr.io/home-assistant/home-assistant:stable
    network_mode: host
    privileged: true
    volumes:
      - ./config:/config
      - /run/dbus:/run/dbus:ro
```

### In Kubernetes

The same constraints map to a pod with `hostNetwork: true`, a `hostPath`
mount of `/run/dbus`, and `securityContext.privileged: true` (or the two
capabilities). The namespace then has to be Pod Security `privileged`, the
Deployment needs `strategy: Recreate` because two replicas cannot share the
host's HCI device, and the pod is pinned to the one node that has the dongle
(a `nodeSelector` on a label you add by hand). BlueZ must run on the node
itself; running `bluetoothd` inside the pod is possible but means the pod
owns the adapter outright and the node's own stack must be disabled.

### Why it was rejected here

* The HA host in question (an ARM single-board computer) had no Bluetooth
  and sat a floor away from the Bot, too far for a dongle on it to help. A
  long USB extension cable was the honest version of this plan.
* The pod was already `hostNetwork` and privileged for Matter and mDNS, so
  the security cost was already paid, but adding a USB device made the pod
  non-schedulable anywhere else and tied Bluetooth recovery to node reboots.
* A $6 ESP32-C6 running ESPHome ([option 3](../03-esphome-bluetooth-proxy/))
  gives HA a Bluetooth adapter over Wi-Fi, placed next to the Bot, with no
  change to the pod spec at all. Once that was understood, the dongle lost on
  every axis except "no extra device on the Wi-Fi".

### If you do try it

* Prefer a dongle with a known-good Linux chipset (CSR8510 clones are hit and
  miss; Intel AX200-era and Realtek 8761B are fine).
* Check `bluetoothctl list` on the host first, then inside the container.
* Watch for the host's own desktop or `bluetooth.service` grabbing the
  adapter after a reboot.
* The `habluetooth` capability warning is informational if presses work.

Reports of this working in a pod would be welcome as a pull request to this
file.
