---
slug: reference/route-info-json
type: reference
audience: tooling authors consuming the JSON route map or the `scan` output
goal: look up the exact JSON shape of a RouteInfo record and every nested field
scope:
  in: the camelCase JSON schema of RouteInfo, HandlerInfo, ReturnType; where the JSON appears (scan stdout, route_map.output_path); determinism guarantees
  out: Rust-side struct docs (see reference/api), how the data is extracted (see explanation/scanning-pipeline)
outline:
  - Overview (serializable with serde camelCase keys)
  - RouteInfo fields table
  - handlerInfo fields table
  - returnType fields table
  - Example record with annotations
  - Determinism notes (sorted files, BTree collections)
source_refs:
  - src/parser.rs (RouteInfo/HandlerInfo serde rename_all camelCase)
  - src/parser/handlers.rs (ReturnType fields, BTreeSet)
  - src/lib.rs (route_map_json, generate_route_map_json)
  - src/lib.rs tests (route map contains "handlerInfo", "method": "GET")
approved_by: self (autonomous pipeline)
---
