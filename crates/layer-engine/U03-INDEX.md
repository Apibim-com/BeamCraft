# U03 UUID and relationship index report

Date: 2026-10-16. Base: `e081859` (verified `origin/master`), branch
`feat/u03-uuid-relationship-index`. Status: **experimental engine API slice; full U03
acceptance remains open** (no CLI/host command, see below). Evidence is Windows x64, Rust
1.96.0, offline; `cargo deny` and a wasm32 target are not installed here and were not run.

## What is implemented

| Plan item | Implementation | Evidence (`tests/index.rs`) |
|---|---|---|
| Full UUIDv7 | `ElementId(u128)`: strict canonical parse, canonical path, no prefix lookup | `ids_are_full_uuidv7_and_same_millisecond_ids_are_never_shortened` (13 bad texts, 7 bad paths, 200 elements sharing one millisecond prefix, prefix lookups refused) |
| ID to prim path | `ElementIndex::lookup`/`lookup_path`; path is derived from the ID, never the reverse | same test; `reader_index_resolves_ids_provenance_and_typed_relationships` |
| Duplicate / short / malformed / mismatched IDs | Re-derived from snapshot identity, `bim:id` and path; fixed phase order, so the failure does not depend on page order | `duplicate_malformed_misplaced_and_mismatched_identities_are_rejected`, `tampered_pages_cannot_forge_identity_provenance_or_bounds` |
| Relationship validation | Every target resolved to element/definition/prim; dangling, short and property-path targets of owned prims rejected; foreign dangling links preserved as `Unresolved` | `dangling_short_and_unsupported_targets_of_owned_prims_are_rejected`, `foreign_ancestors_descendants_and_dangling_foreign_links_stay_inert_and_read_only` |
| Outgoing/incoming index | Sorted links plus a target-keyed incoming map; completeness and context token | `reader_index_...`, `delete_and_dependent_reads_collide_...` |
| Rename keeps ID | Label excluded from `structure_digest`; ID, path and provenance layer unchanged | `rename_preserves_id_path_and_structure_and_invalidates_the_previous_index` |
| Rebuild after save/reopen | Real USDA files written by the U02 authoring layer, read back from disk, rebuilt by a fresh `Reader` and a fresh `Author`; indexes compare equal | `index_rebuilds_identically_from_saved_usda_and_usdc_files_and_fresh_readers` |
| Copy with fresh IDs | `copy_plan` remaps self links, shares definitions, refuses other links (`DEPENDENCY_CONFLICT`), foreign sources, reused IDs; source unchanged; plan applied through `Author` | `copy_gets_a_fresh_id_remaps_owned_links_and_leaves_the_source_unchanged`, `foreign_sources_cannot_be_copied` |
| Deactivate / reactivate | `bim:lifecycle`; deactivated targets reported, links and identity retained | `lifecycle_is_indexed_...`, `deactivated_and_reactivated_authored_elements_keep_identity_through_reopen` |
| Delete / dependent-read collisions | `check_delete`, `certify_dependent_reads` (lifecycle write vs read of the element or its dependents; direction matters) | `delete_and_dependent_reads_collide_and_missing_link_evidence_is_unavailable` |
| Missing evidence | Partial/gapped/short delivery, missing provenance: `DEPENDENCY_UNAVAILABLE` | `partial_gapped_replayed_and_mixed_context_deliveries_are_never_complete` |
| Stale/partial/forged cannot authorize | `verify_index`: live context, complete, equal to fresh rebuild | `authorized_writes_need_a_live_complete_owned_untampered_index`, `rename_...`, `delete_and_dependent_...` |
| Bounds, cancellation | `MAX_LINKS`, per-page/array/path caps, cooperative cancel in loops | `index_budgets_and_cancellation_fail_closed` |

Error mapping: `INVALID_ID` (DV-BASE-001), `DUPLICATE_ID` (DV-BASE-002), `INVALID_ARGUMENT`
(DV-BASE-003/004), `DANGLING_RELATIONSHIP` (DV-REL-001), `DEPENDENCY_CONFLICT` (DV-REL-009/011),
`DEPENDENCY_UNAVAILABLE` (DV-REL-010), `FOREIGN_READ_ONLY` (LEW-01), `STALE_WORKSPACE` (LEW-07).

## Design decisions to review

- The index lives in the layer-engine crate beside the reader because its inputs are this
  crate's typed snapshot (`Prim`, `Provenance`, `Relationship`); the module boundary table is
  unchanged except for the task note.
- The U02 schema maps only `bim:profile` and `bim:material` links to definitions. The vocabulary
  (section 5) defines typed element-to-element records but no USD mapping, so the index treats
  every composed relationship generically, including unknown names, and applies no endpoint-kind,
  cardinality, cycle or role rules (DV-REL-002..007, 013). Those stay with F10/the owning modules.
- `bim:lifecycle` absent means `active`; present and not `active`/`deactivated` is an error for
  every element, owned or foreign (identity-level fail-closed). Owned labels are checked against
  DV-BASE-003 (1 to 256 bytes, no control characters); foreign labels are inert.
- Ownership of an element includes every contributing site, its ancestors and its descendants.
- `Page` gained a private start offset so gaps and replays are detectable; no public field changed.

## Not implemented / limits

- **CLI/host wiring is not done.** The U04 profile `u04-rust-read-v1` is closed: the worker
  advertises `relationships=false` and the host rejects `true`; the page body has no
  `relationships` member; its schema commitment is the profile string; the dispatcher exposes
  only open/read/close/query-outcome. Carrying the index needs a new profile (page body with
  relationships, capability flag, host validation, an index method or result) and an F05 schema
  amendment. This PR does not change that frozen protocol, so the end-to-end evidence here is
  in-process over real USDA files written to disk by the U02 authoring layer.
- USDC: the restricted binary profile refuses relationship specs and U02 element layers
  (`UNSUPPORTED_PROPERTY`), so USDC evidence covers identities only
  (`binary_and_text_encodings_index_the_same_identities`).
- No persisted index. No copy/delete command, no op_id ledger, no conflict-free certificate for
  non-lifecycle writes (LEW-06 engine footprints remain open), no host authorization.
- No independent-reader or SDK comparison (N03), no `cargo deny`, no wasm32 check.
- Parser containment (LEW-03) is unchanged and unproved.

## Verification

`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and
`cargo test --workspace` pass: 269 workspace tests, 48 in layer-engine (17 new in
`tests/index.rs`).
