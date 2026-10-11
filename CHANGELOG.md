# 0.8

- updated to bevy 0.20
- interaction styles and events now use the picking `Hovered` and `Pressed` components
- button interaction styles and events now require the picking feature
- bevy_hui_widgets updated to 0.7.0
- on_ui_enter no longer fires when a press is released
- hover enter/exit events are mouse only, touch no longer triggers them

# 0.4

- updated to bevy 0.16
- new shadow styles for text shadow
- new full rect image border rules.

# 0.3.0

- new atlas & sprite animation system.
- multiple minor bug fixes

# 0.2.0

- added support for `overflow_clip_margin`

- added new `on_changed` function binding attribute for all nodes.
  Does nothing by default. Must be triggered by the user. For notifying change in
  custom widgets.

- added `scroll` to overflow values.

- added `shadow_color`,`shadow_blur`,`shadow_spread` and `shadow_offset` for the new box shadow.
  These values can all be used with conditional styling and easing!

- added `image_region` style attribute. `(float,float)(float,float)` -> maps to `Rect`
  and can be used together with the `image`-Node.

- added new optional `bevy_hui_widgets` sub crate. Provides helper functions for some basic
  widgets. Checkout the new example.

# 0.1.0

- First release
