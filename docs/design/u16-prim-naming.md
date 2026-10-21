# U16: category paths

The layer engine and CLI use the [prim naming rule](../architecture.md#prim-hierarchy-and-naming-rule).
`beamcraft-model::prim` defines the immutable element and definition categories. Computing an
element path requires both its category and its full UUIDv7. Parts share one category regardless
of whether their kind is plate, beam or column; welds and bolt groups share the connection category.

Authoring creates category folders in the authored layer and adds derived `Geometry` opinions
under the same element path. Relationships resolve identities to their actual paths before
semantic kind validation. The UUID index checks category against known kind, rejects duplicate
UUIDs across categories and resolves targets by their exact path. Copy plans retain the source's
category. Delete removes the element from its actual category folder in both layers.

Material and profile definitions have separate folders and prefixes. Their content digest still
commits the canonical isolated `/Definition` layer; embedding the definition in a category does
not change its content identity. The CLI's `prim_path` fields use the same categories. View
snapshots and picking still identify elements by UUID, independent of their USD paths.

This is an intentional pre-alpha compatibility break: only the new schema descriptor digest is
admitted. Older pins and element/definition layouts are refused; there is no migration or fallback.
Owned USDA layer pins are checked individually, including weaker layers. Foreign bytes retain
their original content and remain read-only under `/World/References/REF_<uuid>/Correction/Source`.

Regression coverage includes all categories and their relationships in real authored layers,
malformed and mismatched paths, exact target resolution, old schema refusals, copy and deletion,
and a CLI project that reopens, exports a view bundle and picks a part after label and grouping
edits. The original generated fixtures and optional OpenUSD oracle scripts use the new paths.
No new dependency or third-party asset is introduced.
