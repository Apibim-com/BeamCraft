# Visual bug report closure

The reports in fork issues #19, #20 and #48 describe behavior from the October 19
2026 builds. The current fork contains the fixes and regression coverage below.
This record links each reported behavior to the check that guards it; it does not
claim that every platform or GPU driver has been visually exercised.

## Issue #19

- **Stored quantities above 1 m³:** `crates/layer-engine/src/backend.rs`
  applies a quantity-specific bound instead of the general scalar bound.
  `apps/beamcraft-cli/tests/plate.rs::a_plate_above_one_cubic_metre_is_within_the_stage_budget`
  creates and reopens a 1.025 m³ plate.
- **Circular geometry stretched in wide windows:** `apps/beamcraft-ui/src/app.rs`
  scales both render-target dimensions together.
  `the_render_target_is_clamped_as_a_whole` checks a 5400 px viewport and a
  simulated 2x Retina viewport.
- **Definition display names missing:** `apps/beamcraft-ui/src/tools.rs` puts
  `display_name` first and falls back to the designation or grade when absent.
  Its tests cover both named and legacy definitions.
- **End Z clipped in the member panel:** the left panel uses a fixed 320 point
  width (`apps/beamcraft-ui/src/app.rs`), and the member form lays out its
  coordinate fields within that panel.

## Issue #20

The viewport clamp, quantity limit, camera refit behavior, small-volume message,
definition labels, panel width, empty-store state, name-byte limit and decimal
separator have regression checks in `beamcraft-ui` and the CLI. Its remaining
reference display finding was split into #48 and is covered below.

## Issue #48

`apps/beamcraft-ui/src/scene.rs` builds display geometry from attached reference
meshes. `apps/beamcraft-ui/tests/viewport.rs::an_attached_reference_is_uploaded_and_renders_visible_pixels`
checks that the attached reference reaches the rendered image and that moving it
reuses the source mesh.

Validation run for this closure: `cargo test --locked -p beamcraft-ui` passes,
including the headless render checks available on the current host. The viewport
integration test skips only its GPU pixel assertions when no adapter is
available.
