---
slug: explanation/naming-and-collisions
type: explanation
audience: a developer who wants to understand how a URL path becomes an enum variant name and why collisions happen
goal: understand the naming pipeline (strip prefix, clean, case, prefix/suffix, dedupe) and the trade-offs of each collision strategy
scope:
  in: the five naming stages, segment-aware prefix stripping, separator handling, digit handling, the three collision strategies and why skip is the default, name() parity with the TS client
  out: how to set the keys (see guides/configure-naming), key types/defaults (see reference/configuration)
outline:
  - Why names matter (Rust variants, TS client keys, query-key parity via name())
  - Stage 1 — strip the name prefix (segment-aware, URLs untouched)
  - Stage 2 — clean the path (braces dropped, separators to underscores, root fallback)
  - Stage 3 — prepend the method (include_method_in_names)
  - Stage 4 — casing and identifiers (case list, digit boundaries, keyword safety with raw identifiers)
  - Stage 5 — collision resolution and its trade-offs
source_refs:
  - src/configuration/naming.rs (generate_route_name, strip_name_prefix, clean_route_path_for_name, VariantNamer)
  - src/utils/case.rs (parse_case, convert_to_case_preserve_numbers, sanitize_identifier)
  - src/generators/rust.rs (to_valid_ident, enum emission)
  - docs/IMPROVEMENTS.md (bugs 4, 6, 7, 9 — maintainer-recorded rationale)
approved_by: self (autonomous pipeline)
---
