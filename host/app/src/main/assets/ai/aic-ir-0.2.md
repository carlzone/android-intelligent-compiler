# AIC IR 0.2 model contract

Adaptive activities declare `capability ui.adaptive` and exactly four mutually
exclusive bodies: `on_create for portrait compact`, `portrait expanded`,
`landscape compact`, and `landscape expanded`. Compact means a current window
width below 600dp; expanded means 600dp or wider. Every variant must declare
the same view identifiers, kinds, and order so event and lifecycle bindings are
stable. Declare `capability lifecycle.state_restoration` when Activity scalar
state must survive recreation and process death. Generated code also assigns
stable recreation-order view IDs so Android restores supported widget state;
password input saving is explicitly disabled.

Interactive controls receive an automatic 48dp minimum width and height. A fixed `android.set_layout` width or height below 48 is invalid for buttons, editable inputs, checkboxes, switches, spinners, lists, and toolbars; `wrap_content` and `match_parent` remain valid.

Every `android.image_view` must have exactly one accessibility treatment: a non-empty `android.set_content_description(view: ..., text: ...)` for informative images, or `android.set_decorative(view: ...)` for images that should be omitted from accessibility navigation. Progress controls require a non-empty content description. `android.set_enabled` accepts only interactive controls; `android.set_visibility` accepts every view. Do not emit conflicting constant enabled or visibility assignments in one block.

Bounded resources are declared first in `resources { ... }`. Strings use `string name = "default"` plus optional canonical locale variants and are referenced with `resource.string(name)`. Colors use `color name = "#RRGGBB"` or `#AARRGGBB` and `resource.color(name)`. Project images use `image name project_asset "name.png"` (PNG/WebP) and `android.image_view(resource: resource.image(name))`. At most one `theme app primary color_name accent color_name` and one `launcher_icon image_name` may be declared. Do not invent paths, qualifiers, XML resources, vectors, density variants, or undeclared assets.

Fixed collections use `state items: string[] = ["One", "Two"]` with 1–100 string literals. `android.list_view(items: items)` and `android.spinner(items: items)` accept collection state or an inline list. Handle either widget with `on_select(view, index, value)`; `index` is an immutable zero-based `i32`, `value` is an immutable `string`, and Spinner setup does not invoke the handler.

Literal colors use `android.set_text_color(view: ..., color: "#RRGGBB")` or `android.set_background_color(view: ..., color: "#AARRGGBB")`. Text color targets must be a TextView subclass: `text_view`, `button`, `edit_text`, `text_input`, `check_box`, or `switch`. Background colors accept every declared view. Resource colors, selectors, tinting, and dynamic color expressions are not supported.

Return a complete program beginning with `aic_version 0.2`. A program contains one or more named `activity` blocks; the first is the launcher. Use `android.start_activity(TargetActivity)` only for an activity declared in the same program, `android.finish()` to close the current activity, and the existing closed 0.1 UI and persistence vocabulary documented by the supplied templates. `android.text_input(hint: ..., input_type: ...)` accepts `text`, `email`, `password`, `phone`, or `integer`; omit `input_type` for text. `android.set_text_size(view: ..., size_sp: ...)` accepts an integer from 1 through 200 for TextView, Button, EditText/text_input, CheckBox, and Switch and always uses scalable pixels. `android.list_view` and `android.spinner` accept 1–100 inline string expressions or fixed `string[]` activity state and support typed `on_select` handlers. `android.set_layout` accepts `match_parent`, `wrap_content`, or a bounded `0..4096` integer dimension; optional margins must provide all four fields in left/top/right/bottom order. Every child has at most one parent, containment must be acyclic, and a `ScrollView` has exactly one child. Do not invent APIs, resource names, permissions, capabilities, or syntax. The compiler and `aic.capabilities/0.2` catalog are authoritative. Preserve the package during patch operations and return the entire proposed program through the required JSON schema.
