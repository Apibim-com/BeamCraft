# E01: own IFC export contract, version 0.1.0

Status: **draft 0.1.0, not frozen; K6 not started**. Date: 2026-10-17.
Base: `43026e26e631c0999dec55d56917981adb8c30a6` (`origin/master`).

This is an original, implementation-independent contract. It fixes **what BeamCraft writes to an IFC
file, how an element becomes an IFC entity, how its 22-character GlobalId is derived, what is lost,
and how the writer is built and driven.** It is written before the exporter (E02) so that E02 has a
public mapping to implement and a loss list to report. **Nothing here is implemented.** No writer,
GlobalId function, export command or derived attribute exists in code. The examples are acceptance
descriptions, not APIs or saved files. Decisions the owner has not made are collected as questions
in section 17; every default stated before that point is a draft proposal and is marked as such.

## 1. Status, scope and provenance

### 1.1 Relation to other contracts

- [F04](model-and-commands-v0.1.md) (frozen 0.1.0): identities (section 2), units and caps
  (section 3), quantity statuses (section 5) and the 23 error codes (section 8). E01 changes
  nothing there and adds no error code.
- [F06](geometry-contract-v0.1.md) (frozen 0.1.0): the member and plate frames (P-1 to P-3), the
  quantity definitions (section 8), the key and signature rules (section 7) and the `GEO_` code
  table, which is the model for the `IFC_` table of section 13.
- [F10](domain-vocabulary-v0.1.md) (draft): the closed element registry (`plate`, `beam`, `column`
  implemented, ten kinds reserved), unknown-content preservation, the validation envelope and
  its ordering (sections 6.3 and 6.4). Its section 8 already names E01: the registry is the set of
  exportable kinds, and reserved and unknown content is not exported.
- [Module boundaries (F08)](module-boundaries-v0.1.md): the layering that section 15.1 extends with
  proposed rows (not applied).
- P03 ([design](../design/p03-straight-member.md)), P04 ([design](../design/p04-quantities-update.md))
  and P02.2 ([design](../design/p02-2-section-properties.md)): the committed members, the stored
  quantity records and their freshness, and the declared section values. Only what those slices
  publish today is exported (section 8). The A04 plate holes ([design](../design/a04-plate-holes.md))
  are an experimental pilot that this contract covers because they exist in stores today.

### 1.2 What the document decides

1. The IFC schema and file format, and the supported entity subset (section 2).
2. The deterministic GlobalId, its exact encoding, its test vectors, its failure behaviour and
   where it is published (section 3).
3. Which committed data is read, which elements are eligible and in what order (section 4).
4. Placement, shape representation, material, quantities, property sets and units (sections 5 to
   10), each row pointing to a public buildingSMART specification section (section 18).
5. Determinism rules (section 11), the explicit list of losses (section 12), the diagnostic codes
   (section 13), size bounds (section 14) and the headless surface and writer design (section 15).

### 1.3 Excluded

- **IFC import and conversion of foreign models.** Reading IFC, and turning IFC into BeamCraft
  elements or read-only references, needs a separate scope decision (plan task E04). This contract
  defines no reading API (`IFCX-SCOPE-003`). A test harness may parse a written file to check it;
  that is not a product feature.
- The exporter itself (E02), BOM and reports (E03) and drawings (D tasks).
- Any IFC feature outside the subset of section 2, including model view definition conformance,
  classification, structural analysis, connections, assemblies, bolts, welds and reinforcement.
- Remote, collaborative or networked export. Export is a local, read-only operation.

### 1.4 Provenance

Contributor-original BeamCraft text, MPL-2.0, derived from the owner's architecture direction (IFC
is an isolated export adapter of an explicitly supported subset, built on the engine API), from
F04, F06, F10 and F08 and from the repository documents above. The plan card (E01) and its 2026-10-17
amendment (a deterministic 22-character GlobalId derived from the UUIDv7 and published in
derived/export data) are the task statement.

**Behaviour comes only from the public buildingSMART IFC specification** (entity definitions,
property and quantity set definitions, the GlobalId type and the STEP physical file format as
described there). Section 18 lists every section used. **No code, schema listing, example file,
naming list or algorithm is taken from ifc-lite, IfcOpenShell, any other IFC library or any
commercial product** (`AGENTS.md` sections 1 and 2). The ifc-lite crates were evaluated in
[N02](../design/n02-ifc-lite-evaluation-v0.1.md) as a possible geometry dependency; E01 neither
depends on them nor reuses anything from that evaluation. The GlobalId encoding of section 3 is
derived from the public type definition and checked by hand (section 3.2); the test vectors are
BeamCraft's own. All examples in this document and the case file were written for it.

## 2. IFC version and supported subset

### 2.1 Schema and file format

| Item | Draft 0.1.0 value | Source |
|------|-------------------|--------|
| Schema identifier in `FILE_SCHEMA` | `IFC4X3_ADD2` (IFC 4.3.2.0, the edition published as ISO 16739-1:2024) | S01. Owner question Q-1: this or `IFC4` (IFC4 ADD2 TC1) |
| File format | Clear-text STEP physical file (`.ifc`), 7-bit ASCII, LF line ends | S02. `ifcXML`, `ifcJSON` and `ifcZIP` are not produced (`IFCX-VER-002`) |
| Model view definition | None claimed (`IFCX-VER-003`) | Q-2 |
| Contract version in requests | Exactly `0.1.0` (`IFCX-VER-001`) | F04 section 6 pattern |

Reason for the draft choice: IFC 4.3 is the current official edition, its entity names for the
subset are the same as in IFC 4, and its plate base quantity set carries weight quantities that
the IFC 4 edition lacks (section 8.2). The cost is narrower reader support than IFC 4 today. The
two editions differ only in a few rows of this contract (the schema identifier, the plate weights
and the enumeration values); an IFC 4 target is an alternative the owner may choose, not a second
supported schema in 0.1.0.

### 2.2 Entity subset

Every entity below is written by a fixed template (section 15.2); the writer has no generic IFC
object model. The spec column cites section 18.

| ID | Entity | Used for | Spec |
|----|--------|----------|------|
| E-01 | `IfcProject` | One per file; units and representation context | S10 |
| E-02 | `IfcSite`, `IfcBuilding` | One synthetic site and building (Q-7) | S11, S12 |
| E-03 | `IfcRelAggregates` | Project to site, site to building | S13 |
| E-04 | `IfcRelContainedInSpatialStructure` | All exported elements in the building | S14 |
| E-05 | `IfcBeam`, `IfcColumn`, `IfcPlate` | One per exported element of kind `beam`, `column`, `plate` | S15, S16, S17 |
| E-06 | `IfcLocalPlacement`, `IfcAxis2Placement3D`, `IfcAxis2Placement2D`, `IfcCartesianPoint`, `IfcDirection` | Placement and frames | S18 to S22 |
| E-07 | `IfcProductDefinitionShape`, `IfcShapeRepresentation` | The body representation | S23, S24 |
| E-08 | `IfcGeometricRepresentationContext`, `IfcGeometricRepresentationSubContext` | One model context and one Body sub-context | S25, S26 |
| E-09 | `IfcExtrudedAreaSolid` | The only solid item | S27 |
| E-10 | `IfcRectangleProfileDef` | Member section | S28 |
| E-11 | `IfcArbitraryClosedProfileDef`, `IfcArbitraryProfileDefWithVoids`, `IfcPolyline` | Plate outline, plate outline with through holes | S29, S30, S31 |
| E-12 | `IfcMaterial`, `IfcRelAssociatesMaterial`, `IfcMaterialProperties` | Pinned material, its density | S32, S33, S34 |
| E-13 | `IfcPropertySet`, `IfcPropertySingleValue`, `IfcRelDefinesByProperties` | `BeamCraft_Element` | S35, S36, S37 |
| E-14 | `IfcElementQuantity`, `IfcQuantityVolume`, `IfcQuantityWeight` | The base quantity sets | S38 to S40, S44 to S46 |
| E-15 | `IfcUnitAssignment`, `IfcSIUnit`, `IfcDerivedUnit`, `IfcDerivedUnitElement` | Units | S41, S42, S43 |
| E-16 | Types `IfcGloballyUniqueId`, `IfcLabel`, `IfcIdentifier`, `IfcText`, `IfcInteger` | Attribute types | S03 to S06 |

Predefined types: `IfcBeam.PredefinedType = .BEAM.`, `IfcColumn.PredefinedType = .COLUMN.`,
`IfcPlate.PredefinedType = .NOTDEFINED.` (S15 to S17 and the enumerations S48 to S50; the plate enumeration has values such as
base, cover and gusset plate that BeamCraft does not know, so it claims none). `IfcRoot.OwnerHistory`
is written as unset (`$`); whether the chosen validator requires it is an E02 finding.

### 2.3 Not in the subset

`IfcOpeningElement` and `IfcRelVoidsElement` (holes are profile voids, section 6), boolean results,
tessellated and brep items, `IfcBeamType`, `IfcColumnType`, `IfcPlateType`, `IfcMaterialProfileSet`
and usage entities, styles and colours, `IfcBuildingStorey`, grids, classification references,
`IfcOwnerHistory`, `IfcRelAssignsToGroup`, assemblies and connections, annotations and any
analysis entity. Elements that need them are not first-scope kinds.

## 3. Identity and the GlobalId

### 3.1 The encoding E

`IfcGloballyUniqueId` is a 22-character string (S03). E(u) encodes the 128 bits of a UUID as follows.

