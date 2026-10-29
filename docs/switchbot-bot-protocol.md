# SwitchBot Bot BLE protocol (the parts you need to press it)

Everything here is from SwitchBot's published
[SwitchBotAPI-BLE](https://github.com/OpenWonderLabs/SwitchBotAPI-BLE) notes
and from watching what [pySwitchbot](https://github.com/Danielhiversen/pySwitchbot)
(the library behind Home Assistant's integration) sends. It is enough to drive
a Bot from any BLE stack; it is not the whole protocol.

## GATT layout

| Item                   | UUID                                   |
| ---------------------- | -------------------------------------- |
| Primary service        | `cba20d00-224d-11e6-9fb8-0002a5d5c51b` |
| Command characteristic | `cba20002-224d-11e6-9fb8-0002a5d5c51b` (write, write-without-response) |
| Notify characteristic  | `cba20003-224d-11e6-9fb8-0002a5d5c51b` (notify)                        |

The Bot advertises the service UUID, so filtering a scan on it is reliable.
Local names seen in the wild are `WoHand` (older firmware) or a generic
`Bot`/empty name; do not depend on the name.

## Command frames (no password)

A command is a short write to the command characteristic. The first byte is
the "magic" `0x57`, the second selects the command family, the rest is the
payload.

| Action          | Bytes      | Notes                                              |
| --------------- | ---------- | -------------------------------------------------- |
| Press           | `57 01 00` | Press mode: extend the arm, retract it             |
| Turn on         | `57 01 01` | Switch mode: arm down and hold                     |
| Turn off        | `57 01 02` | Switch mode: arm up                                |
| Push (hold)     | `57 01 03` | Arm down, no auto-retract (rarely useful)          |
| Pull (release)  | `57 01 04` |                                                    |
| Get settings    | `57 02`    | Response arrives on the notify characteristic      |

In press mode `57 01 01` and `57 01 02` both behave like a press. The mode is
set in the SwitchBot app, not over this interface (there is a settings frame
for it, but HA and this repo leave it alone).

The sequence that works everywhere:

1. Connect, discover services.
2. Optionally subscribe to notify (`cba20003`) if you want the one-byte status
   reply; `0x01` means OK.
3. Write the three bytes to `cba20002`. Write-without-response is fine and is
   what pySwitchbot uses.
4. Wait about 1 to 2 s for the arm to move, then disconnect. Disconnecting
   immediately after the write has caused missed presses on some firmware.

## Password

If you set a password in the SwitchBot app, the Bot ignores the plain frames
above. The passworded frame replaces command family `01` with `11` and inserts
the CRC32 of the password string (big-endian, 4 bytes) before the action:

```
57 11 <crc32(password)> <action>
      ^^^^^^^^^^^^^^^^  4 bytes
```

So a press with password `abc` is `57 11 35 24 41 c2 00`. pySwitchbot computes
it as `binascii.crc32(password.encode()) & 0xFFFFFFFF` and formats it as 8 hex
digits. The Rust bridge in this repo does not implement passwords; Home
Assistant's SwitchBot integration does.

## Encryption

The Bot does not encrypt its link beyond the password CRC. Other SwitchBot
devices do: the Lock, the Lock Pro, some newer Plug Mini and Curtain firmware
use a session-key handshake derived from a per-device key and the user's
cloud account. Those devices cannot be driven with the frames here and need
the cloud-issued key material (pySwitchbot's `SwitchbotEncryptedDevice`).
This repo is about the Bot only.

## Advertisement payload

The Bot's service data (on the 16-bit UUID `0x0D00` under company/service
`0xFD3D` on newer firmware, or the 128-bit service above) carries:

| Byte | Meaning                                                                 |
| ---- | ----------------------------------------------------------------------- |
| 0    | Bit 7: encryption flag; bits 6:0: device type, `H` (`0x48`) for Bot     |
| 1    | Bit 7: switch mode (0 = press/one-state, 1 = on/off); bit 6: state (inverted: 0 = on); bit 5: encryption type |
| 2    | Bit 7: sync-UTC flag; bits 6:0: battery percentage                      |

(Per SwitchBot's published BLE spec and pySwitchbot's Bot advertisement parser.)

HA shows the battery sensor and the "on" state straight from this, which is
why the ESPHome proxy option gives you battery for free: it needs no
connection, only advertisements.

## Timing you will run into

* After a write and disconnect, the Bot can take 2 to 6 s before it advertises
  again. A second command sent immediately often fails at discovery, not at
  the write. Retry discovery rather than the write.
* Connection setup through a proxy adds a few hundred milliseconds; through
  CoreBluetooth on a sleepy Mac it can add seconds.
* The arm takes about 0.5 s to extend and 0.5 s to retract. The target button
  may need a longer press than that (the SwitchBot app can lengthen it per
  Bot).
