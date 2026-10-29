# The verification pattern: do not trust the press

Every bridge in this repo will happily report success when the Bot's arm
moved. None of them can tell you whether the button underneath did anything.
A Bot slides off its tape, a heater refuses to start because its door switch
is open, the arm is one millimetre short on a cold morning. If the appliance
matters, close the loop with a measurement.

The cheapest reliable measurement is a **power-monitoring smart plug** on the
appliance's supply. The appliance draws current when it is on and (near) zero
when it is not, and HA already has the sensor. The automation below presses
the Bot and then waits for the current to cross a threshold in the expected
direction. If it does not, it raises an error you can see on a dashboard
instead of silently leaving the thing off.

## Entities you need

| Role                      | Example entity                           | Notes                                                       |
| ------------------------- | ---------------------------------------- | ----------------------------------------------------------- |
| The press                 | `switch.bot_xxxx`                        | From HA's SwitchBot integration (proxy option) or a `rest_command` (Rust option) |
| The supply plug           | `switch.appliance_plug`                  | Any smart plug that reports current or power                |
| The measurement           | `sensor.appliance_plug_current`          | Amps; use `_power` in watts if that is what your plug gives |
| The user's request        | `input_boolean.appliance_toggle`         | A dashboard switch the user flips; the automation resets it |
| Busy indicator            | `timer.appliance_actuating`              | Drives a "working..." tile so a second tap is not tempting  |
| Error banner              | `input_boolean.appliance_error`, `input_text.appliance_last_error` | Conditional card on the dashboard |

The Bot is in **press mode**, so `switch.turn_on` on it is "press once". The
same press toggles the appliance, which is why the automation reads the
current *before* pressing to decide whether it expects a rise or a fall.

## The automation

```yaml
alias: Appliance toggle via SwitchBot Bot
description: >-
  Press the Bot, then confirm the appliance actually changed state by watching
  the supply plug's current. Powers the plug first if it is off.
mode: single
variables:
  # 1 A works for a resistive heater; pick a value between idle and running draw.
  threshold: 1
  was_running: "{{ states('sensor.appliance_plug_current') | float(0) > threshold }}"
triggers:
  - trigger: state
    entity_id: input_boolean.appliance_toggle
    to: "on"
actions:
  - action: timer.start
    target:
      entity_id: timer.appliance_actuating
  - action: input_boolean.turn_off
    target:
      entity_id: input_boolean.appliance_error

  # 1. Make sure the appliance has power at all.
  - if:
      - condition: state
        entity_id: switch.appliance_plug
        state: "off"
    then:
      - action: switch.turn_on
        target:
          entity_id: switch.appliance_plug
        continue_on_error: true
      - wait_for_trigger:
          - trigger: state
            entity_id: switch.appliance_plug
            to: "on"
        timeout: "00:00:10"
        continue_on_timeout: true
      - delay: "00:00:03"   # let the appliance's own electronics boot

  # 2. Bail out early if the Bot is not reachable (proxy offline, out of range).
  - if:
      - condition: state
        entity_id: switch.bot_xxxx
        state:
          - unavailable
          - unknown
    then:
      - action: input_text.set_value
        target:
          entity_id: input_text.appliance_last_error
        data:
          value: "SwitchBot Bot unavailable: Bluetooth proxy offline or Bot out of range."
      - action: timer.finish
        target:
          entity_id: timer.appliance_actuating
      - action: input_boolean.turn_off
        target:
          entity_id: input_boolean.appliance_toggle
      - action: input_boolean.turn_on
        target:
          entity_id: input_boolean.appliance_error
      - stop: Bot unavailable

  # 3. Press. continue_on_error: a failed press is caught by the check below.
  - action: switch.turn_on
    target:
      entity_id: switch.bot_xxxx
    continue_on_error: true

  # 4. Wait for the current to move the way we expect, then judge.
  - wait_for_trigger:
      - trigger: template
        value_template: >-
          {% set a = states('sensor.appliance_plug_current') | float(0) %}
          {{ (a < threshold) if was_running else (a > threshold) }}
    timeout: "00:00:08"
    continue_on_timeout: true
  - action: timer.finish
    target:
      entity_id: timer.appliance_actuating
  - action: input_boolean.turn_off
    target:
      entity_id: input_boolean.appliance_toggle
  - if:
      - condition: template
        value_template: "{{ wait.trigger is not none }}"
    then: []   # success: add a run timer, a notification, whatever you like
    else:
      - action: input_text.set_value
        target:
          entity_id: input_text.appliance_last_error
        data:
          value: "Bot pressed, but the appliance's power draw did not change within 8 s."
      - action: input_boolean.turn_on
        target:
          entity_id: input_boolean.appliance_error
```

### With the Rust REST bridge instead

Replace step 3 with the `rest_command` and keep everything else:

```yaml
  - action: rest_command.switchbot_bot_press
    continue_on_error: true
```

The bridge may take 20 s to answer when it retries; HA waits for the
`rest_command` to return before the `wait_for_trigger`, so the 8 s window
starts late but still measures the right thing.

## Dashboard side

Three conditional cards keyed on a template binary sensor
(`sensor.appliance_plug_current > threshold`) make the state unambiguous:

* "Supply is off" (tap turns on the plug only),
* "Start" (flips `input_boolean.appliance_toggle`),
* a red "Running, tap to stop" (same toggle; the automation knows from
  `was_running` which way it is going).

Plus a tile bound to `timer.appliance_actuating` so the user sees that a press
is in flight, and a conditional markdown card showing
`input_text.appliance_last_error` while `input_boolean.appliance_error` is on.

## Things this pattern does not solve

* An appliance whose idle draw is close to its running draw (standby heaters,
  electronics). Use the power sensor and a wider gap, or a different signal
  (a contact sensor, a temperature rise).
* A Bot in switch mode where on and off are distinct arm positions. Simpler:
  no need for `was_running`, just pick the command.
* A second user tapping during the 8 s window. `mode: single` drops the second
  run; the timer tile is there so they do not tap.