1. **Input.** The 16 bytes of the UUID in the order of the canonical text, byte 0 first (RFC 9562
   section 4 network order, which is `Uuid7::as_bytes`). The bytes are read from the hex digits of
   the `8-4-4-4-12` text from left to right; no field is swapped or reversed.
2. **Number.** Read the 16 bytes as one unsigned 128-bit big-endian integer `N`.
3. **Digits.** Write `N` in base 64 as 22 digits, most significant first: the first digit is
   `N >> 126`, which is 0 to 3 (2 bits); each of the other 21 digits is the next 6 bits. In byte
   terms, byte 0 supplies the first two digits (its top 2 bits, then its low 6 bits) and every
   following group of 3 bytes supplies 4 digits. This is the layout the public type definition
   describes, and the first digit is always 0, 1, 2 or 3.
4. **Alphabet.** Digit value 0 to 63 maps to `0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$`
   (S03): digits `0`-`9` are values 0 to 9, `A`-`Z` 10 to 35, `a`-`z` 36 to 61, `_` is 62 and `$` is 63.

E is a bijection between 128-bit values and the 22-character strings whose first character is in
`0123`. It is not RFC 4648 base64: it has no padding, uses `_$` instead of `+/` and starts with a
2-bit digit.

The ASCII order of GlobalIds is not the numeric order of the UUIDs, because `_` (0x5F) sorts before
`a` (0x61) but has the larger digit value. **Every list in the file and the report is therefore
ordered by ElementId, never by GlobalId** (`IFCX-DET-001`, case `E01-POS-11`).

### 3.2 Test vectors (hand-derived)

The vectors were derived by hand from the bit groups and then confirmed with a throwaway script
that also checked decode(encode(x)) = x. They are the contract; an implementation that disagrees is
wrong. Vectors 1 to 5 are encoder-only (their version bits are not 7, so they are not valid
ElementIds); 6 to 11 are valid UUIDv7 text.

| # | Vector | UUID text | GlobalId | Note |
|---|--------|-----------|----------|------|
| 1 | V-ENC-01 | `00000000-0000-0000-0000-000000000000` | `0000000000000000000000` | encoder-only (version nibble 0) |
| 2 | V-ENC-02 | `ffffffff-ffff-ffff-ffff-ffffffffffff` | `3$$$$$$$$$$$$$$$$$$$$$` | encoder-only (version nibble f) |
| 3 | V-ENC-03 | `80000000-0000-0000-0000-000000000000` | `2000000000000000000000` | encoder-only |
| 4 | V-ENC-04 | `00000000-0000-0000-0000-00000000003f` | `000000000000000000000$` | encoder-only |
| 5 | V-ENC-05 | `00000000-0000-0000-0000-000000000040` | `0000000000000000000010` | encoder-only |
| 6 | V-ENC-06 | `019a7bf0-0000-7000-8000-000000200001` | `01cdlm001m080000008001` | valid ElementId |
| 7 | V-ENC-07 | `019a7bf0-0000-7000-8000-000000000005` | `01cdlm001m080000000005` | valid ElementId |
| 8 | V-ENC-08 | `01234567-89ab-7def-bfed-cba987654321` | `018qLdYQjzxx$jowc7PKCX` | valid ElementId |
| 9 | V-ENC-09 | `0197c3a8-5e21-7b4f-9a06-3d8c1e72f4b9` | `01byEeNY5xJve6FOmUSlIv` | valid ElementId |
| 10 | V-ENC-10 | `019a7bf0-0000-7000-8000-000000000024` | `01cdlm001m08000000000a` | valid ElementId, last byte 36 |
| 11 | V-ENC-11 | `019a7bf0-0000-7000-8000-00000000003e` | `01cdlm001m08000000000_` | valid ElementId, last byte 62 |

Worked example, vector 6. The bytes are `01 9a 7b f0 00 00 70 00 80 00 00 00 00 20 00 01`. The first
digit is the top two bits of `01`, which is 0. The other 126 bits in groups of six have the values
1, 38, 39, 47, 48, 0, 0, 1, 48, 0, 8, 0, 0, 0, 0, 0, 0, 8, 0, 0, 1, which are the characters
`1 c d l m 0 0 1 m 0 8 0 0 0 0 0 0 8 0 0 1`, giving `01cdlm001m080000008001`.

The first two characters of the GlobalId of an ElementId whose millisecond timestamp falls between 2004 and 2039 are `01`, because they
come from the top byte of the 48-bit millisecond timestamp. GlobalIds therefore share a prefix.
This is an observation, not a rule; the timestamp is a creation hint and never an authority (F04
section 2).

### 3.3 Input that is not an ElementId

`IFCX-GUID-002`. The GlobalId function takes the text of `bim:id` and succeeds only when
`beamcraft_model::id::Uuid7::parse` succeeds: 36 characters, hyphens at positions 8, 13, 18 and 23,
**lowercase** hexadecimal, version nibble 7 and RFC variant. Anything else gives `INVALID_ID`, no
GlobalId, and no repair. In particular the function does not lower-case upper-case text, remove
braces or hyphens, accept a version 4 or other-version UUID, pad or truncate, or fall back to a
hash of the text. A fallback would mint an identity that BeamCraft's own identity rules (F04 section
2, F10 `DV-BASE-001`) do not recognise, and two different malformed texts could silently collide.

At export time a stored element whose `bim:id` fails this check is skipped with `INVALID_ID` and
listed (section 4.2). The test vectors 1 to 5, which are not valid ElementIds, exercise the 128-bit
function E of 3.1 only; the ElementId check of this section is a separate step in front of it, so
the vectors can be checked without weakening the rule.

### 3.4 Decoding

`IFCX-GUID-003`. D(g) accepts exactly 22 characters, each in the alphabet of 3.1, the first being
`0`, `1`, `2` or `3`. Any other text (length 21 or 23, a character such as `+`, `/`, `=` or a
non-ASCII character, a first character `4` or higher) is `INVALID_ARGUMENT`, because the value
would not fit in 128 bits or is not an IfcGloballyUniqueId. D(E(u)) = u for every 128-bit value.
Decoding is a helper for external matching (section 15.3); it does not turn the result into an
ElementId unless it also passes `Uuid7::from_bytes`.

### 3.5 Synthetic GlobalIds

Entities that are not elements but are `IfcRoot` subtypes (project, site, building, relationships,
property set, quantity set) need a GlobalId too (`IFCX-GUID-004`):

`G(role, keys) = E( first 16 bytes of SHA-256( "BCIFC-GUID" 0x00 role 0x00 key_1 ... key_k ) )`

with SHA-256 as in `beamcraft_model::digest` (FIPS 180-4), `role` the ASCII token below and each
key a fixed-width byte string, so the preimage is unambiguous.

| Entity | Role token | Keys |
|--------|-----------|------|
| `IfcProject` | `project` | ProjectId (16 bytes) |
| `IfcSite` | `site` | ProjectId |
| `IfcBuilding` | `building` | ProjectId |
| `IfcRelAggregates` project to site | `rel-project-site` | ProjectId |
| `IfcRelAggregates` site to building | `rel-site-building` | ProjectId |
| `IfcRelContainedInSpatialStructure` | `rel-contained` | ProjectId |
| `IfcRelAssociatesMaterial` | `rel-material` | ProjectId, material ContentDigest (32 bytes) |
| `IfcPropertySet` | `pset-element` | ElementId (16 bytes) |
| `IfcElementQuantity` | `qto-element` | ElementId |
| `IfcRelDefinesByProperties` for the property set | `rel-pset-element` | ElementId |
| `IfcRelDefinesByProperties` for the quantity set | `rel-qto-element` | ElementId |

These values are not UUIDs and carry no version bits. Vector `V-SYN-01` (role `site`, ProjectId
`019a7bf0-0000-7000-8000-000000000001`) is `2ghTuGpW1EXNtW6xLArrhQ`; it was derived with a throwaway
script and must be reproduced with the in-repo SHA-256 before it is trusted. `IfcMaterial`,
`IfcMaterialProperties` and the geometry entities are not `IfcRoot` and carry no GlobalId.
A repeated GlobalId anywhere in one file refuses the export (`IFCX-GUID-006`); this cannot
happen for distinct valid ElementIds and distinct roles, and the check exists so that it never goes
unnoticed.

The ProjectId is an explicit export argument in 0.1.0, because the experimental stores of today
record none (Q-3).

### 3.6 Stability

`IFCX-GUID-007`. The element GlobalId is a pure function of the ElementId. It is the same after a
rename, an edit of any property, a move, a new revision, another export run, another platform and
another day. It ignores the element name, revision and every time. A copy has a new ElementId
(F04 `CopyElement`) and so a new GlobalId; the source's GlobalId never changes. The export report
lists both identifiers for every element, so a downstream program can map either way.

### 3.7 Publication in derived and export data

The amendment asks that external programs match BeamCraft elements by GlobalId. Two publications:

1. **Export data (decided here).** The IFC file carries the GlobalId as `IfcRoot.GlobalId`, and the
   export report lists `element_id` and `ifc_global_id` for every element (section 15.4).
2. **Derived data (proposal, Q-6).** The derived layer carries `bim:ifcGlobalId` (USD `string`, 22
   characters) on the element's `Geometry` prim next to the stored quantities. It is derived, not
   authored: it is written only by the derived-layer builder, never by a command, and the
   publish-time verifier recomputes E(`bim:id`) and treats any other value as `INTEGRITY_ERROR`
   (`IFCX-GUID-008`). A plain USD reader such as pxr then shows the same identifier an IFC reader
   shows. Adding the attribute changes the U02 schema descriptor and its digest, so it needs its own
   small task or goes into E02; this contract does not make that choice.

