# F12 stage 6: common Craft reading and specialized editing, version 0.1.0

Status: **draft contract; principle approved by the owner on 2026-10-22; not implemented**.
Part of the existing [F12](f12-project-coordination-v0.1.md) documentation package and PR.

## 1. Application and module are different concepts

Every Craft application uses the common project standard to inspect admitted content from all
sections. Specialized engineering edits require the owning module. BeamCraft can show MEP
content read-only; MEPCraft can show steel content read-only and supplies its MEP editing
capabilities. These examples describe the approved product direction, not existing MEPCraft
software or implemented MEP kinds.

An application is a shell that exposes common reading and its available modules. A module is
the F10 boundary of namespaces, kinds, commands and engineering rules. The application name
does not grant authority. This decision does not require separate model formats, cloned cores,
one crate per module, dynamic plugins or a particular application packaging/licensing scheme.
F08 dependencies and F10's initial static registration approach remain applicable.

## 2. Common read projection

**CR-READ-001.** Common reading does not require the owning engineering module to be enabled.
It identifies and lists admitted elements, exposes their readable properties and relationships,
and displays supported supplied representations. It does not invoke a disabled module to
rebuild engineering geometry. Reads and pages are pinned to one verified revision/snapshot;
selection and property inspection must refer to that same snapshot.

The logical projection contains:

| Group | Required meaning |
|-------|------------------|
| Identity | Source ProjectId, ElementId, source revision and coordination occurrence where applicable |
| Classification | Kind/module token, section, zone, label and provenance; unknown tokens remain explicit |
| Placement and links | Full admitted rigid placement, scoped relationships and their resolution status |
| Properties | Stable property name, admitted value/type, declared unit, source, freshness and edit capability |
| Representations | Optional bounded geometry references, role, source, freshness and authority |
| Capabilities | Available read/display/edit/evaluation operations and reasons for unavailable operations |

An unknown property can be shown as bounded declared data with its original name/type/unit;
the reader must not infer its engineering meaning or convert an unrecognized unit. Unavailable
values remain unavailable, not zero. This projection is disposable; USD remains the saved model.

**CR-READ-002.** Geometry is optional. A level, relation or assignment can be listed and
inspected without a solid body. If an element has no supported display representation, keep it
in the model/query results with an explicit unavailable-display status. "All applications can
view" means a common supported read contract, not support for every possible USD encoding.
Corruption, unsupported composition and exceeded input budgets retain their reader refusals.

**CR-READ-003.** Displayability, engineering validity and editability are independent. A
supplied mesh is display-only unless its engineering authority is established by the applicable
contract. A visible pipe does not imply a verified flow calculation; a visible structural mesh
does not establish certified volume, fabrication suitability or engineering-feature snapping.
F06 guarantees for certified representations remain intact. Stale or unknown results stay
labelled as such; the generic viewer never upgrades them to current certified results.

## 3. Specialized editing and authority

**CR-EDIT-001.** An engineering edit requires all of: an enabled compatible owning module,
a supported typed command, trusted authority for the active section/zone, and satisfied
ownership/dependency rules under F10 0.2. Reading a property does not make it writable. Generic
property panels dispatch the owning typed command; there is no raw USD/JSON setter bypass.
The engine rechecks capabilities and authority even when a UI offers an enabled control.

**CR-EDIT-002.** Missing, disabled or incompatible modules leave their elements read-only,
including rename, move, copy-to-native, delete and edits through generic panels. Other modules
must preserve their admitted unknown attributes, relationships and supplied representations
with their meaning unchanged when publishing edits to owned content. Re-enabling a compatible
module restores eligible edits without turning foreign references into owned elements.

**CR-EDIT-003.** Common reading allows local selection, filters and camera navigation without
modifying engineering data. Persisted shared metadata, coordination updates and assignment
decisions still use explicitly authorized shared commands from their contracts. An application
may expose those commands independently of a foreign engineering editor; none authorizes it
to modify the foreign model. For example, receiving an assignment decision does not grant
BeamCraft a MEP edit capability or MEPCraft permission to cut the structural reference.

## 4. Required acceptance cases

These are requirements for future executable tests, not runtime claims. Current F04 error
codes and F10 rules apply; no new top-level refusal codes are introduced.

| Rule | Positive case | Negative case |
|------|---------------|---------------|
| CR-READ-001 | Without MEP enabled, list a synthetic admitted MEP element and inspect declared properties at one pinned revision | Properties from a newer revision cannot silently describe the older selected occurrence |
| CR-READ-002 | Show its supported supplied representation, or explicitly list unavailable geometry | Missing geometry must not silently remove the element; unsafe input is still refused |
| CR-READ-003 | Show supplied geometry as display-only and stale quantities as stale | A visible mesh or declared property cannot imply certified volume, current mass or validated engineering snaps |
| CR-EDIT-001 | Compatible owning module and authorized section execute a typed edit | An enabled module without section rights or a raw property mutation cannot bypass F10/F04 refusals |
| CR-EDIT-002 | A steel edit preserves unfamiliar MEP content; enabling its compatible module restores eligible tools | Generic rename/move/delete of a disabled module's element is `FOREIGN_READ_ONLY`; new authoring through it is `UNSUPPORTED_COMMAND` |
| CR-EDIT-003 | Navigate both models and submit an authorized shared assignment record | Navigation changes no engineering data; a shared decision never cuts a foreign model |

The first proof uses an original synthetic unknown-module fixture, not a functioning MEPCraft
application or a new MEP implementation. It must verify preservation through save/reopen and
actual rendering of supported supplied geometry where an adapter is available. A skipped
render test is not evidence of display support.

Before implementation, specify the exact common read API, supported representation types,
property/unit mapping, capability descriptors, revision tokens, numeric budgets and any
necessary new F04/F06 versions. Frozen 0.1 contracts and runtime behaviour stay unchanged.
This approval does not advance module implementation, remote access or the broader point 5
source-provenance workflow into the MVP.
