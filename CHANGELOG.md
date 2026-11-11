# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.2.0] - 2026-11-11

### Bug Fixes
- derive hook names from resolved route names by @elcoosp ([b118aa9](https://github.com/elcoosp/route-info-builder/commit/b118aa9ea57c04cdac4eedbdd707848d98eaab8a))
- derive client keys from resolved route names by @elcoosp ([1c71c17](https://github.com/elcoosp/route-info-builder/commit/1c71c17d81a93bc0afbf1e262c1bcc61b0419f1f))
- keep operationIds unique via resolved names by @elcoosp ([c7c7644](https://github.com/elcoosp/route-info-builder/commit/c7c7644fbd6a40a071fb271618ec7660ceb49e8c))
- return null if 204 not as T by @elcoosp ([9fba80c](https://github.com/elcoosp/route-info-builder/commit/9fba80c62916d387f5c7589b50d57a3d187b86b1))
- error imports incorrect path by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([d2bc64c](https://github.com/elcoosp/route-info-builder/commit/d2bc64c87f241c1fc25d11b7f70b5cd5181e4433))
- check is_builtin_type_rust, may be importable by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([e50480f](https://github.com/elcoosp/route-info-builder/commit/e50480f08af892f712edd2a20aa7311e818867aa))
- use is_builtin_type by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([b263d18](https://github.com/elcoosp/route-info-builder/commit/b263d188e4c8b2fa2b99a8959245e720acf71389))
- do not import parsed Vec::<T>::new() response by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([079b786](https://github.com/elcoosp/route-info-builder/commit/079b786d330b942512adb197ab14fc43130b04e3))
- fix duplicate imports by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([6742f7d](https://github.com/elcoosp/route-info-builder/commit/6742f7da0575d08e27ff34d347898b4cdacb6897))
- generate_route_name fix + full url by @elcoosp ([a0b5c24](https://github.com/elcoosp/route-info-builder/commit/a0b5c24abe79ef3325e0bce06d10f9a09e7d3bb5))
- correct handling with build_full_path by @elcoosp ([23407b0](https://github.com/elcoosp/route-info-builder/commit/23407b07c51ef113dd6b1997c0e1feafb31c7e23))
- handle push_operations correctly by @elcoosp ([e132229](https://github.com/elcoosp/route-info-builder/commit/e1322296a05e724c6e4e44cce4f42b8a278bb593))
- no screaming case by @elcoosp ([7f736a1](https://github.com/elcoosp/route-info-builder/commit/7f736a1dd951b7597178b519f3743c9592e8c2cd))

### CI
- add release workflow for changelog, binaries, and crates.io by @elcoosp ([f01ba3f](https://github.com/elcoosp/route-info-builder/commit/f01ba3fe94ab06dce145e09f7509a54943cc2cec))
- add formatting, lint, test, coverage, package, and semver jobs by @elcoosp ([1c429e7](https://github.com/elcoosp/route-info-builder/commit/1c429e75a8db83d1997960738e582243ed88b5fe))

### Documentation
- document resolve_route_names and generate_named by @elcoosp ([d58923f](https://github.com/elcoosp/route-info-builder/commit/d58923f9089fe71ed9119a53326ed53a6f6d6e65))
- document the 0.2.0 improvement brainstorm by @elcoosp ([9f733a8](https://github.com/elcoosp/route-info-builder/commit/9f733a8131452148b09ad5da53a63288b6ae4720))
- rewrite README for the 0.2.0 API by @elcoosp ([6ccf943](https://github.com/elcoosp/route-info-builder/commit/6ccf943aa4765f784ab4d8acaa8b3f0ac5d35dc3))
- add MIT license file by @elcoosp ([dbad0b6](https://github.com/elcoosp/route-info-builder/commit/dbad0b67dcebf0efeaadc4e630bef20e23b24a8e))
- add generated 0.2.0 changelog by @elcoosp ([5e7bdf2](https://github.com/elcoosp/route-info-builder/commit/5e7bdf239eef50b09ec13835b49231839a204cd4))
- add readme by @elcoosp ([bccafd4](https://github.com/elcoosp/route-info-builder/commit/bccafd41f2e2964bbdd487ee7f78772e69e86032))

### Features
- add single-pass route name resolution by @elcoosp ([5fb1453](https://github.com/elcoosp/route-info-builder/commit/5fb14539f95bed3543c1550c8e4264455a3d5d24))
- drive every artifact through resolved route names by @elcoosp ([19028df](https://github.com/elcoosp/route-info-builder/commit/19028dfcb44fa8ff27d7272317446a13db0a57df))
- normalize modern and legacy path syntax by @elcoosp ([6561cdb](https://github.com/elcoosp/route-info-builder/commit/6561cdb83ade56ba0dfd50a7950ec59b9414c8e3))
- add identifier sanitizing and keyword escaping by @elcoosp ([d932dfd](https://github.com/elcoosp/route-info-builder/commit/d932dfdb6c90e6d8a43ecc2753dfc2ca5cde4511))
- add TypeScript indentation helper by @elcoosp ([f5f5703](https://github.com/elcoosp/route-info-builder/commit/f5f5703e0946f5a3de0ae77bbf7606fb20b4196f))
- support Loco and Axum route styles by @elcoosp ([473e7f0](https://github.com/elcoosp/route-info-builder/commit/473e7f07320fa1be820ec4b12db80634242840fd))
- add recursive deterministic route scanner by @elcoosp ([15fb359](https://github.com/elcoosp/route-info-builder/commit/15fb3599b1ad151613c5cdf2aa7c7ffee5c65ba3))
- add binary entry point by @elcoosp ([1fcf2aa](https://github.com/elcoosp/route-info-builder/commit/1fcf2aa0b809d3cfc1d179d8fff35351d4a7b0cd))
- expose the 0.2.0 scanning and generation API by @elcoosp ([85d131a](https://github.com/elcoosp/route-info-builder/commit/85d131a14da1d41720e2e3ea0c7caa6a8086bf6e))
- add OpenAPI 3.1 skeleton generator by @elcoosp ([03f09d4](https://github.com/elcoosp/route-info-builder/commit/03f09d451a2a0e4c3e54123a9d04322cade07ea2))
- add typed error enum by @elcoosp ([d95e9b7](https://github.com/elcoosp/route-info-builder/commit/d95e9b740143e04ad2448e3240da34dc31c323a9))
- add naming and collision strategies by @elcoosp ([252ec87](https://github.com/elcoosp/route-info-builder/commit/252ec8779fc02d922b8b91ac171985c825dac989))
- add typed TOML configuration model by @elcoosp ([44cd6f3](https://github.com/elcoosp/route-info-builder/commit/44cd6f38f3d1b25e191312e32cd4a92180bce078))
- add generate, scan, and init commands by @elcoosp ([076c1aa](https://github.com/elcoosp/route-info-builder/commit/076c1aad88815720508cb43edd4b30136696a403))
- handle query params extract & add to client & hooks by @elcoosp ([b4e999a](https://github.com/elcoosp/route-info-builder/commit/b4e999af9c2bddb080e8dfb3e9095a243333d21d))
- split api (hooks) & client by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([3fab2b5](https://github.com/elcoosp/route-info-builder/commit/3fab2b5b3b7d385a99ffa2ba3a9fa8c23980664b))
- primitive error return types detection by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([809c0b1](https://github.com/elcoosp/route-info-builder/commit/809c0b15797b33a68a8d9011a95c9b0ffd2f87ac))
- api error by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([5f94d73](https://github.com/elcoosp/route-info-builder/commit/5f94d73db2f8880cfdb94d432f5a0ac355afbdb6))
- split in multiple files by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([07e0ab2](https://github.com/elcoosp/route-info-builder/commit/07e0ab2e28c58cf22ee9c6a76370f2fd08556923))
- extract return type by @elcoosp ([0adb62f](https://github.com/elcoosp/route-info-builder/commit/0adb62f5ff4ba8ef584b241fa27cb1b0976eaf34))
- generate type imports by @elcoosp ([2d37914](https://github.com/elcoosp/route-info-builder/commit/2d3791493ce3014b4f2296a820de2e386073e2dc))
- use api client & check auth by @elcoosp ([e17a696](https://github.com/elcoosp/route-info-builder/commit/e17a696654ab476143d851e575759499832d3134))
- correctly handle string in client by @elcoosp ([87b7e9d](https://github.com/elcoosp/route-info-builder/commit/87b7e9d88e1afb2952269b4c4522d9deb2e3ecca))
- use ts_quote with edition 2024 by @elcoosp ([6b1f286](https://github.com/elcoosp/route-info-builder/commit/6b1f28621bcec2174daadbc0670be39b3d1205e0))

### Miscellaneous
- logo by @elcoosp ([5cc5bb1](https://github.com/elcoosp/route-info-builder/commit/5cc5bb141bb74544e34a265df74dd1a43bd7f14b))
- add git-cliff changelog configuration by @elcoosp ([8d01001](https://github.com/elcoosp/route-info-builder/commit/8d01001828934bd7b0770807e867bfaa03e14417))
- ignore generated outputs and Rust backup files by @elcoosp ([80969ef](https://github.com/elcoosp/route-info-builder/commit/80969efd0314cb17c8265359e064ac4e08d309a9))
- add Dependabot configuration by @elcoosp ([96f88af](https://github.com/elcoosp/route-info-builder/commit/96f88af18a530a3a7083a034b6e7222b5a0039e2))
- readme by @elcoosp ([c98a5b9](https://github.com/elcoosp/route-info-builder/commit/c98a5b91c7f132147477faedb419eba7731d20f4))
- run clippy by @elcoosp in [#2](https://github.com/elcoosp/route-info-builder/pull/2) ([4cf8ad4](https://github.com/elcoosp/route-info-builder/commit/4cf8ad4fad43c6e8b326b177c034702d72d78e2f))
- init by @elcoosp ([8dec208](https://github.com/elcoosp/route-info-builder/commit/8dec20863d96dd8f654d3621e4400004c138bf0c))

### Refactor
- accept route iterators in TypeImportManager by @elcoosp ([b4d0f0e](https://github.com/elcoosp/route-info-builder/commit/b4d0f0e6de6482a54ce2a36ee62d8593fdc35368))
- resolve TypeScript display names in one stage by @elcoosp ([9f2edab](https://github.com/elcoosp/route-info-builder/commit/9f2edab3dc4c4abc1f73c571286cc49afae7f24f))
- build the link enum from pre-resolved names by @elcoosp ([e452393](https://github.com/elcoosp/route-info-builder/commit/e452393af6fafb10809cd14de8419a468b20873c))
- add shared utility module by @elcoosp ([b2bb17f](https://github.com/elcoosp/route-info-builder/commit/b2bb17f85223aa5dd290408d935212c097b31102))
- use syn visitors for handler metadata by @elcoosp ([1cf01a2](https://github.com/elcoosp/route-info-builder/commit/1cf01a28b92dfb6384a59c3c5e4aab37d7f75ea4))
- make TypeScript type imports configurable by @elcoosp ([2af653b](https://github.com/elcoosp/route-info-builder/commit/2af653b178992e66328d358391012ce7ff2c749b))
- simplify TanStack Query hook generation by @elcoosp ([63c3094](https://github.com/elcoosp/route-info-builder/commit/63c309469994a7f85ad82ef5062bb6ef3b15213d))
- simplify TypeScript client generation by @elcoosp ([7d7f87e](https://github.com/elcoosp/route-info-builder/commit/7d7f87e7b319fca249342d2e3802975fe4249a2a))
- split TypeScript generators into modules by @elcoosp ([78ddf13](https://github.com/elcoosp/route-info-builder/commit/78ddf137a2c411da6cf27303297cf6f974988665))
- rewrite Rust link enum generation by @elcoosp ([6a38596](https://github.com/elcoosp/route-info-builder/commit/6a385966e1edcb4b1248c3285d8c0ac626066a78))
- define shared CodeGenerator trait by @elcoosp ([5ede1ac](https://github.com/elcoosp/route-info-builder/commit/5ede1acc8c6318476ea1b65546846b8b1223f211))

### Testing
- pin client key, operationId and parity uniqueness by @elcoosp ([234fddc](https://github.com/elcoosp/route-info-builder/commit/234fddcfd22c598b01f3187f3d605ed4f82afef4))
- add end-to-end scanner and generator coverage by @elcoosp ([8643a91](https://github.com/elcoosp/route-info-builder/commit/8643a917dd3e699b044b7e6e2ca9cfd8effd48aa))
- cover the binary entry point by @elcoosp ([dac2fcd](https://github.com/elcoosp/route-info-builder/commit/dac2fcdc3e2d676a5fbaa782d309149f0c25dbc2))
- add working-directory guard helper by @elcoosp ([190d058](https://github.com/elcoosp/route-info-builder/commit/190d058b8ea3256a6f0b25dfb98c62f52236ff57))

### build
- bump crate to 0.2.0 and configure features and lints by @elcoosp ([c651ded](https://github.com/elcoosp/route-info-builder/commit/c651deddae714ea8c65416c51fba9570e6fe74bc))
- refresh lockfile for the 0.2.0 dependency set by @elcoosp ([8479f3a](https://github.com/elcoosp/route-info-builder/commit/8479f3abd2ca720b3c7905022bc379c0a4f9b76c))
**Full Changelog**: https://github.com/elcoosp/route-info-builder/releases/tag/v0.2.0

## Bug Fixes


- No screaming case (7f736a1)
- Handle push_operations correctly (e132229)
- Correct handling with build_full_path (23407b0)
- Generate_route_name fix + full url (a0b5c24)
- Fix duplicate imports (6742f7d)
- Do not import parsed Vec::<T>::new() response (079b786)
- Use is_builtin_type (b263d18)
- Check is_builtin_type_rust, may be importable (e50480f)
- Error imports incorrect path (d2bc64c)
- Return null if 204 not as T (9fba80c)
- *(deps)* [**breaking**] Update all dependencies and drop the broken ts_quote path dependency (d24f07c)

## Documentation


- Add readme (bccafd4)
- Rewrite the README against the real API and record the brainstorm (ddfbff2)

## Features


- Use ts_quote with edition 2024 (6b1f286)
- Correctly handle string in client (87b7e9d)
- Use api client & check auth (e17a696)
- Generate type imports (2d37914)
- Extract return type (0adb62f)
- Split in multiple files (07e0ab2)
- Api error (5f94d73)
- Primitive error return types detection (809c0b1)
- Split api (hooks) & client (3fab2b5)
- Handle query params extract & add to client & hooks (b4e999a)
- *(parser)* [**breaking**] Recursive Axum/Loco route scanning with canonical paths (c30d4c9)
- *(config)* [**breaking**] TOML configuration files, typed errors and collision strategies (376a595)
- *(generators)* [**breaking**] Single-path code generation, OpenAPI skeletons and route maps (65b325a)
- *(cli)* Add route-info-builder binary with generate, scan and init (fb7bc65)

## Miscellaneous Tasks


- Init (8dec208)
- Run clippy (4cf8ad4)
- Readme (c98a5b9)
- GitHub Actions pipelines, git-cliff changelog and dependabot (f92049f)

## Tests


- Cover the crate to 100% line coverage (6aa35fc)
<!-- generated by git-cliff -->