## 4. Source data

### 4.1 What is read

`IFCX-SCOPE-002`. The export reads one verified committed revision through the host's verified reader
(U06): by default the published head, optionally an explicit RevisionId. Layers and blobs are
verified; a revision that cannot be read completely is refused with `DEPENDENCY_UNAVAILABLE`. A
pinned snapshot of the revision is the only input (the same snapshot the F06 queries use), so all
data of one export comes from one revision. A personal workspace draft revision is exportable, if the host exposes it, only through its own
RevisionId. The export never reads a mutable working state.

`IFCX-SCOPE-004`, `IFCX-SURF-001`. The export changes nothing: no authored layer, derived layer,
revision, generation or journal is touched. Recording an export is not part of 0.1.0.

### 4.2 Eligibility of an element

An element is exported if and only if all of these hold, checked in this order; the first failure
skips the element and lists it (the request fails instead where the table says so):

| Order | Check | Failure |
|-------|-------|---------|
| 1 | `bim:elementKind` is `plate`, `beam` or `column` | skip, `IFCX-SCOPE-001` (no F04 code; warning) |
| 2 | `bim:lifecycle` is `active` | skip, `IFCX-SCOPE-005` (info) |
| 3 | `bim:id` is a valid ElementId | skip, `INVALID_ID` |
| 4 | Placement is rigid and its quaternion is within the F04 caps | skip, `UNSUPPORTED_GEOMETRY` or `INVALID_ARGUMENT` |
| 5 | Shape inputs are finite | skip, `NON_FINITE_NUMBER` |
| 6 | Pinned profile and material digests resolve | **request fails**, `NOT_FOUND` |
| 7 | Geometry is a supported sealed result: rectangular plate (with or without holes), or a member with a rectangle profile | skip, `UNSUPPORTED_GEOMETRY` |
| 8 | The stored derived mesh equals the kernel's re-evaluation | **request fails**, `INTEGRITY_ERROR` |

Check 7 is deliberately strict: a valid non-rectangular plate is an accepted authored input whose
evaluation is `unsupported` until A02 (F04 section 3), and BeamCraft must not hand a consumer
geometry that it does not certify itself. Elements that are skipped stay in the store untouched.

### 4.3 Order

All elements are processed in ascending ElementId order (byte comparison of the lowercase text,
the order of F06 `ListElements`), materials in ascending ContentDigest order (`IFCX-DET-001`).

## 5. Spatial structure and placement

The model has no site, building or level concept. The file therefore holds one synthetic
`IfcProject` (Name from the optional `project_name` argument, otherwise unset), one `IfcSite` and
one `IfcBuilding`, aggregated in that order, with both placements at the world origin with identity
axes. Names are the fixed texts `Site` and `Building`. All exported elements are contained in the
building. There is no storey (Q-7).

**Element placement** (`IFCX-PLC-001`). The F04 pose maps element-local coordinates to world by
`world = R(q) * local + translation_mm`. The element's `ObjectPlacement` is an `IfcLocalPlacement`
with `PlacementRelTo` the building placement and `RelativePlacement` an `IfcAxis2Placement3D`:

| Attribute | Value |
|-----------|-------|
| `Location` | `translation_mm` as an `IfcCartesianPoint` (millimetres) |
| `Axis` | `R(q)` applied to `(0, 0, 1)`, the third column of `R(q)` |
| `RefDirection` | `R(q)` applied to `(1, 0, 0)`, the first column of `R(q)` |

The second axis is implied (`Axis` cross `RefDirection`), so the element-local frame becomes the IFC
local frame exactly. `q` is normalized first (`IFCX-PLC-004`, F04 section 3); `q` and `-q` give
identical numbers because `R` is built from products of components, and negative zero is written
as zero. The direction values are written as computed in binary64; F04's dimensionless comparison
(absolute 1e-6 at zero, relative 1e-6 otherwise) applies to checks. A placement with scale, shear,
reflection or a matrix form is `UNSUPPORTED_GEOMETRY` (`IFCX-PLC-002`, F06 `GEO_UNSUPPORTED_TRANSFORM`);
a stored quaternion outside the F04 caps is `INVALID_ARGUMENT` (`IFCX-PLC-003`, `DV-LIM-003`).
Nothing is baked into the geometry: the solid is written in element-local coordinates and the
placement moves it.

Note: the P01/P03/A04 slices refuse a placement rotation other than the identity today
(`UNSUPPORTED_GEOMETRY`), so exported files of today have identity axes. The rule is specified now
so that it does not change when the limit is lifted.

## 6. Shape representation

### 6.1 Choice: a swept solid, not a tessellation

| Criterion | Swept solid (`IfcExtrudedAreaSolid`) | Tessellated item |
|-----------|--------------------------------------|------------------|
| Fidelity to the authored shape | Exact: the same numbers as the authored inputs and the kernel frame | The derived mesh stores `float32` points in USD, so a tessellation from the stored layer would carry a rounding that the kernel's `float64` result does not have |
| Meaning for a consumer | A rectangle section with a length, a plate with an outline and a thickness: editable, measurable, classifiable | Anonymous triangles |
| Size | A few entities per element | 8 vertices and 12 triangles per element at least, more with holes |
| Fit with the first scope | Both bodies are boxes or extruded outlines (F06 section 6) | Needs a second representation to carry holes without voids |
| Reader support | Universal for the swept-solid case | Required by some model view definitions (Q-2) |

**Decision (draft):** one `Body` representation of type `SweptSolid` with one `IfcExtrudedAreaSolid`
per element (`IFCX-GEO-001`), and no tessellated companion in 0.1.0 (`IFCX-GEO-009`). Q-2 asks
whether a tessellated companion or a tessellated-only profile is wanted for target viewers.

**Loss stated:** the exact derived mesh (vertex order, face table, FeatureIds, mesh digest, shape
key) is not exported; a reader gets the solid, not BeamCraft's triangle mesh. A round hole is a
polygon, as it is in the store. See L-03 to L-05.

### 6.2 Beam and column (`IFCX-GEO-002`)

In the kernel frame of F04 P-2 and P-3, with `Z_m = unit(p2 - p1)`, rolled `X_m`, rolled `Y_m` and
axis length `ell = |p2 - p1|`:

| IFC attribute | Value |
|---------------|-------|
| `IfcExtrudedAreaSolid.Position` | `IfcAxis2Placement3D(Location = p1, Axis = Z_m, RefDirection = X_m)` in element-local millimetres |
| `SweptArea` | `IfcRectangleProfileDef(ProfileType = .AREA., ProfileName = declared designation or unset, Position = IfcAxis2Placement2D at the origin, XDim = width_mm, YDim = height_mm)` (S28) |
| `ExtrudedDirection` | `IfcDirection(0, 0, 1)` in the solid's own frame |
| `Depth` | `ell` |

The rectangle profile is centred on its position, so the section is centred on the axis (P-2); its
x extent is the width along `X_m` and its y extent the height along `Y_m`; the end faces are
perpendicular to the axis through `p1` and `p2`, with no extension. The solid's second axis is
`Z_m x X_m`, which equals `Y_m` because `X_m x Y_m = Z_m` (F04 P-3). The beam example of F04 gives
`Axis = (1,0,0)`, `RefDirection = (0,1,0)`, hence the 300 mm height vertical (case `E01-POS-18`).
There is no loss in the geometry of a rectangle member.

### 6.3 Plate (`IFCX-GEO-003`)

In the kernel frame of F04 P-1 (`p0` the first contour point, `X_p = unit(p1 - p0)`, `Z_p` the
contour normal, `Y_p = Z_p x X_p`):

| IFC attribute | Value |
|---------------|-------|
| `IfcExtrudedAreaSolid.Position` | `IfcAxis2Placement3D(Location = p0, Axis = Z_p, RefDirection = X_p)` |
| `SweptArea` without holes | `IfcArbitraryClosedProfileDef(.AREA., unset, OuterCurve = IfcPolyline)` through the contour points as 2D coordinates `((p_i - p0) . X_p, (p_i - p0) . Y_p)`, closed by repeating the first point |
| `SweptArea` with through holes | `IfcArbitraryProfileDefWithVoids` with the same outer curve and one inner `IfcPolyline` per hole, each closed by repeating its first point, in the stored vertex order (S30) |
| `ExtrudedDirection`, `Depth` | `(0, 0, 1)` and the thickness |

The contour is the bottom face (body z = 0) and the top face is at the thickness (F04 P-1). Hole
polygons are written as stored; no boolean and no opening element is created. Whether a validator
requires a particular winding of inner curves is an E02 finding; the exporter does not reorder
points.

### 6.4 Checks

`IFCX-GEO-008`. The exporter takes the frames from the kernel's derivation (the same function that
builds the mesh) and never re-implements them. E02's acceptance includes an independent check that
the volume of the written solid, recomputed from the file by a validation tool, equals the published
volume within the CR-TOL-001 quantity bound (relative 1e-6), and that the solid's bounds equal the
published local bounds (case `E01-POS-40`). `IFCX-GEO-007`: the stored derived mesh of every exported
element is compared with the kernel's re-evaluation, as the publish-time verifier does, so a layer
edited behind BeamCraft's back cannot reach the file.

## 7. Material and profile

