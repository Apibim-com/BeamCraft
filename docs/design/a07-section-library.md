# A07 section families, environments and profile libraries

Date: 2026-10-18. Status: **A07.1 implemented as an experimental slice (section 9); A07.2 and
A07.3 are design only. Owner decisions D1 and D2 of section 7 are taken; D3 and D4 are open.** Branch `feat/a07-section-library`, based on `origin/master` `d893f94`.

Plan row: A07, the I-section families named by the P02.2 note, widened by the owner on
2026-10-18. The owner's goals:

1. The user never types section dimensions to place a member. They choose "insert column",
   pick a designation (for example the name of a wide-flange shape) from the profile library of
   the active **environment**, and the engine and kernel do the rest.
2. An environment is a per-country (or per-company) set of libraries, selected in the
   application's settings.
3. Users can add their own sections: parametric shapes with their own dimensions, cold-formed
   (light-gauge, LSTK) shapes, and fully custom outlines.

This is the MVP scope item 1 of `ROADMAP.md`. The UI picker belongs to V05; this note covers the
engine, data and CLI parts that the picker will call (`AGENTS.md` section 3: engine API and CLI
first).

## 1. What exists today

* A member pins exactly one immutable profile definition by the full SHA-256 of its USD layer
  (`bim:profile`, P02/P03). The definition is a content-addressed prim
  `/World/Definitions/D_<sha256>`; equal content is one definition.
* The only profile kind is `RectangleProfile { width_mm, height_mm, section }` with optional
  declared `SectionProperties` (designation, standard, area, mass per metre, surface per metre).
* `kernel::straight_member` builds an 8-vertex box. G05 (`kernel::profile`) already admits any
  planar outer contour with cavities, triangulates it and extrudes it into a sealed mesh, with a
  flagged polygon approximation for curves. A04 introduced `engine::solid::SolidMesh` for bodies
  that are not boxes.

## 2. The model in one picture

```text
Environment (USD layer)         Libraries (USD layers)            Model (USD revision)
  env-example.usda                example-sections.usda              /World/Definitions/D_<sha>
    subLayers = [user,              EX-I300 = i_shape{...} --resolve-->  profile definition:
                 example]         user-sections.usda                       shape + dims + labels
                                    MY-C150 = lipped_c{...}          member --bim:profile--> D_<sha>
```

* **Libraries are inputs, not model state.** Resolving a designation produces a complete profile
  definition (shape family, every dimension, declared section numbers, designation and library
  provenance labels) which is pinned by content like any other definition. A model therefore
  never depends on the library files: changing or deleting a library never changes an existing
  member, and opening a model on another machine needs no library.
* Resolving the same entry twice gives the same bytes and therefore the same digest, so placing
  a hundred `W310x52` columns creates one definition.
* An entry edited later (a corrected flange thickness) is a different definition; existing
  members keep the old one until an explicit `UpdateElement` repins them. Detecting "library
  entry changed since placement" is a later report, not an automatic edit.

## 3. Section families (A07.1)

A `SectionShape` is a closed enumeration. Every family generates its outline in the section
plane (millimetres, the F06 member frame: width along the rolled `X_m`, height along `Y_m`,
centred on the axis by the rule of section 4) and is admitted by its own rules before the kernel
sees a point.

| Family | Parameters | Notes |
|---|---|---|
| `rectangle` | `h`, `b` | Existing; solid bar. Stays byte-compatible. |
| `i_shape` | `h`, `b`, `tw`, `tf`, `r` | Doubly symmetric I/H (wide flange, IPE/HE style). |
| `channel` | `h`, `b`, `tw`, `tf`, `r` | Hot-rolled parallel-flange channel. |
| `angle` | `h`, `b`, `t`, `r` | Equal or unequal leg. |
| `rhs` | `h`, `b`, `t`, `r_out` | Rectangular/square hollow section (one cavity). |
| `chs` | `d`, `t` | Circular hollow section (one cavity). |
| `lipped_c` | `h`, `b`, `c`, `t`, `r_in` | Cold-formed C with lips (LSTK). |
| `plain_c` | `h`, `b`, `t`, `r_in` | Cold-formed U/C without lips. |
| `lipped_z` | `h`, `b`, `c`, `t`, `r_in` | Cold-formed Z with lips. |
| `omega` | `h`, `b`, `c`, `t`, `r_in` | Cold-formed hat section. |
| `custom` | outer contour, cavities | Any user outline, admitted by G05 as is. |

