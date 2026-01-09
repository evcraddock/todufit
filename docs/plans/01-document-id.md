# Step 1: DocumentId Type

## Goal

Add a `DocumentId` type compatible with automerge-repo for identifying documents.

## What

- UUID internally (16 bytes)
- bs58check encoding for display/sharing (human-friendly, has checksum)
- Automerge URL format: `automerge:<bs58check-id>`

## Tasks

- [ ] Create `document_id.rs` in `todu-fit-core`
- [ ] Implement `DocumentId` struct
  - `new()` - generate random
  - `to_bs58check()` / `from_bs58check()` - encode/decode
  - `to_url()` / `from_url()` - automerge URL format
- [ ] Implement Display, Serialize, Deserialize
- [ ] Add tests for roundtrip encoding

## Reference

Based on rott's implementation: `rott-core/src/document_id.rs`

## Done When

- `DocumentId` type exists and is tested
- Can generate, encode, decode, and display document IDs