**Material** (`IFCX-MAT-001` to `IFCX-MAT-003`). A pinned material definition (`bim:material`) used
by at least one exported element becomes one `IfcMaterial` (S32). `Name` is the declared grade
(P02.2) or, when none is declared, the text `BC-` followed by the full 64 lowercase hexadecimal
digits of its ContentDigest (a name that never collides and never invents a grade; Q-9);
`Description` is the declared standard when present. One `IfcRelAssociatesMaterial` (S33) links the
material to all its exported elements in ElementId order. A declared density becomes
`IfcMaterialProperties` named `Pset_MaterialCommon` with the property `MassDensity`
(`IfcMassDensityMeasure`, kg/m3, S34 and S47). An absent density writes no property: it is
never zero and never a catalogue value. A grade never selects a density (P02.2). An element
with no pinned material gets no association.

**Profile** (`IFCX-GEO-005`). Only the rectangle profile kind exists in first-scope stores and is
the only one exported. Its width and height are the values re-read from the pinned definition. The
declared designation, if any, is the rectangle profile's `ProfileName`. The profile ContentDigest is
exported as a property (section 9) but no `IfcBeamType`, material profile set or catalogue
reference is created (L-07). The declared section values of P02.2 (area, mass per metre, surface
per metre) are not exported in 0.1.0 (L-06; Q-10).

## 8. Quantities

### 8.1 Rule

`IFCX-QTY-001`. Only **published current quantities** are exported. A quantity is written only when
its stored record exists, its signature equals the one the engine computes from the present authored
inputs, the stored value and status equal the engine's own computation and the status is `current`
(the `assess` rule of P04; a forged value under a matching signature is stale). Nothing is
recomputed at export time. The exporter may call the kernel for frames (section 6) but never to
produce a quantity the store lacks (`IFCX-QTY-001`, plan card E03 rule "published current only").

| Freshness | Meaning (F04 section 5, P04) | Written? |
|-----------|-----------------------------|----------|
| `current` | Record present, signature and value match | yes |
| `stale` | Record present, signature or value differ | no value; status in `BeamCraft_Element`; omission listed |
| `missing` | No record stored | no value; status `missing`; omission listed |
| `unknown` | Required input absent (density) | no value, never zero; status `unknown`; omission listed |
| `unsupported` | Geometry outside the scope | no value; the element is already skipped (section 4.2) |

A quantity omission is a **warning** (`IFCX-QTY-002`), not an error, and does not fail the export,
including in strict mode. A `current` record with a non-finite value, or a volume that is not
greater than zero, is `INTEGRITY_ERROR` and refuses the export (`IFCX-QTY-005`).

### 8.2 Mapping of the published quantities

Published today (P04): `volume_mm3` and `mass_kg` with their statuses, shape and mass signatures
and an algorithm label. Units: volume is divided by `1e9` to cubic metres, as F06 section 2 does;
mass is already kilograms.

| BeamCraft quantity | Element kinds | IFC quantity (set) | Rule |
|--------------------|---------------|--------------------|------|
| Volume (body as authored, including holes) | beam, column, plate | `NetVolume` (`IfcQuantityVolume`) in `Qto_BeamBaseQuantities`, `Qto_ColumnBaseQuantities`, `Qto_PlateBaseQuantities` (S44 to S46) | always when current |
| Same volume | same | `GrossVolume` in the same sets | only for an element without operations: a plate without holes, and every member (`IFCX-QTY-004`) |
| Mass (volume times density) | same | `NetWeight` (`IfcQuantityWeight`) | when current; kilograms |
| Same mass | same | `GrossWeight` | same condition as `GrossVolume` |

The base quantity sets are the buildingSMART-defined ones for each product type; each
`IfcElementQuantity` is named exactly as the set (`Qto_...`). A plate with holes has no
`GrossVolume` and no `GrossWeight`, because the volume of the plate without its holes is not a
published record and the exporter does not recompute it. In IFC4X3_ADD2 the plate set has weight
quantities; under an IFC 4 target (Q-1) the plate weights would go to `BeamCraft_Element` instead.

Not published today and therefore not exported (L-06): beam `Length`, `CrossSectionArea`,
`OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, plate `Width`, `Perimeter`, `GrossArea`,
`NetArea`, and the amended P04 fields `lengthMm`, `cutLengthMm`, `nominalMassKg`, `surfaceAreaM2`.
When P04 publishes one of them as a current record, a contract version adds its row; the rule
"published current only" does not change.

### 8.3 Provenance

`IFCX-QTY-006`. `BeamCraft_Element` carries the volume and mass statuses, the shape and mass
signatures (64 lowercase hex digits) and the algorithm label of the records, so a reader can
compare the file with the derived layer. These signatures are P04's own, not the F06 section 7
keys.

## 9. Property sets

### 9.1 `BeamCraft_Element`

One `IfcPropertySet` per exported element, linked by one `IfcRelDefinesByProperties`
(`IFCX-PSET-001`). Every property is an `IfcPropertySingleValue`; names are fixed here.

| Property | Type | Content | Present |
|----------|------|---------|---------|
| `ElementId` | `IfcIdentifier` | Canonical 36-character ElementId | always |
| `ElementKind` | `IfcLabel` | `plate`, `beam` or `column` | always |
| `SchemaVersion` | `IfcLabel` | `bim:schemaVersion` | always |
| `GeometryMode` | `IfcLabel` | `bim:geometryMode` | always |
| `ProfileDigest` | `IfcIdentifier` | Full ContentDigest of the profile definition | members |
| `MaterialDigest` | `IfcIdentifier` | Full ContentDigest of the material definition | with a material |
| `HoleCount` | `IfcInteger` | Number of through holes | plates |
| `VolumeStatus`, `MassStatus` | `IfcLabel` | `current`, `stale`, `missing`, `unknown` | always |
| `ShapeSignature`, `MassSignature` | `IfcIdentifier` | Signatures of the stored records | when a record exists |
| `QuantityAlgorithm` | `IfcLabel` | Algorithm label of the stored record | when a record exists |
| `Label` | `IfcText` | The full label | only when the name was truncated |

`IfcRoot.Name` is `bim:name` (S15 to S17). A label longer than 255 characters (`IfcLabel` is limited
to 255, S04) is truncated to 255 characters in `Name` and written in full in `Label`, and the truncation is
listed (`IFCX-PSET-004`). No custom set name starts with `Pset_` or `Qto_` (`IFCX-PSET-002`), because
those prefixes belong to the specification's own sets. The actor id (`bim:createdBy`), the
revision number of the element, file paths, host names and any time are not written
(`IFCX-PSET-003`; L-08). The project-level data (ProjectId, RevisionId, contract and exporter
versions) go in the header of section 11.5 and in the report, not in a property set.

## 10. Units

`IFCX-UNIT-001`. One `IfcUnitAssignment` on the project (S41 to S43):

| Unit type | Definition |
|-----------|-----------|
| `LENGTHUNIT` | `IfcSIUnit(.LENGTHUNIT., .MILLI., .METRE.)` |
| `PLANEANGLEUNIT` | `IfcSIUnit(.PLANEANGLEUNIT., $, .RADIAN.)` |
| `AREAUNIT` | `IfcSIUnit(.AREAUNIT., $, .SQUARE_METRE.)` |
| `VOLUMEUNIT` | `IfcSIUnit(.VOLUMEUNIT., $, .CUBIC_METRE.)` |
| `MASSUNIT` | `IfcSIUnit(.MASSUNIT., .KILO., .GRAM.)` |
| `MASSDENSITYUNIT` | `IfcDerivedUnit` of kilogram to the power 1 and metre to the power -3 |

`IFCX-UNIT-002`. The internal unit is the millimetre (F04 section 3), so **no length is scaled**:
the number in the file is the internal number. Only volume is converted, from mm3 to m3, because the
quantity sets are in the project's volume unit. The area and angle units are declared for
completeness; no area or angle value is written in 0.1.0 (roll is carried by the frame vectors).
The `IfcGeometricRepresentationContext.Precision` is `0.001` (millimetres), the absolute geometric
floor of F04 section 3.

## 11. Determinism

`IFCX-DET-006`. The same revision, the same options and the same exporter version give a
byte-identical file and a byte-identical report. Nothing else influences them: no clock, random
source, locale, thread count, hash-map iteration order, environment variable, working directory or
output path (`IFCX-DET-006`; F06 section 2 applies the same rule to the kernel).

### 11.1 Ordering

Section 4.3. Within an element the entities are emitted in the fixed order of 11.2. Sets with no
natural order (the elements of a relationship) are sorted by ElementId.

### 11.2 Instance numbers

STEP instance names `#1`, `#2`, ... are assigned by one counter in this emission order
(`IFCX-DET-002`):

1. units, then the representation context and the Body sub-context;
2. the project, the site, the building, their placements and the two aggregation relationships;
3. the materials in ascending digest order, each followed by its properties;
4. for each element in ElementId order: placement entities, profile, solid, shape representation,
   product definition shape, the product, its property set, its quantity set (if any) and the two
   `IfcRelDefinesByProperties`;
5. the containment relationship, then one material association per material in digest order.

Forward references are allowed in a STEP file; the counter order, not the reference order, is fixed.

### 11.3 Real numbers

`IFCX-DET-003`. A REAL is the shortest decimal digit string `d1 d2 ... dn` that parses back to the
same binary64 (ties to the value closest to the exact one), with scientific exponent `x`
(value = `d1.d2...dn` times 10 to the `x`). Notation:

- zero, including negative zero, is `0.`; a negative value has a leading `-`;
- if `-6 <= x <= 14` the notation is positional: digits, zero fill if needed, and a decimal point,
  with at least the point after an integral value (`1000.`, `0.5`, `0.000001`);
- otherwise the notation is `d1.d2...dn` followed by `E` and the exponent without `+` and without
  leading zeros (`1.E15`, `1.25E-9`).