Rules:

* Admission per family: finite, inside the F04 dimension cap, above the geometric floor, and the
  family's own consistency (`2*tf < h`, `tw < b`, `r` fits between web and flange, `2*t < d`,
  lips shorter than the flange, ...). A violation is `INVALID_ARGUMENT` naming the parameter.
* **Two representations (owner decision D2).** Every member body is generated twice from the one
  definition: a **detailed** outline with every root and corner radius, and a **simplified**
  outline with sharp corners. This is the full/simplified pair of `docs/architecture.md` ("Part
  representations and view filters"): both are stored in the revision, identical content once,
  the detailed one is the default geometry an ordinary USD reader sees, and a view setting of the
  application chooses which one to load, so a large model can be viewed light. A section without
  radii has one outline referenced by both.
* Radii in the detailed outline are arcs approximated by a deterministic polygon with its vertices
  on the arc and a sagitta bound of `max(0.25 mm, 0.5 % of the radius)` (section 9), so that geometry is flagged `Approximated` (G05 rule; F06 0.1.0 has no exact
  curves). The simplified outline is flagged not exact. Volume, mass, BOM and snapping use the
  detailed one only, and the two are never summed (section 5).
* The point budget is the G05 one (1024 per contour, 2048 total); the generator chooses the arc
  segment count from the radius and the sagitta bound and refuses (`LIMIT_EXCEEDED`), never
  truncates.
* `custom` is stored as authored points; nothing is reordered, merged or repaired (F04 P-1).
* A new family is a new enumeration value and a new schema descriptor line, never a free-form
  string interpreted at runtime.

## 4. Kernel and engine (A07.1)

* `kernel::section_member(input)`: admits the generated outline with G05, extrudes it along
  `+Z` by the axis length and places it with the `geom::MemberFrame` basis (the same frame, roll
  and minima as `straight_member`). The result is a `SolidMesh`; the rectangle keeps
  `straight_member` so existing digests do not change.
* The derived layer stores the detailed mesh as the default geometry and the simplified mesh as
  a second representation (USD `proxy` purpose is the candidate encoding, as `docs/architecture.md`
  proposes); identical blobs are stored once. Each representation has its own mesh digest, and
  the publish-time verifier rebuilds both. The view snapshot (V02) gains the representation
  choice; the UI setting itself is V05/V06.
* Insertion point: the section's reference point is the centre of its bounding box, as for the
  rectangle today. Offsets (top-of-steel, flange edge) are a later placement option.
* `engine::member` resolves `ProfileInfo` as the full `SectionShape` instead of
  `width_mm`/`height_mm`, and the publish-time verifier re-generates and re-extrudes every member.
* Layer engine: `Definition::Profile { shape, section, source }` beside the existing
  `RectangleProfile`, written as `steel:profile:family`, one `double` attribute per parameter
  (`steel:profile:hMm`, ...) and, for `custom`, the same flat point and count arrays as A04
  holes. The schema descriptor gains these lines; the previous descriptors stay accepted.
* CLI: `define-profile --family i_shape --h 310 --b 165 --tw 5.8 --tf 9.7 --r 8.9 ...` for
  explicit definitions (tests, scripting, user-defined sections).

## 5. Section numbers and mass

* The library entry may declare area, mass per metre and surface per metre (P02.2 fields). They
  are what a BOM reads, and they are part of the definition's identity.
* The kernel computes its own area from the generated polygon. A declared area that differs from
  the polygon area by more than the family tolerance is refused, as P02.2 does for the rectangle.
  The tolerance must cover the radius approximation; the value is decided with the first real
  data (section 7, D3).
* No density, grade or mass is ever guessed from a designation.

## 6. Environments and libraries (A07.2, A07.3)

**File format (owner decision D1): USD.** A library is a separate USD layer, written and read by
the same layer engine and with the same `steel:profile:*` attributes as a model definition, so
there is one mapping, one admission and one validator. Each entry is a prim under
`/Library/Profiles/` whose name is derived from the designation and which carries the
designation itself as a label:

```usda
#usda 1.0
(
    customLayerData = {
        string "bim:library:id" = "example"
        string "bim:library:title" = "Synthetic example sections"
        string "bim:library:version" = "1"
        string "bim:library:source" = "Contributor-original synthetic values for tests"
        string "bim:library:licence" = "MPL-2.0"
    }
)
def Scope "Library" {
    def Scope "Profiles" {
        def Scope "EX_I300" {
            string steel:profile:designation = "EX-I300"
            token  steel:profile:family = "i_shape"
            double steel:profile:hMm = 300
            double steel:profile:bMm = 150
            double steel:profile:twMm = 7
            double steel:profile:tfMm = 10
            double steel:profile:rMm = 15
            double steel:profile:massPerMeterKg = 36   # optional declared number
        }
    }
}
```

National libraries (AISC, ICHA, EN, ...) are produced separately as their own `.usd` files with
their own provenance; the repository ships only the mechanism and one synthetic test library
that follows no country's standard.

**Environment.** An environment is also a small USD layer: its `subLayers` list the libraries in
order of strength, and its `customLayerData` carries its id and title. USD sublayer strength is
exactly the lookup order (stronger first), so no second precedence rule is invented:

```usda
#usda 1.0
(
    customLayerData = { string "bim:environment:id" = "example" }
    subLayers = [@./user-sections.usda@, @./example-sections.usda@]
)
```

**Lookup order.** Project libraries, then the user's own library, then the environment's
libraries in order. The first library that has the designation wins; a duplicate inside one
library is refused when the library is loaded, and a shadowed designation is reported. Sublayer
paths are local and contained (the U01 resolution rules); a network path is refused before any
I/O, as for R01 references.

**Where files live.** The application data directory holds `environments/` and `libraries/`;
the active environment is a user setting. The CLI takes `--environment FILE` explicitly, never a
hidden global, so tests and scripts are reproducible.

**Untrusted input.** A library file is untrusted (`AGENTS.md` section 4): it is opened with the
U01 bounded read (size, prim and stage budgets), every entry is admitted by its family rules, a
bad entry is reported with its file, prim and designation and never stops the other entries from
loading, and nothing in a library entry can name a path, URL or executable.

**Commands (headless first).**

```text
beamcraft library-list   --environment FILE [--family F] [--filter TEXT]
beamcraft library-show   --environment FILE --designation NAME
beamcraft create-member  --store ROOT --environment FILE --profile NAME --start .. --end ..
```

`create-member --profile NAME` resolves the entry, defines the profile if it is not in the model
yet (content-addressed, so it is idempotent) and pins it, in one revision.

**User sections (A07.3).** A user library is the same format in the user's data directory.
`library-add` writes an admitted entry to it (parametric, cold-formed or `custom` outline), so
the UI "new profile" dialog of V05 calls the same API.

## 7. Owner decisions needed

* **D1. Library data source and format — decided 2026-10-18.** Libraries are separate `.usd`
  files made outside this change. For tests the repository ships one synthetic library that
  follows no country's standard. Any real table later records its source and licence
  (`AGENTS.md` section 1.2) and may live outside the open repository.
* **D2. Radii — decided 2026-10-18.** Store both: a detailed representation with every radius
  and a simplified one without, chosen by an application view setting (section 3).
* **D3. Declared-area tolerance** for families with radii (section 5).
* **D4. Contract amendment.** F04 and F06 0.1.0 name the rectangle as the only initial profile.
  A07.1 needs an amendment (F04 section 4 profile kinds, F06 section 2 member input) recorded
  before or with the implementation, as A04 did for holes.

## 8. Work split

| Sub-task | Result | Depends on |
|---|---|---|
| A07.1 | Families of section 3 (hot-rolled and cold-formed) and `custom`, detailed and simplified outlines, kernel `section_member`, layer-engine definition, engine, `define-profile --family`, end-to-end test | D4 |
| A07.2 | USD library and environment layers, loader, `library-list`, `library-show`, `create-member --profile NAME`, synthetic test library, end-to-end test | A07.1 |
| A07.3 | User library and `library-add` for parametric, cold-formed and custom sections | A07.2 |
| V05 | UI: insert member, pick from the library; new-profile dialog | A07.2, A07.3 |

Each sub-task is its own PR. Completing a sub-task does not complete A07.

## 9. A07.1 as implemented

```text
beamcraft define-profile --store ROOT --family i_shape --h 300 --b 150 --tw 7 --tf 10 --r 15
                         [--designation TEXT] [--standard TEXT] [--area-mm2 F] [--mass-per-meter-kg F]
beamcraft define-profile --store ROOT --family custom --outer "X,Y;..." [--cavity "X,Y;...|..."]
beamcraft create-member  --store ROOT --start X,Y,Z --end X,Y,Z --profile-digest SHA256 ...
```

| Area | What exists |
|---|---|
| `beamcraft_model::section` | `SectionFamily` (9 families, tokens, parameter order), `ParametricSection` (admission: finite, caps, floor, family consistency, all `INVALID_ARGUMENT` naming the parameter; `without_radius`), `CustomSection` (counts and coordinate caps only), `SectionShape` (`token`, `validate`, `extent_mm`); `definition::SectionProfileSpec`. |
| `beamcraft_kernel::section` | `outline(shape, Detailed or Simplified)`: corner lists per family, quarter arcs centred at `P - r*d1 + r*d2` on the G05 deterministic angular grid, CHS as two circles; `section_member`: G05 extrusion along the axis length, `RigidTransform` of the F06 member frame, G06 certification of one shell; `SectionFailure::f04_code`. |
| `beamcraft_layer_engine::authored` | `Definition::SectionProfile`: `bim:definitionKind` is the family token or `custom`, one `steel:profile:<parameter>Mm` per parameter, flat `outerPointsMm`/`cavityPointsMm` + `cavityVertexCounts` for custom outlines; a member's `bim:profile` may target any profile kind; descriptor amended, the A06 descriptor stays accepted. |
| `beamcraft_engine::member` | `ProfileInfo::{Rectangle, Section}`; a rectangle keeps the P03 box and its bytes, a section is the kernel's detailed outline (`SECTION_VOLUME_ALGORITHM`); `MemberBody` keeps the resolved profile and its volume; the member shape signature of a rectangle is unchanged. |
| `beamcraft_cli` | `define-profile --family`, section definitions decoded, listed and pinned; `definitions` and member JSON show `parameters_mm` or the outline. |

Deviations and limits, recorded rather than hidden:

* **Contract (D4).** F04/F06 0.1.0 name the rectangle as the only initial profile. A07.1 adds kinds
  under a recorded deviation, as A04 did for holes; the formal amendment is the owner's decision.
* **Simplified body not stored yet.** The kernel builds both outlines, but the derived layer still
  stores one mesh per element. Storing the simplified mesh as the second representation and the
  view setting that chooses it follow with V02/V05 work.
* **Declared area (D3).** For section families, a declared area is admitted as a label-like value
  and not yet compared with the outline area; the rectangle keeps its P02.2 check.
* **Volume status.** A section member's volume is `current` for the polygon it stores; the arcs are
  approximated (flagged in the kernel result), as the A04 round holes are.
* **Layer budgets.** The U01 first-slice ceilings (64 KiB per layer, 1024 items per array) bound
  model size: a probe stored 7 detailed I-sections 300x150 (or 36 rectangle members) in one
  revision before `LIMIT_EXCEEDED`. This is not caused by A07.1 but makes it visible; raising the
  ceilings or storing meshes outside the derived layer text is a separate task.
* No placement offsets, no rotated placement (P03 rule), no member holes or cuts.

Evidence: `cargo test -p beamcraft-model section`, `cargo test -p beamcraft-kernel --test section`
(closed-form areas of every family in both representations, bounding-box centring, member bounds
in the F06 frame, divergence-theorem volume, determinism, custom and refusal codes), the
layer-engine round trip and tamper test, the engine unit test, and `cargo test -p beamcraft-cli
--test sections` (real binary: define, idempotent redefine, build, reopen in a fresh process,
world bounds, `definitions`, saved text, cold-formed, CHS and custom members in one revision,
refusal codes).

## Provenance

Section families and their parameters are generic engineering geometry described in public
standards (EN 10365, ASTM A6, EN 10162 for cold-formed sections) and textbooks; no dimension
table, catalogue file or software material is used. The environment and library concept is the
owner's requirement of 2026-10-18; the USD layout of libraries and environments is original to
this note and uses only standard USD layer metadata, sublayers and prims.
