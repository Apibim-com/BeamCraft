# Element interface: how the platform stops naming steel kinds

Contributor-original BeamCraft documentation, MPL-2.0. Design note for slice 1 of
[#795](https://github.com/Apibim-com/BeamCraft/issues/795), read against `master` at `94203bbd`
(2026-11-01). Nothing here is implemented: the note fixes the shape of the seam and the order of
the code slices. It continues [`platform-and-steel.md`](platform-and-steel.md), whose "What is
left" section is the starting point, and the owner's comment of 2026-10-31 on the issue.

Fixed by the issue and not reopened here: the stored `steel:*` attributes, the schema descriptor
and every store written today stay byte-identical; refusal codes and the output of existing
commands do not change.

## 1. The problem in one paragraph

The steel code is one crate, `steel`, but `layer-engine` and the CLI still branch on the kind of
element, so they depend on it and the crate cannot become kind `discipline` (nothing but an app
may depend on a discipline). Reading the code shows that these are **two different branch
points**, with two different fixes: the operations on a typed element (copy, edit, mirror,
array, evaluate) and the persistence of an element in the USD layers.

## 2. What the code does today

### 2.1 Typed operations: symmetric pairs

Every operation exists once per kind, with the same shape: the source element plus a command in,
the new element out.

| Operation | Plate | Member | Source |
|---|---|---|---|
| copy | `copy_plate(&AuthoredPlate, &CopyElement) -> AuthoredPlate` | `copy_member(&AuthoredMember, &CopyElement) -> AuthoredMember` | `steel/engine/copy.rs:148`, `:221` |
| copy as instance (P11) | `copy_plate_instance` | `copy_member_instance` | `steel/engine/instance.rs:131`, `:141` |
| edit | `patch_plate(&AuthoredPlate, &ElementPatch)` | `patch_member(&AuthoredMember, &ElementPatch)` | `steel/engine/update.rs:98`, `:160` |
| mirror, local part | `mirror_plate_local(&mut AuthoredPlate)` | `mirror_member_local(&mut AuthoredMember)` | `steel/engine/transform.rs:312`, `:337` |
| array | `array_plate_copies` | `array_member_copies` | `steel/engine/transform.rs:440`, `:466` |
| evaluate body | `BodyCache::plate`, `evaluate_committed*` -> `PlateRecord` | `BodyCache::member`, `evaluate_committed_members*` -> `MemberRecord` | `steel/engine/committed.rs:108`, `:127`, `:431-541` |
| quantity signatures | `plate_*_signature`, `plate_record` | `member_*_signature`, `member_record` | `steel/engine/quantity.rs:162-462` |

Placement-only work is already generic: `rotate_placement`, `mirror_placement`,
`array_placements` (and its helper `stepped_placement`, `:381`) take a `Placement`, not an element
(`transform.rs:187`, `:246`, `:427`). The per-kind part is only what the symmetry does to the
local shape records (cuts, thickness side, section placement, path).

The platform names the pairs in exactly these places, so the interface has few callers:

- `apps/beamcraft-cli/src/copy.rs:66` (`CopiedElement::{Plate, Member}`), `:98`
  (`creation_changes`), `:267-279`;
- `apps/beamcraft-cli/src/array.rs:160`, `:171`;
- `apps/beamcraft-cli/src/edit.rs:166-169` (`Target::{Plate, Member}`), `:191`, `:209`, `:324`,
  `:335`;
- `apps/beamcraft-cli/src/plate.rs:1471-1516` (the `BodyCache` and the two evaluations);
- the decoders (`Decoded`) and `registry.rs` for the command inventory.

`ElementPatch` (`steel/model/command.rs:3648`) carries one set of fields for both kinds. The
generic ones are `name`, `class`, `material`, `level`, `translation_mm`, `orientation_xyzw`,
`rotate`, `mirror`, `move_mm`. The kind-specific ones are `corners`, `contour_points_mm`,
`thickness_mm`, `thickness_side` (plate) and `axis_points_mm`, `profile` (member). Each pair
function already refuses the other kind's half with `UNSUPPORTED_PROPERTY`
(`update.rs:98-175`), and `effect(&ElementPatch)` (`update.rs:71`) derives which groups change.
The patch enters the content fingerprint `BeamCraft/P04/UpdateElement/v2` (`command.rs:3913`).

### 2.2 Persistence: reading is neutral, writing is not

- **Reading is already neutral.** `layer-engine/src/authored/validate.rs` checks `Prim`s by
  attribute lookup (`plate_holes(&Prim)`, `plate_hole_records`, `member_hole_records`,
  `weld_properties`, `bolt_properties`, `bolt_group_properties`). `rebase.rs:49`
  (`property_group`) already maps attribute names to the semantic write groups as strings.
- **Writing is a closed enum.** `Shape` (`authored.rs:285`: `Plate`, `Member`, `LoftedPlate`,
  `Instance`, `Weld`, `Bolt`, `BoltGroup` beside `Assembly`, `Grid`, `Construction`,
  `Annotation`, `Level`) and `Change` (`authored.rs:630`: `Contour`, `Thickness`,
  `ThicknessSide`, `Corners`, `Bends`, `Cuts`, `Axis`, `Roll`, `Profile`, `HoleRecords`,
  `MemberHoles`, `BoltGroup` beside `Label`, `Mesh`, `Delete`, `Level`, `Mark`, `Attr*`) carry
  typed steel fields. The same variants are matched in `edit.rs` (`create` `:634`, kind token
  `:658-668`, steel arms `:699-1039`, `capture` `:2039`, `author_corners` `:2517`,
  `author_bends` `:2581`), `validate.rs` (create arms `:2712-2978`, finiteness `:3263-3357`),
  `index.rs:421` (`"plate" | "beam" | "column"`) and `rebase.rs`.
- **The schema descriptor** (`SCHEMA_DESCRIPTOR`, `authored.rs:40`) is one byte string in which
  steel text and platform text alternate: the plate, cut, hole and member blocks are steel;
  the assembly, grid, level, construction and annotation blocks are platform; yet platform lines
  also name steel tokens (`kinds=plate|beam|column` in the part line, `bim:instanceOf`,
  `bim:level` and the numbering line list plate, beam and column).

### 2.3 The kind vocabulary

`ElementKind` (`engine/src/element.rs:26`) is the **closed F10 registry**: sixteen kinds,
including the reserved `Wall`, `Slab`, `Pipe`, ... F10 is frozen at 0.1.0
([`f10-domain-vocabulary-v0.1.md`](../contracts/f10-domain-vocabulary-v0.1.md) section 2.1: "the
registry is closed for a vocabulary version"). The enum is a vocabulary, not behaviour, and
stays. What the platform must not do is **branch on Plate, Beam or Column to decide behaviour**;
today that happens in `query/pick.rs:133` (the `Parts` switch), `:1503` (member axes must be a
beam or column), `query/snapshot.rs:235` (the `extrusion:` role length), and in
`model/src/prim.rs:43` (`from_kind`: plate, beam, column are the `Part` category) and `:58`
(the `/World/Steel/...` prefixes). The rest of the matches in `bom.rs` and `pick.rs` are tests.

## 3. Decisions

**D1. Two seams, not one big trait.** The issue asks to compare the full interface with the
narrow form from the owner's comment (steel converts its typed elements to a neutral prim
description that `layer-engine` writes and reads). The code says to use both, each where it fits:
the narrow form for persistence, a small trait for the typed operations. A single trait that did
both would force `layer-engine` to know the typed element, which is the dependency to remove.

**D2. Seam A (persistence): the neutral description lives in `model`.** `model` is below both
`layer-engine` and `steel`, so no back-edge appears. A discipline registers, per element
category:

- the attribute-name -> write-group table (today the `steel:*` arms of `rebase.rs:49-135`);
- the author function that turns a change into attribute writes (today `author_corners`,
  `author_bends`, `author_cuts`, the arms of `create` and `Change::*` in `edit.rs`);
- the validators over a `Prim` (today the steel arms of `validate.rs`);
- the kind/category rows (`prim.rs`, `index.rs:421`).

`Shape` and `Change` keep their platform variants and gain one neutral variant each: a category,
a label, a placement and a list of named attribute values (create), and a list of named attribute
replacements with their group (edit). The author function of the discipline produces that list
from its typed element, so the order of attribute writes is the order of today's code.

**D3. Seam B (typed operations): a trait object per element kind, in `engine`.**

```rust
// engine::element_ops (pseudo-signatures; the names are settled in slice 2)
pub trait ElementKindOps: Send + Sync {
    fn kind_token(&self) -> &'static str;                 // "plate", "beam", "column"
    fn category(&self) -> ElementCategory;                // the prim category
    fn copy(&self, e: &dyn Element, c: &CopyElement, as_instance: bool)
        -> Result<Box<dyn Element>, CommandError>;
    fn apply(&self, e: &dyn Element, p: &ElementPatch)
        -> Result<Box<dyn Element>, CommandError>;
    fn mirror_local(&self, e: &mut dyn Element) -> Result<(), CommandError>;
    fn array(&self, e: &dyn Element, c: &ArrayElements)
        -> Result<Vec<Box<dyn Element>>, CommandError>;
    fn evaluate(&self, e: &dyn Element, ctx: &EvalContext)
        -> Result<EvaluatedElement, CommandError>;         // body + quantities
    fn footprint(&self, e: &dyn Element) -> Footprint;
    fn to_description(&self, e: &dyn Element) -> ElementDescription;   // seam A
    fn from_prim(&self, p: &Prim, ctx: &ReadContext) -> Result<Box<dyn Element>, CommandError>;
}
```

`Element` is the small common view the platform needs (id, name, class, material, placement,
level); the concrete `AuthoredPlate` and `AuthoredMember` implement it and are downcast only
inside `steel`. The interface is the **intersection of the pairs in section 2.1**, nothing more:
copy, edit, mirror, array, evaluate, footprint and the two conversions. What exists for one kind
only (contour edits, corners, bends, lofted plates, welds, bolts, fabrication, numbering
geometry) is not in the interface: it stays `steel::engine` and is reached from steel's own
commands.

**D4. `ElementPatch` splits in two.** The platform keeps the generic fields; the kind-specific
fields become a discipline-owned payload that the discipline's own argument decoder builds and
its `apply` reads. Requirements:

- the refusals stay: a payload that does not belong to the kind is `UNSUPPORTED_PROPERTY` with
  the same field name and message as today (`update.rs:98-175`);
- the content fingerprint `BeamCraft/P04/UpdateElement/v2` keeps its byte layout: the platform
  writes the generic fields where it does today, and the payload writes its fields through a
  `fingerprint_into` method in today's order, so a replayed operation compares equal;
- `effect()` keeps its meaning; its `shape` flag is asked of the payload.

**D5. Registration is explicit.** `steel::register(&mut Disciplines)` is called by the apps
(`beamcraft-cli`, `beamcraft-ui`) and the registry is passed down to `engine`, `layer-engine`
and the CLI pipeline. A process-wide `OnceLock` is rejected: it makes tests depend on order,
hides the dependency from the F08 table and cannot be built twice in one test binary with
different sets.

**D6. The descriptor is composed from ordered segments.** Because steel and platform text
alternate (section 2.2), the descriptor becomes a list of segments, each owned by the platform
or by a discipline, and the registry concatenates them in the **present order**. For the steel
registry the bytes equal today's constant; a test pins the old literal byte for byte and the
digest next to it, in addition to the existing descriptor-pin tests. A registry without steel
would legitimately produce another descriptor, which is correct: another schema. Where a
platform line names steel tokens (`kinds=plate|beam|column`), the platform segment is a
template filled from the registered kind tokens in the registration order, again byte-equal for
steel.

**D7. `ElementKind` stays; behaviour moves to properties.** Slice 4 does not make the F10
registry data. It adds kind **properties** answered by the registered discipline (`is_part`,
`is_linear_member`, `extrusion_role_applies`) and replaces the three behaviour matches of
section 2.3 with them. `prim.rs` `from_kind` and `prefix` take the part category rows from the
registry.

## 4. Slice order

The crate `steel` already exists, so the issue's slice 7 shrinks to flipping its kind. Each code
slice is its own PR, the whole suite green, the `tests/fixtures/f07` read-back and the descriptor
pin tests unchanged, command output unchanged.

| # | Slice | Closed by |
|---|---|---|
| 1 | This note | review |
| 2 | Seam B: `ElementKindOps` in `engine`, implemented in `steel` for Plate and Member; the CLI `copy.rs`, `array.rs`, `edit.rs` and the evaluation in `plate.rs` call it; `CopiedElement` and `Target` become one type | no `*_plate`/`*_member` pair is called outside `steel`: `rg "(copy|patch|array)_(plate|member)"` finds only `steel` |
| 3 | `ElementPatch` split (D4) | the fingerprint test vectors of `UpdateElement/v2` unchanged; refusal messages unchanged |
| 4 | Kind properties (D7) and the category rows from the registry | `rg "ElementKind::(Plate\|Beam\|Column)"` finds no behaviour match in `engine/src` |
| 5 | Seam A in `layer-engine`: neutral variants, registered author functions, validators and group tables; the descriptor segments (D6) | `rg "steel:" crates/layer-engine/src` finds nothing; descriptor literal and digest pinned equal |
| 6 | CLI: platform commands stay, steel commands and registry entries move to `steel` | `registry.rs` lists platform commands only |
| 7 | F08 table: `steel` becomes `discipline`; the `layer-engine -> steel` and the CLI-platform edges go | `beamcraft-boundaries` passes; `rg "crate::steel\|beamcraft_(model\|kernel)::steel"` finds nothing outside the steel subtrees |

Slices 2 and 3 can be one PR if the patch split is small once the trait exists; the grep in the
last column is the judge, not the PR count. Slice 5 is the largest (about two thirds of the
6,200 lines of `validate.rs` and `edit.rs`) and may split by element family: plate and cut records,
then member and holes, then welds and bolts.

## 5. Risks

- **Hot paths.** `BodyCache` and `evaluate_committed*` run per element on every inspection. A
  trait object per call is not a measurable cost against a kernel evaluation, but the dispatch
  must be per kind once per batch (resolve the `ElementKindOps` once, evaluate the slice), not
  per element. Slice 2 measures `query_bench` before and after.
- **Down-casting.** `dyn Element` hides the concrete type; a wrong down-cast must be an internal
  error with a stable code, never a panic (`AGENTS.md` section 4: no `unwrap`, `expect` or
  indexing in non-test code).
- **Fingerprints and journals.** Command fingerprints are stored in the journal. A changed
  layout would make a replayed operation `OP_ID_REUSE`. D4 keeps the layout; slice 3 adds a test
  that fixes the bytes of one patch per field before the change.
- **Attribute write order.** The author function must write attributes in today's order or the
  `authored.usda` bytes change. The check is the existing byte-level store comparisons plus a
  golden comparison of the `f07` fixtures.
- **The descriptor.** Segmentation is a pure refactor of one constant; a mistake changes the
  pinned identity and every store written today is refused. D6's byte-for-byte test goes in
  with the first segment, before any other change in slice 5.
- **Test fixtures.** Platform tests that use a plate body as a neutral body (the box primitive
  that stayed in `kernel`, see `platform-and-steel.md`) keep it; they must not need the registry.
- **A second discipline.** The interface is cut from the plate and member pairs only. When a
  second discipline lands, expect to widen it; that is a platform change by design, not a break.

## 6. Out of scope

- A new element kind or a command: no behaviour, vocabulary or stored format changes.
- Making the F10 registry data-driven: it is a frozen contract; a change needs a new vocabulary
  version.
- The command registry and the long-lived session (`architecture.md`, "The window"): they use
  this seam, they are not part of it.
- Moving `layer-engine` below `steel`: the owner decision of 2026-10-31 keeps it above until
  slice 5 is done, then slice 7 removes the edge.