There is never a locale separator, a leading `+`, a bare exponent without a point, or a NaN or
infinity (those are rejected before writing, `IFCX-GEO-006`). Vectors are in the case file
(`E01-POS-26`).

### 11.4 Strings and encoding

`IFCX-DET-004`. The file is 7-bit ASCII with LF line ends. In a string, an apostrophe is doubled and
a backslash is doubled. Each maximal run of non-ASCII characters is one segment: `\X2\` followed by
four upper-case hex digits per UTF-16 unit and `\X0\` when the run is all in the basic multilingual
plane, otherwise `\X4\` followed by eight upper-case hex digits per code point and `\X0\`. Control
characters cannot occur (F04 label rule). These are the escapes of the STEP physical file as
described for IFC (S02); E02 must confirm them against an independent reader (L-14).

### 11.5 Header

`IFCX-DET-005`. The fixed header, with the exporter version `V` and the time stamp `T`:

```text
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('BeamCraft IFC export profile 0.1.0'),'2;1');
FILE_NAME('beamcraft-export.ifc','T',(''),(''),'BeamCraft IFC writer V','BeamCraft V','');
FILE_SCHEMA(('IFC4X3_ADD2'));
ENDSEC;
```

`T` is the UTC calendar time of the 48-bit millisecond timestamp inside the RevisionId, truncated to
whole seconds, in the form `YYYY-MM-DDTHH:MM:SS` (a pure function of the revision; Q-4 offers a
fixed value or a caller value instead). `FILE_NAME`'s file name is the fixed text shown, not the
real path. `V` is the exporter's crate version, so byte identity is claimed per exporter version.
The project-level provenance (ProjectId, RevisionId, contract version) is the report's, not the
file's.

### 11.6 Platform

`IFCX-DET-007`. Frames use only `+ - * /` and `sqrt` except for a nonzero member roll, which uses
`sin` and `cos`. Cross-target byte identity for rolled members is therefore not claimed until the
sin/cos reproducibility question of P03 and F06 section 2 is resolved. For all other elements it is
claimed, as in F06 for the meshes.

## 12. Losses

The export is lossy by design. The file never claims to be a complete copy of the project. Each loss
is listed in the report when it applies to an element and is stated here once.

| ID | Loss |
|----|------|
| L-01 | Reserved and unknown element kinds (including `reference`) and deactivated elements are not exported; they stay untouched in the store (`IFCX-SCOPE-001`, `-005`). |
| L-02 | Elements with unsupported geometry (valid non-rectangular plates; members with a non-rectangle profile) are skipped (`IFCX-GEO-004`, `-005`). |
| L-03 | A hole is its polygon. A round hole made by a generator is a polygon of N sides; the intent (diameter, centre) is not exported. |
| L-04 | FeatureIds (`fid1`), the face table, the mesh digest, the shape key and the dependency keys are not exported. No `IfcShapeAspect` is written. |
| L-05 | The derived triangle mesh, its normals and its extent are not exported; only the swept solid is. |
| L-06 | Quantities that are not current are absent (stale, missing, unknown). Quantities that no record publishes are absent (section 8.2). The declared section values (area, mass per metre, surface per metre, standard) are not exported. |
| L-07 | A profile is reduced to the swept area. There is no `IfcBeamType`, no material profile set, no catalogue reference. The profile ContentDigest is a property only. |
| L-08 | Authorship (`bim:createdBy`), per-element revision, workspace and draft state, the journal, undo and redo history and every time are not exported. |
| L-09 | Relationships between elements (F10 section 5) are not exported; none exist in the first slice. |
| L-10 | Display appearance and colours are not exported. |
| L-11 | A label longer than 255 characters is truncated in `Name`; the full text is in `Label`. |
| L-12 | The spatial structure is synthetic: one site, one building, no storeys or grids. |
| L-13 | The quaternion is not recoverable: only the rotation is, and only up to the rounding of the direction vectors (`q` and `-q` are indistinguishable). The roll angle and the two axis points are not stored as such: the point `p1`, the axis direction and the length are; `p2` and the roll are recoverable up to rounding. |
| L-14 | The escapes of 11.4 and the header form are taken from the public description of the STEP physical file and are not yet checked against an independent reader. |
| L-15 | No model view definition is satisfied or claimed; a consumer that requires one (for example a tessellation-only view) may reject the file (Q-2). |

## 13. Diagnostics and codes

The export returns one result with the envelope of F10 section 6.3 (`status` `pass`/`fail`/`incomplete`
here named `ok`, `ok_with_losses`, `failed` or `incomplete`; `primary_code`; `total_findings`;
`truncated`; at most 256 findings) and the ordering of F10 section 6.4 (severity, then code, then
ElementId, then field, bytewise). `IFCX-ERR-001`: unavailable input makes the status `incomplete` and
the primary code `DEPENDENCY_UNAVAILABLE`, ahead of any other error. A failed or incomplete export
writes no file. A finding has `code` (the `IFC_` token), `f04_code` (F04 section 8 or `null`),
`severity`, an optional `element_id` and a fixed schema `field` token; messages are fixed English
text that never echo input values, private paths or exceptions (the F06 section 3 rule).

| `IFC_` code | F04 code | Level | Meaning | Rules |
|-------------|----------|-------|---------|-------|
| `IFC_VERSION_UNSUPPORTED` | `UNSUPPORTED_VERSION` | request | Contract version or schema identifier not supported | VER-001 |
| `IFC_FORMAT_UNSUPPORTED` | `UNSUPPORTED_PROPERTY` | request | Output format other than STEP | VER-002 |
| `IFC_IMPORT_UNSUPPORTED` | `UNSUPPORTED_COMMAND` | request | Import or any IFC reading requested | SCOPE-003 |
| `IFC_ARGUMENT_INVALID` | `INVALID_ARGUMENT` | request | Missing, unknown, repeated or out-of-range option; existing or link output name | REQ-002, ERR-003 |
| `IFC_ID_INVALID` | `INVALID_ID` | request or element | Malformed ProjectId or RevisionId; malformed ElementId | REQ-001, GUID-002 |
| `IFC_REVISION_NOT_FOUND` | `NOT_FOUND` | request | Explicit revision does not exist; pinned definition does not resolve | REQ-003, MAT-004 |
| `IFC_INPUT_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | request | Revision, definition or index cannot be read completely | SCOPE-002, ERR-001 |
| `IFC_ID_DUPLICATE` | `DUPLICATE_ID` | request | Two elements with one ElementId | GUID-005 |
| `IFC_GUID_COLLISION` | `INTEGRITY_ERROR` | request | Repeated GlobalId in one file; derived attribute disagrees | GUID-006, GUID-008 |
| `IFC_INTEGRITY` | `INTEGRITY_ERROR` | request | Stored mesh differs from re-evaluation; current record with a bad value | GEO-007, QTY-005 |
| `IFC_GEOMETRY_UNSUPPORTED` | `UNSUPPORTED_GEOMETRY` | element | Unsupported plate contour, profile kind or placement form | GEO-004, GEO-005, PLC-002 |
| `IFC_PLACEMENT_INVALID` | `INVALID_ARGUMENT` | element | Quaternion outside the caps | PLC-003 |
| `IFC_NON_FINITE` | `NON_FINITE_NUMBER` | element | Non-finite shape input | GEO-006 |
| `IFC_KIND_SKIPPED` | none (warning) | element | Reserved or unknown kind | SCOPE-001 |
| `IFC_ELEMENT_DEACTIVATED` | none (info) | element | Deactivated element | SCOPE-005 |
| `IFC_QUANTITY_OMITTED` | none (warning) | element | Quantity stale, missing or unknown | QTY-002 |
| `IFC_NAME_TRUNCATED` | none (warning) | element | Label longer than 255 characters | PSET-004 |
| `IFC_LIMIT` | `LIMIT_EXCEEDED` | request | More than 65,536 elements or more than the byte cap | LIM-001, LIM-002 |
| `IFC_OUTPUT_FAILED` | `BACKEND_UNAVAILABLE` | request | The output sink or file system failed | ERR-002 |
| `IFC_CANCELLED` | `CANCELLED` | request | Cancelled before the install point | ERR-004 |

Every F04 code used above exists in F04 section 8; no code is added. Strict mode (`--strict`,
`IFCX-ERR-005`) turns every element-level skip with an F04 code into a refusal of the export with
that code (the first skip in finding order); warnings, quantity omissions and the info case do
not. Default is lenient (Q-5). The status is `ok_with_losses` when anything was skipped or any
warning was raised, `ok` otherwise.

## 14. Bounds

| Item | Bound | Origin |
|------|-------|--------|
| Elements per file | 65,536 (`IFCX-LIM-001`), checked before any output | the Pick scan bound of F06 section 9 |
| Output size | 268,435,456 bytes (256 MiB), checked before each write with checked arithmetic (`IFCX-LIM-002`) | draft proposal |
| Findings in the report | 256 listed, total counted up to 1,000,000 | F10 section 6.3 |
| Label | 1 to 256 bytes at input (F04); 255 characters in `Name` | F04 section 3, S05 |
| Holes per plate, points per hole | 64 and 1024 | A04 caps, upstream |
| Plate contour points | 3 to 1024 | F04 section 3 |
| Strings | at most 4096 bytes before escaping (F04); the escaped text counts toward the output cap | F04 section 3 |
| Instance numbers | a checked counter; overflow is `LIMIT_EXCEEDED` | the output cap makes it unreachable |

