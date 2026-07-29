# Snapdash

Snapdash mirrors live Home Assistant state onto the desktop as small always-on-top windows, and lets the user act on that state without opening Home Assistant.
This glossary fixes the vocabulary for that mirroring and for the interactions layered on top of it.

## Language

### Entities and widgets

**Entity**:
Home Assistant's unit of state, addressed as `domain.object_id`.
Snapdash never invents entities, it only mirrors them.
_Avoid_: device, thing, sensor (as a general term)

**Widget**:
A frameless always-on-top window bound to exactly one entity.
_Avoid_: card, tile, panel

**Drag handle**:
The widget surface that moves the window, which is the whole card apart from the header icons.
The card is deliberately not split into drag and tap regions, for reasons recorded in `docs/adr/0001-widget-interaction-model.md`.
_Avoid_: title bar, chrome

**Expanded**:
A widget grown downwards out of its size preset to reveal its continuous controls, and shrunk back when they are dismissed.
The controls are part of the widget window rather than a second window of their own.
_Avoid_: popover, dialog, panel, drawer

### Capabilities and actions

**Capability**:
What an entity supports, discovered from its attributes rather than inferred from its domain.
Not every `light` is dimmable and not every `cover` can be positioned, so the domain alone never answers this.
_Avoid_: feature, support flag

**Primary action**:
The single zero-argument service call a widget fires from its header icon.
A toggle, a scene activation or a script run.
_Avoid_: tap action, default action

**Axis**:
One numeric dimension of an entity that is set rather than toggled, such as brightness, white colour temperature, thermostat setpoint or cover position.
An entity may expose several at once, each with its own range reported by Home Assistant.
_Avoid_: slider, analog, channel

**Absent axis**:
An axis Home Assistant is currently reporting as `null`, meaning the device is not driving that dimension at all.
Absent is a reading of the present, not a gap in the record, and it is rendered as such: dimmed, with no knob and no readout.
It is never rendered as the axis minimum, for reasons recorded in `docs/adr/0006-a-null-axis-renders-as-absent.md`.
_Avoid_: missing, unknown, zero, unset

**Armed**:
A widget with confirmation enabled that has taken its first tap and is awaiting a second.
Being armed has a bounded life: it ends on confirmation, on cancellation, on a timeout, or when the pointer leaves the widget.
_Avoid_: pending, confirming

### State reconciliation

**Echo**:
The `state_changed` event Home Assistant broadcasts once a service call has taken effect.
It carries no reference to the call that caused it, so it can only be matched to a send by comparing values.
_Avoid_: ack, response, confirmation

**Pending value**:
A locally set value for one axis that overrides Home Assistant truth while the user is interacting and until reconciliation completes.
_Avoid_: optimistic value, local state

**Settle window**:
The bounded interval after the last send during which a pending value stays authoritative.
It exists because a command Home Assistant clamps or silently drops would otherwise leave the pending value authoritative forever.
_Avoid_: debounce, cooldown, grace period
