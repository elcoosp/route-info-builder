---
slug: guides/wire-authentication
type: how-to
audience: a developer whose API requires bearer authentication and whose frontend keeps the token in a constant exported from an auth module
goal: make the generated client attach the auth token where required and flag authenticated routes
scope:
  in: the two [typescript.auth] keys, what code they generate, how a route becomes requiresAuth (the `auth: JWT` extractor), the anonymous-client default
  out: naming rules (see guides/configure-naming), client function signatures (see reference/api), handler-extraction internals (see explanation/scanning-pipeline)
outline:
  - Task statement
  - Prerequisites (a token key constant exported by a frontend module)
  - Configure [typescript.auth] with token_key_import and import_from
  - Mark handlers with an `auth: JWT` extractor so the client sends the token
  - Verify the generated client imports and usage of requiresAuth
  - Omit auth to generate an anonymous client
  - Next steps (LinkCard to reference/configuration)
source_refs:
  - src/configuration.rs (AuthConfig, auth_token_key, auth_import_from)
  - src/generators/typescript/client.rs (runtime plumbing, requiresAuth)
  - src/parser/handlers.rs (auth: JWT extractor detection)
  - README.md (auth example)
approved_by: self (autonomous pipeline)
---