The writer allocates only for validated, capped input, uses no recursion, indexes nothing by an
input-derived position and writes through a bounded sink (`IFCX-LIM-003`, `AGENTS.md` section 4). An
element failure never panics; a request failure leaves no file.

**Output publication.** The writer streams to a temporary file in the destination directory,
checks the running size against the cap, flushes and syncs it, and installs it under the final name
only after every check passed. An existing file is not replaced unless the caller asks for it, and
a symbolic link or Windows reparse point at the output name or its parent is refused, as the byte
storage of U05 refuses them (`IFCX-ERR-002`, `IFCX-ERR-003`). A failure removes the temporary file.
No network access is involved; the output path is a local I/O-edge argument and authorises nothing.

## 15. Headless surface and writer design

E02 implements this section. Names are proposals that E02 may refine through a contract amendment.

### 15.1 Placement in the workspace (proposal, not applied)

| Piece | Path | Kind | Layer | Allowed dependencies |
|-------|------|------|-------|---------------------|
| Pure IFC writer: GlobalId, STEP lexical rules, entity templates, bounded sink | `crates/ifc` (`beamcraft-ifc`) | native | 2 | `beamcraft-geom`, `beamcraft-model` |
| Export model builder and use case: `beamcraft_engine::export` | `crates/engine` | native | 3 | adds `beamcraft-ifc` |
| Atomic output sink (temporary file, sync, non-replacing install) | `crates/host` | host | 4 | existing edges |
| CLI command `export-ifc` and helper `ifc-guid` | `apps/beamcraft-cli` | app | 5 | existing edges (calls the engine) |

`beamcraft-ifc` is not one of the three WASM crates and needs no WASM check. It has no third-party
dependency (SHA-256 comes from `beamcraft_model::digest`); a dependency would need the `AGENTS.md`
section 5 audit. It has no file system access: it writes to a `std::io::Write` sink supplied by the
caller. The writer placing at layer 2 lets the engine (layer 3) depend on it without a
same-layer edge; an equal-layer placement would be rejected by the boundary checker. These rows are
a proposed amendment to F08 section 2 and 4 (section 19) and are not applied here.

### 15.2 Writer design

1. **`guid`**: `encode(&Uuid7) -> [u8; 22]`, `decode(&str) -> Result<[u8; 16], _>`, `synthetic(role,
   keys) -> [u8; 22]`. Pure functions with the vectors of section 3.2 as tests.
2. **`step`**: the lexical layer: `real(f64) -> String` per 11.3, `string(&str) -> String` per 11.4, an
   instance counter, and a `BoundedSink` that fails when the byte cap would be exceeded.
3. **`model`**: the plain input structure the engine builds: project data, unit constants, materials,
   and a sorted list of element records that already contain **resolved numbers** (placement
   vectors, solid position and profile, depth, outline and holes, quantities with their freshness,
   names, digests). The writer derives no geometry and calls no kernel: frames come from the engine,
   which gets them from the kernel (`IFCX-GEO-008`).
4. **`emit`**: a fixed sequence of entity templates in the order of 11.2. There is no generic IFC
   object graph, no schema table and no reflection; adding an entity is adding a template and a case.
5. **`report`**: the findings, the per-element table and counts, assembled by the engine from the
   writer's element results.

Tests are the case file plus unit tests of `guid` and `step`. An independent check of the produced
file by a permitted validation tool is a gate of E02 (Q-8); this contract names none.

### 15.3 Engine API (logical)

`export_ifc(snapshot, request, sink) -> ExportReport` in `beamcraft_engine::export`, with `request`
holding: `contract_version`, `schema`, `format`, `project_id`, optional `revision_id`, optional
`project_name`, `strict`, `overwrite`. It reads only the pinned snapshot, never writes to the store,
returns the report of 15.4 and writes the file only through the sink. Two pure helpers are exposed:
`ifc_global_id(&ElementId) -> GlobalId` and `decode_global_id(&str) -> Result<[u8; 16], IdError>`
(sections 3.1 and 3.4). The Rust names are not fixed by this contract.

### 15.4 CLI and report

```text
beamcraft export-ifc --store ROOT --project-id UUID --out FILE
  [--revision UUID] [--project-name TEXT] [--schema IFC4X3_ADD2] [--format step]
  [--strict] [--overwrite] [--report FILE]
beamcraft ifc-guid --id UUID          prints the GlobalId (INVALID_ID for a malformed id)
beamcraft ifc-guid --decode GLOBALID  prints the UUID text, or INVALID_ARGUMENT
beamcraft plate-inspect --store ROOT  gains ifc_global_id per element (proposal)
```

The CLI parses, calls the engine API and prints the report as JSON with sorted keys; it computes
nothing else (`IFCX-SURF-001`). The report contains:

| Field | Content |
|-------|---------|
| `contract_version`, `schema`, `exporter_version` | strings |
| `project_id`, `revision_id` | UUID text |
| `status`, `primary_code`, `incomplete`, `total_findings`, `truncated`, `findings` | F10 envelope, section 13 |
| `counts` | `elements_total`, `exported`, `skipped` |
| `elements` | one entry per element of the revision in ElementId order: `element_id`, `ifc_global_id` (null when none), `kind`, `exported`, `representation` (`SweptSolid` or null), `volume`, `mass` (freshness tokens), `omissions` |
| `output` | `bytes` and `sha256` of the file (null when no file) |

It holds no path, no host name and no time (`IFCX-SURF-002`). The file and the report are the only
outputs; the exit code follows the existing CLI convention.

## 16. Acceptance cases

[The case file](ifc-export-cases-v0.1.json) holds 90 original logical acceptance
descriptions: 43 positive and 47 negative, with stable IDs `E01-POS-nn` and `E01-NEG-nn`, and
66 rules `IFCX-<FAMILY>-<NNN>` of which every one is exercised by at least one case. A
negative case is one where the rule fires: an element is skipped, a loss is listed, or the request
is refused with the stated F04 code. A case with a `needs` field depends on a limit that is not yet
lifted, a harness hook or an undecided proposal. They are proposed future tests; acceptance needs a
real exporter, a real store and an independent check of the written file.

| Topic | Cases |
|-------|-------|
| GlobalId encoding, decoding, ordering, synthetic ids, stability, derived attribute | `E01-POS-01` to `E01-POS-15`, `E01-NEG-01` to `E01-NEG-13` |
| Plate, beam, column, hole, rotation, material, quantities (positive) | `E01-POS-16` to `E01-POS-22`, `E01-POS-28`, `E01-POS-29`, `E01-POS-40` to `E01-POS-43` |
| Determinism, REAL and string text, units, header | `E01-POS-23` to `E01-POS-27`, `E01-POS-39` |
| Scope, headless, report | `E01-POS-30` to `E01-POS-38`, `E01-NEG-36`, `E01-NEG-43` |
| Skips, losses, stale/missing/unknown quantities, integrity | `E01-NEG-14` to `E01-NEG-29` |
| Request errors, limits, output failures | `E01-NEG-30` to `E01-NEG-35`, `E01-NEG-37` to `E01-NEG-42`, `E01-NEG-44` to `E01-NEG-47` |

### 16.1 Rule catalogue

Severity `error` rules name the F04 code of the failing condition; `mapping` rules have no runtime
diagnostic and are checked by the acceptance cases. Counts: 66 rules, 27 error, 3 warning, 3 info,
33 mapping.

