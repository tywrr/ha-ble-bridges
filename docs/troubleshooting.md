# Troubleshooting

Collected from a year of pressing one button. Grouped by where the problem
usually turns out to be.

## Radio and range

**RSSI.** The Bot's HA device page (proxy option) or `switchbot-bridge scan`
(Rust option) shows it. Rough guide from this setup:

| RSSI (dBm) | What happened                                                     |
| ---------- | ----------------------------------------------------------------- |
| > -70      | Every press worked first try                                      |
| -70 to -80 | Presses worked, discovery after a write sometimes needed a retry  |
| < -85      | Discovery timeouts, "device unavailable" flapping                 |

Move the proxy or bridge host, not the Bot. A Bot is a coin cell behind a
plastic shell stuck to an appliance; its radio is the weak end. One room away
through a stud wall was fine; two rooms or a floor was not.

**Neighbours' devices in scans.** `scan --all` and HA's Bluetooth page will
show every advertising device in range, including things in the flat next
door. Filter on the SwitchBot service UUID
(`cba20d00-224d-11e6-9fb8-0002a5d5c51b`) rather than on names, and when you
have two Bots in range, match on the ID substring, never on "the first one".

**Microwave ovens and 2.4 GHz Wi-Fi** share the band with BLE. A press
attempted while the microwave runs can fail; the retry in the bridge and in
HA's integration covers it.

## ESP32-C6 proxy

**It never joins Wi-Fi.** The C6 is 2.4 GHz only. A 5 GHz or 6 GHz only SSID
is invisible to it. Look for the fallback AP (SSID equals the device name) in
your phone's Wi-Fi list; if it is there, the device booted and could not find
your network.

**Wrong credentials but no error.** Same symptom as above. Reflash over USB
after fixing `secrets.yaml`; the fallback AP's captive portal also accepts new
credentials.

**`Permission denied: /dev/ttyACM0`.** Missing udev rule for Espressif vendor
`303a`; see the option 3 README. Log out and in after adding your user to a
group.

**Board not detected.** Hold BOOT while plugging in. Some USB-C cables are
power only.

**HA shows the proxy but no Bot.** Check the proxy's `bluetooth_proxy` has
`active: true`, then check RSSI. HA only offers the SwitchBot integration once
it has seen an advertisement with the Bot's service data.

**Bot flaps between available and unavailable.** Usually range. Also happens
for a few seconds after every press, which is normal: the Bot stops
advertising while connected and for a moment after.

**`habluetooth` warning about `NET_ADMIN`/`NET_RAW`.** Seen when HA runs in a
container without those capabilities. It concerns resetting a *local* USB
adapter and does not affect proxies. Ignore it unless you also have a local
adapter.

## Rust bridge on macOS

**`adapter_state` stays `Unknown` / scans return nothing.** TCC has not
granted Bluetooth. Causes, in order of likelihood:

1. The binary has no embedded `Info.plist` (rebuilt without `build.rs`?).
   `otool -s __TEXT __info_plist target/release/switchbot-bridge | head`
   should print bytes.
2. You never ran it in a GUI session. BLE over SSH on macOS does not prompt;
   it just fails. Open Terminal.app on the Mac itself, run
   `switchbot-bridge scan`, click Allow.
3. It is running as a LaunchDaemon (system context) instead of a user agent
   (`gui/<uid>`).
4. The bundle id changed since permission was granted. `tccutil reset
   Bluetooth <bundle id>` and prompt again.

Check with: `sqlite3 ~/Library/Application\ Support/com.apple.TCC/TCC.db
"select client, auth_value from access where service='kTCCServiceBluetoothAlways'"`
(needs Full Disk Access for your terminal).

**Works in Terminal, fails from launchd.** The launchd job inherits no
`PATH`; use absolute paths in `ProgramArguments`. Also check that the Mac is
not asleep: `pmset -g` and set `sleep 0` while on power.

**Discovery flakiness.** CoreBluetooth caches peripherals and does not always
re-list a device that just disconnected. The bridge retries three times with
a 1.5 s pause for exactly this reason. If you see the third attempt succeed
regularly, the Bot is at the edge of range.

**Two Bots, wrong one pressed.** Pass `?device=` with a distinctive substring
of the name or ID. Names on macOS can be empty for a Bot that has not been
connected to recently; use the UUID.

## Rust bridge on Linux

**`No Bluetooth adapter found`.** `bluetoothctl list` must show one. In a
container you need the host's D-Bus system bus mounted
(`/run/dbus/system_bus_socket`) and the user allowed in
`/etc/dbus-1/system.d/bluetooth.conf`.

**`org.bluez.Error.Failed: Software caused connection abort`.** The adapter
is busy with another connection or scan. Stop other BLE consumers
(`bluetoothctl scan off`) and make sure only one bridge runs.

**Writes succeed but nothing moves.** Password set on the Bot. Remove it in
the app or implement the `57 11 ...` frame.

## Home Assistant automation

**Press reported, appliance did nothing.** This is why the verification
pattern exists. Check the Bot's tape and arm length first, then whether the
appliance needs a longer press than the Bot's default.

**The 8 s window is too short.** Heaters with a soft start, or appliances with
a fan delay, can take longer to draw current. Lengthen the `wait_for_trigger`
timeout; keep it under whatever a human would consider "stuck".

**`rest_command` timeout errors.** The default is 10 s and the bridge may take
longer when it retries. Set `timeout: 30`.
