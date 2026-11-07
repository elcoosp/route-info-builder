---
slug: guides/configure-naming
type: how-to
audience: a developer whose generated variant names collide, read poorly, or mismatch their team's casing conventions
goal: shape variant and field names with NamingConfig and pick a collision strategy
scope:
  in: all nine naming keys, worked examples for each, collision_strategy behavior differences
  out: how names are derived internally (see explanation/naming-and-collisions), key types/defaults table (see reference/configuration)
outline:
  - Task statement
  - Prerequisites (working config file)
  - Shorten names with path_prefix_to_remove (segment-aware)
  - Drop the method from names with include_method_in_names
  - Change casing (variant_case / field_case) and accepted spellings
  - Handle digits with preserve_numbers
  - Add prefixes/suffixes and extra word separators
  - Resolve collisions: skip vs append-method vs number
  - Next steps (LinkCard to explanation/naming-and-collisions)
source_refs:
  - src/configuration.rs (NamingConfig, CollisionStrategy, defaults)
  - src/configuration/naming.rs (generate_route_name, strip_name_prefix segment-awareness, VariantNamer)
  - src/utils/case.rs (accepted case spellings)
  - README.md (naming example values)
approved_by: self (autonomous pipeline)
---