| Rule | Severity | F04 code | Statement |
|------|----------|----------|-----------|
| IFCX-SCOPE-001 | warning | - | Only the kinds plate, beam and column are exported. Any other kind, reserved or unknown, is skipped, stays untouched in the store and is listed (F10 DV-KIND-002). |
| IFCX-SCOPE-002 | error | DEPENDENCY_UNAVAILABLE | The export reads exactly one verified committed revision, the published head by default or an explicit RevisionId. A revision that cannot be read and verified completely is refused; there is no partial export. |
| IFCX-SCOPE-003 | error | UNSUPPORTED_COMMAND | IFC import, and any reading of IFC by BeamCraft, is outside this contract. A request to import is refused. |
| IFCX-SCOPE-004 | error | INTEGRITY_ERROR | Export is read-only: it changes no authored state, derived state, revision or generation. |
| IFCX-SCOPE-005 | info | - | An element whose lifecycle is deactivated is not exported and is listed (F10 section 3.2). |
| IFCX-VER-001 | error | UNSUPPORTED_VERSION | contract_version is exactly 0.1.0 and the requested schema identifier is in the supported set, which is IFC4X3_ADD2 only (open question Q-1). |
| IFCX-VER-002 | error | UNSUPPORTED_PROPERTY | The only output format is the clear-text STEP physical file. Any other requested format is refused. |
| IFCX-VER-003 | info | - | No model view definition is claimed. The header names none and the file does not assert conformance to one. |
| IFCX-GUID-001 | mapping | - | The GlobalId of an element is E(ElementId) of section 3.1: alphabet, first character 0 to 3, big-endian bit order, 22 characters. No other value is written to the file or to the derived attribute. |
| IFCX-GUID-002 | error | INVALID_ID | An ElementId that is not a canonical lowercase UUIDv7 with the RFC variant gets no GlobalId. There is no repair, lowercasing, truncation, padding or hash fallback. |
| IFCX-GUID-003 | error | INVALID_ARGUMENT | The decoder accepts exactly 22 characters of the alphabet whose first character is 0, 1, 2 or 3. Everything else is rejected. |
| IFCX-GUID-004 | mapping | - | Entities that are not elements get a synthetic GlobalId from the SHA-256 construction of section 3.5, a function of the ProjectId and the fixed role keys only. |
| IFCX-GUID-005 | error | DUPLICATE_ID | Two elements of the revision with the same ElementId refuse the export (F10 DV-BASE-002). |
| IFCX-GUID-006 | error | INTEGRITY_ERROR | Two entities of one file with the same GlobalId, including a synthetic one equal to an element one, refuse the export. |
| IFCX-GUID-007 | mapping | - | The GlobalId is a pure function of the ElementId. It does not change with rename, edit, placement, revision, export run, platform or time; a copy has a new ElementId and so a new GlobalId. |
| IFCX-GUID-008 | error | INTEGRITY_ERROR | When the derived layer carries bim:ifcGlobalId, it must equal E(bim:id). A different value is an integrity error, never a second identity. |
| IFCX-REQ-001 | error | INVALID_ID | project_id and an explicit revision_id are full canonical UUIDv7 text. |
| IFCX-REQ-002 | error | INVALID_ARGUMENT | A required option is missing, an option is unknown or repeated, or a value is out of range. |
| IFCX-REQ-003 | error | NOT_FOUND | An explicit revision_id names no committed revision of the store. |
| IFCX-PLC-001 | mapping | - | The element placement becomes one IfcLocalPlacement relative to the building placement: Location is translation_mm, Axis is R(q) applied to (0,0,1), RefDirection is R(q) applied to (1,0,0). |
| IFCX-PLC-002 | error | UNSUPPORTED_GEOMETRY | A placement with scale, shear, reflection or a matrix form is not exported; the element is skipped. |
| IFCX-PLC-003 | error | INVALID_ARGUMENT | A stored quaternion outside the F04 section 3 caps (component range, squared norm) skips the element. |
| IFCX-PLC-004 | mapping | - | The quaternion is normalized before use; q and -q give identical output; negative zero is written as zero. |
| IFCX-GEO-001 | mapping | - | Every exported element has exactly one shape representation, identifier Body, type SweptSolid, with exactly one IfcExtrudedAreaSolid item. |
| IFCX-GEO-002 | mapping | - | A beam or column is a rectangle profile centred on the axis, extruded from the point p1 along the member axis by the axis length, in the frame the kernel derived (F04 P-2, P-3). |
| IFCX-GEO-003 | mapping | - | A plate is its outline in the plate frame, extruded along the plate normal by the thickness. Through holes become inner curves of the profile. No boolean entity and no opening element is written. |
| IFCX-GEO-004 | error | UNSUPPORTED_GEOMETRY | An element whose geometry status is not a supported sealed result (for example a valid non-rectangular plate) is skipped. No geometry is exported that BeamCraft itself does not certify. |
| IFCX-GEO-005 | error | UNSUPPORTED_GEOMETRY | A member whose profile kind is not rectangle is skipped. |
| IFCX-GEO-006 | error | NON_FINITE_NUMBER | A non-finite stored number in a shape input skips the element. |
| IFCX-GEO-007 | error | INTEGRITY_ERROR | The stored derived mesh of an exported element must equal the kernel's re-evaluation of its authored inputs (digest equality). A mismatch refuses the whole export. |
| IFCX-GEO-008 | mapping | - | The exporter takes frames and dimensions from the kernel's own derivation and never re-implements them. The tessellation of the written solid reproduces the published volume within the CR-TOL-001 quantity bound. |
| IFCX-GEO-009 | mapping | - | Version 0.1.0 writes no tessellated or boundary-representation item and no second representation (open question Q-2). |
| IFCX-MAT-001 | mapping | - | Each distinct pinned material digest used by an exported element becomes one IfcMaterial, associated with all its elements by one IfcRelAssociatesMaterial. An element without a material gets no association and no invented default. |
| IFCX-MAT-002 | mapping | - | IfcMaterial.Name is the declared grade, otherwise the text BC- followed by the full 64 lowercase hex digits of the digest; Description is the declared standard when present. |
| IFCX-MAT-003 | mapping | - | MassDensity is written only when the definition authors a density, as received. An unknown density is never written as zero or any default. |
| IFCX-MAT-004 | error | NOT_FOUND | A pinned profile or material digest of an exported element that does not resolve to a definition refuses the export (F10 DV-BASE-006). |
| IFCX-QTY-001 | mapping | - | A quantity is written only from a stored record whose signature equals the freshly computed one and whose status is current. Nothing is recomputed at export time. |
| IFCX-QTY-002 | warning | - | A quantity whose freshness is stale, missing, unknown or unsupported is not written (no zero, no previous value); its status is written in the BeamCraft_Element property set and the omission is listed. |
| IFCX-QTY-003 | mapping | - | Mapping of the published quantities: volume to NetVolume, mass to NetWeight, in the base quantity set of the product type and in SI units (section 8.2). |
| IFCX-QTY-004 | mapping | - | GrossVolume and GrossWeight are written, with the same values, only for an element without operations (a plate without holes, every member). Otherwise they are omitted. |
| IFCX-QTY-005 | error | INTEGRITY_ERROR | A stored record with status current whose value is not finite, or whose volume is not greater than zero, refuses the export. |
| IFCX-QTY-006 | mapping | - | Every written quantity set also names the algorithm label and signatures of the records in BeamCraft_Element, so a reader can compare them with the derived layer. |
| IFCX-PSET-001 | mapping | - | Each exported element has one property set BeamCraft_Element with the properties of section 9.1, and no other property set in 0.1.0. |
| IFCX-PSET-002 | mapping | - | A BeamCraft property set name never starts with Pset_ or Qto_; only the buildingSMART quantity sets of section 8.2 use the Qto_ prefix. |
| IFCX-PSET-003 | mapping | - | No property, string or header field contains an actor id, credential, private path, host name, user name or wall-clock time. |
| IFCX-PSET-004 | warning | - | A label longer than 255 characters is written truncated to 255 characters in IfcRoot.Name, with the full label in the property Label, and the truncation is listed. |
| IFCX-UNIT-001 | mapping | - | The unit assignment is: length milli metre, plane angle radian, area square metre, volume cubic metre, mass kilogram, mass density kilogram per cubic metre (derived unit). |
| IFCX-UNIT-002 | mapping | - | Lengths and coordinates are written exactly as the internal millimetre values. No scaling is applied to any length. |
| IFCX-DET-001 | mapping | - | Elements are processed and listed in ascending canonical ElementId order (byte comparison of the 36-character lowercase text); materials in ascending digest order. |
| IFCX-DET-002 | mapping | - | STEP instance numbers are assigned by a counter in the fixed emission order of section 11.2. |
| IFCX-DET-003 | mapping | - | A REAL is written as the shortest decimal text that parses back to the same binary64, in the notation of section 11.3. |
| IFCX-DET-004 | mapping | - | Strings are escaped as in section 11.4. The whole file is 7-bit ASCII with LF line ends. |
| IFCX-DET-005 | mapping | - | Header fields are the fixed values of section 11.5; the only variable one is the time stamp derived from the RevisionId. |
| IFCX-DET-006 | mapping | - | The same revision, options and exporter version give a byte-identical file and report. No clock, random source, locale, thread count, hash-map order, environment variable or path influences them. |
| IFCX-DET-007 | info | - | Cross-platform byte identity is claimed only when no member has a nonzero roll; with a nonzero roll it depends on the unresolved sin/cos reproducibility question (F06 section 2). |
| IFCX-ERR-001 | error | DEPENDENCY_UNAVAILABLE | The export result uses the F10 section 6.3 envelope precedence: an unavailable input makes the status incomplete and the primary code DEPENDENCY_UNAVAILABLE, before any other error. |
| IFCX-ERR-002 | error | BACKEND_UNAVAILABLE | A failing output sink or file system gives no output file. The file is written to a temporary name and installed only after complete success. |
| IFCX-ERR-003 | error | INVALID_ARGUMENT | An existing output file is not replaced unless overwrite is requested; a symbolic link or reparse point at the output name or its parent is refused. |
| IFCX-ERR-004 | error | CANCELLED | Cancellation before the install point discards all work and leaves no file. |
| IFCX-ERR-005 | mapping | - | In strict mode any skipped element refuses the export with the code of the first skip. Omitted quantities do not. |
| IFCX-LIM-001 | error | LIMIT_EXCEEDED | At most 65,536 elements are exported in one file; more refuse the export before any output is produced. |
| IFCX-LIM-002 | error | LIMIT_EXCEEDED | The output is at most 268,435,456 bytes. The writer checks the running size before each write; exceeding it refuses the export and removes the temporary file. |
| IFCX-LIM-003 | mapping | - | Every count in the writer is bounded by a cap already enforced upstream (F04 section 3 and the A04 hole caps) or by LIM-001/002; no allocation is sized by an unchecked input, no recursion is used and no input-derived index is used. |
| IFCX-LIM-004 | mapping | - | The report keeps at most 256 findings in the F10 order and states total_findings and truncated; the per-element table is complete. |
| IFCX-SURF-001 | mapping | - | Every export is an engine API call first; the CLI command adds no computation and no private path (AGENTS.md section 3). |
| IFCX-SURF-002 | mapping | - | The export report of section 15.4 is deterministic, lists every element's GlobalId, representation, quantity freshness and omissions, and carries the SHA-256 of the file. |


## 17. Open questions for the owner

None of these is decided by this contract. The stated default is what the draft assumes so that E02 can
start, and each is cheap to change before the contract is frozen.

1. **Q-1, schema edition.** Target `IFC4X3_ADD2` (draft default) or `IFC4` (ADD2 TC1), or both? IFC 4.3
   is the current edition and has plate weights; IFC 4 is read by more tools today. Choosing IFC 4
   changes the schema identifier, the plate weight rows and the enumeration check.
2. **Q-2, model view and tessellation.** Is a specific model view definition a target? Some views
   require tessellated geometry and would reject swept solids. Should 0.1 add a tessellated
   representation next to the swept solid, or replace it? (Draft: swept solid only, no claim.)
3. **Q-3, ProjectId.** The experimental stores record no ProjectId. Draft: an explicit required
   argument. Alternative: store one in the project entry (U-task) and derive project-level GlobalIds
   from it.
4. **Q-4, header time stamp.** Draft: derived from the RevisionId timestamp. Alternatives: a fixed
   constant, or a caller-supplied value.
5. **Q-5, default mode.** Draft: lenient (skip and report), strict as an option. Alternative: strict by
   default.
6. **Q-6, derived `bim:ifcGlobalId`.** Persist it in the derived layer (changes the U02 schema digest)
   or compute only at export and in the CLI? Which task owns the change if persisted?
7. **Q-7, spatial structure.** Draft: one synthetic site and building, no storeys. Does a level concept
   belong to the model before the first IFC export?
8. **Q-8, independent validation.** Which tool checks E02's output? An external oracle process under the
   copyleft rule of `AGENTS.md` section 2, or a hosted validator (new network access is the owner's
   decision, section 5)? This contract names none.
9. **Q-9, material name without a grade.** Draft: `BC-` plus the full digest. Alternative: a fixed text
   such as `Unspecified` with the digest in a property.
10. **Q-10, declared section values.** Export P02.2's declared area, mass per metre and surface per
    metre as clearly labelled declared properties (never in a quantity set)? Draft: no.
11. **Q-11, identity of non-element entities.** Is the SHA-256 construction of 3.5 acceptable for the
    GlobalIds of the project, site, building and relationship objects, or should any of them be
    caller-supplied?

## 18. Public specification index

All pages are from the public buildingSMART IFC specification. Section numbers are not quoted
because the editions number them differently; the entity or definition name identifies the page.
URLs use the pattern `https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/<Name>.htm`.
**Verification status:** in the session that wrote this draft, direct fetches of these pages returned
HTTP 403. Content was taken from web search result excerpts of the same pages (marked "excerpt") and
from the author's knowledge of the public schema (marked "not seen"). A reviewer must open every
link before the contract is frozen; a row whose description proves wrong is a defect of this
document, not of the specification.

| ID | Definition | URL | Used for | State |
|----|------------|-----|----------|-------|
| S01 | IFC 4.3.2.0 (IFC4X3_ADD2) edition | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/> | Schema identifier and edition (published by buildingSMART as ISO 16739-1:2024; not seen in the pages) | not seen |
| S02 | IFC-SPF, STEP physical file format | <https://technical.buildingsmart.org/standards/ifc/ifc-formats/ifc-spf/> | File structure, header, string escapes (the underlying ISO 10303-21 text is not public and is not used) | not seen |
| S03 | IfcGloballyUniqueId | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcGloballyUniqueId.htm> | 22 characters; alphabet 0-9 A-Z a-z _ $; first byte fills the first two characters, further bytes in groups of three give four characters; first character 0 to 3 | excerpt |
| S04 | IfcLabel | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcLabel.htm> | Name type, limit of 255 characters | not seen |
| S05 | IfcIdentifier | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcIdentifier.htm> | Identifier property values | not seen |
| S06 | IfcText | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcText.htm> | Unbounded text property values | not seen |
| S10 | IfcProject | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcProject.htm> | Unit assignment and representation contexts | not seen |
| S11 | IfcSite | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcSite.htm> | Synthetic site | not seen |
| S12 | IfcBuilding | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcBuilding.htm> | Synthetic building | not seen |
| S13 | IfcRelAggregates | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcRelAggregates.htm> | Project, site, building decomposition | not seen |
| S14 | IfcRelContainedInSpatialStructure | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcRelContainedInSpatialStructure.htm> | Elements in the building | not seen |
| S15 | IfcBeam | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcBeam.htm> | Beam entity, Body representation, predefined type | not seen |
| S16 | IfcColumn | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcColumn.htm> | Column entity | not seen |
| S17 | IfcPlate | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcPlate.htm> | Plate entity | excerpt (page listed) |
| S18 | IfcLocalPlacement | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcLocalPlacement.htm> | Relative object placement | not seen |
| S19 | IfcAxis2Placement3D | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcAxis2Placement3D.htm> | Location, Axis (Z), RefDirection (X) | not seen |
| S20 | IfcAxis2Placement2D | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcAxis2Placement2D.htm> | Profile position | not seen |
| S21 | IfcCartesianPoint | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcCartesianPoint.htm> | Points | not seen |
| S22 | IfcDirection | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcDirection.htm> | Directions | not seen |
| S23 | IfcProductDefinitionShape | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcProductDefinitionShape.htm> | Shape of a product | not seen |
| S24 | IfcShapeRepresentation | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcShapeRepresentation.htm> | Identifier Body, type SweptSolid | not seen |
| S25 | IfcGeometricRepresentationContext | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcGeometricRepresentationContext.htm> | Model context, precision | not seen |
| S26 | IfcGeometricRepresentationSubContext | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcGeometricRepresentationSubContext.htm> | Body sub-context | not seen |
| S27 | IfcExtrudedAreaSolid | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcExtrudedAreaSolid.htm> | Sweep of a profile along a direction by a depth; position; example with a rectangle profile | excerpt (page listed on another host) |
| S28 | IfcRectangleProfileDef | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcRectangleProfileDef.htm> | XDim and YDim; profile centred on its position | excerpt (via a subtype page) |
| S29 | IfcArbitraryClosedProfileDef | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcArbitraryClosedProfileDef.htm> | Outer curve profile | not seen |
| S30 | IfcArbitraryProfileDefWithVoids | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcArbitraryProfileDefWithVoids.htm> | Outer boundary and inner boundaries (holes); inner curves closed, inside the outer curve, not touching | excerpt |
| S31 | IfcPolyline | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcPolyline.htm> | Curves of the profile | not seen |
| S32 | IfcMaterial | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcMaterial.htm> | Name (required), Description, Category | excerpt |
| S33 | IfcRelAssociatesMaterial | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcRelAssociatesMaterial.htm> | Material to element association | excerpt (mentioned on S32) |
| S34 | IfcMaterialProperties | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcMaterialProperties.htm> | Named property group linked to a material | excerpt |
| S35 | IfcPropertySet | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcPropertySet.htm> | Custom property set | not seen |
| S36 | IfcPropertySingleValue | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcPropertySingleValue.htm> | Single typed value | not seen |
| S37 | IfcRelDefinesByProperties | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcRelDefinesByProperties.htm> | Property and quantity set to element | not seen |
| S38 | IfcElementQuantity | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcElementQuantity.htm> | Quantity set | not seen |
| S39 | IfcQuantityVolume | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcQuantityVolume.htm> | Volume quantity | not seen |
| S40 | IfcQuantityWeight | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcQuantityWeight.htm> | Weight quantity | not seen |
| S41 | IfcUnitAssignment | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcUnitAssignment.htm> | Project units | not seen |
| S42 | IfcSIUnit | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcSIUnit.htm> | SI units with prefix | not seen |
| S43 | IfcDerivedUnit and IfcDerivedUnitElement | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcDerivedUnit.htm> | Mass density unit | not seen |
| S44 | Qto_BeamBaseQuantities | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/Qto_BeamBaseQuantities.htm> | Length, CrossSectionArea, surface areas, GrossVolume, NetVolume, GrossWeight, NetWeight (the IFC 4 edition page was seen in search: https://standards.buildingsmart.org/IFC/RELEASE/IFC4/FINAL/HTML/schema/ifcsharedbldgelements/qset/qto_beambasequantities.htm) | excerpt (IFC 4 edition) |
| S45 | Qto_ColumnBaseQuantities | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/Qto_ColumnBaseQuantities.htm> | Column base quantities | not seen |
| S46 | Qto_PlateBaseQuantities | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/Qto_PlateBaseQuantities.htm> | Width, Perimeter, GrossArea, NetArea, GrossVolume, NetVolume; weights in the 4.3 edition; net quantities account for openings by subtraction | excerpt |
| S47 | Pset_MaterialCommon and IfcMassDensityMeasure | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/Pset_MaterialCommon.htm> | MassDensity property of a material | not seen |
| S48 | IfcPlateTypeEnum | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcPlateTypeEnum.htm> | Eleven values including NOTDEFINED | excerpt |
| S49 | IfcBeamTypeEnum | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcBeamTypeEnum.htm> | Value BEAM | not seen |
| S50 | IfcColumnTypeEnum | <https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcColumnTypeEnum.htm> | Value COLUMN | not seen |

## 19. Mapping to plan tasks, proposed amendments and log

- **E02** implements sections 3 to 15 and the case file; its acceptance includes an independent
  validation (Q-8) and an end-to-end CLI test.
- **E03** (BOM and reports) reads published current quantities by the same rule as section 8.
- **E04** (IFC import research) may reuse section 3's decoder only; it may not widen this contract.
- **U02/U-task** (if Q-6 is yes): the `bim:ifcGlobalId` attribute and its verifier.
- **F08 amendment (proposed, not applied):** add the rows of section 15.1 to the module table and to
  the contract-area table (writer, engine export, host sink), with `beamcraft-engine` allowed to
  depend on `beamcraft-ifc`.
- **F10 amendment (proposed, not applied):** none needed; section 8 of F10 already assigns the
  registry as the export set.

Log:

- 2026-10-17: initial draft 0.1.0 (E01).
