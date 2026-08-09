## [0.2.0](https://github.com/terror/kotomori/releases/tag/0.2.0) - 2026-08-09

### Added

- Add composer prompt history ([#184](https://github.com/terror/kotomori/pull/184) by [terror](https://github.com/terror))

### Fixed

- Avoid leading transcript blank lines ([#188](https://github.com/terror/kotomori/pull/188) by [terror](https://github.com/terror))
- Keep unknown commands out of model context ([#187](https://github.com/terror/kotomori/pull/187) by [terror](https://github.com/terror))
- Fix clear racing active agent ([#186](https://github.com/terror/kotomori/pull/186) by [terror](https://github.com/terror))

### Misc

- Move `assert_matches` into crate root ([#194](https://github.com/terror/kotomori/pull/194) by [terror](https://github.com/terror))
- Refactor tool invocation architecture ([#193](https://github.com/terror/kotomori/pull/193) by [terror](https://github.com/terror))
- Tag persisted tool calls ([#192](https://github.com/terror/kotomori/pull/192) by [terror](https://github.com/terror))
- Move output decoding into executor ([#191](https://github.com/terror/kotomori/pull/191) by [terror](https://github.com/terror))
- Collapse presented frame into frame ([#190](https://github.com/terror/kotomori/pull/190) by [terror](https://github.com/terror))
- Separate composer from command menu ([#189](https://github.com/terror/kotomori/pull/189) by [terror](https://github.com/terror))
- Move selection handling into composer ([#183](https://github.com/terror/kotomori/pull/183) by [terror](https://github.com/terror))
- Inline request construction ([#182](https://github.com/terror/kotomori/pull/182) by [terror](https://github.com/terror))
- Use `Default` for provider sink ([#181](https://github.com/terror/kotomori/pull/181) by [terror](https://github.com/terror))
- Inline tool invocation helper ([#180](https://github.com/terror/kotomori/pull/180) by [terror](https://github.com/terror))

# Changelog

## [0.1.1](https://github.com/terror/kotomori/releases/tag/0.1.1) - 2026-08-08

### Added

- Add resume `--last` flag ([#156](https://github.com/terror/kotomori/pull/156) by [terror](https://github.com/terror))
- Store session data in SQLite ([#155](https://github.com/terror/kotomori/pull/155) by [terror](https://github.com/terror))

### Fixed

- Run command tool through shell ([#168](https://github.com/terror/kotomori/pull/168) by [terror](https://github.com/terror))
- Fix stderr result semantics ([#165](https://github.com/terror/kotomori/pull/165) by [terror](https://github.com/terror))
- Limit agent tool loop ([#164](https://github.com/terror/kotomori/pull/164) by [terror](https://github.com/terror))
- Enforce tool approval in agent ([#162](https://github.com/terror/kotomori/pull/162) by [terror](https://github.com/terror))
- Isolate agent events by run identifier ([#158](https://github.com/terror/kotomori/pull/158) by [terror](https://github.com/terror))

### Misc

- Update demo image in readme ([#178](https://github.com/terror/kotomori/pull/178) by [terror](https://github.com/terror))
- Add spacing before agent activity ([#177](https://github.com/terror/kotomori/pull/177) by [terror](https://github.com/terror))
- Add horizontal component padding ([#176](https://github.com/terror/kotomori/pull/176) by [terror](https://github.com/terror))
- Add spacing above footer ([#175](https://github.com/terror/kotomori/pull/175) by [terror](https://github.com/terror))
- Replace message rails with left gutter ([#174](https://github.com/terror/kotomori/pull/174) by [terror](https://github.com/terror))
- Standardize key labels ([#173](https://github.com/terror/kotomori/pull/173) by [terror](https://github.com/terror))
- Simplify approval choices ([#172](https://github.com/terror/kotomori/pull/172) by [terror](https://github.com/terror))
- Use semantic style roles ([#171](https://github.com/terror/kotomori/pull/171) by [terror](https://github.com/terror))
- Remove `ToolResult` message helper ([#170](https://github.com/terror/kotomori/pull/170) by [terror](https://github.com/terror))
- Remove `ToolResult` constructors ([#169](https://github.com/terror/kotomori/pull/169) by [terror](https://github.com/terror))
- Centralize transcript spacing ([#167](https://github.com/terror/kotomori/pull/167) by [terror](https://github.com/terror))
- Extract resume picker component ([#166](https://github.com/terror/kotomori/pull/166) by [terror](https://github.com/terror))
- Remove tool invocation indirection ([#163](https://github.com/terror/kotomori/pull/163) by [terror](https://github.com/terror))
- Inject executor from agent ([#161](https://github.com/terror/kotomori/pull/161) by [terror](https://github.com/terror))
- Enforce consistent field ordering ([#160](https://github.com/terror/kotomori/pull/160) by [terror](https://github.com/terror))
- Add `assert_matches` test helper ([#159](https://github.com/terror/kotomori/pull/159) by [terror](https://github.com/terror))
- Fix Japanese translation in readme ([#157](https://github.com/terror/kotomori/pull/157) by [casey](https://github.com/casey))
- Run integration tests on Windows ([#153](https://github.com/terror/kotomori/pull/153) by [terror](https://github.com/terror))
- Test resume picker cancellation ([#152](https://github.com/terror/kotomori/pull/152) by [terror](https://github.com/terror))
- Test resume with no saved sessions ([#151](https://github.com/terror/kotomori/pull/151) by [terror](https://github.com/terror))
- Test two-stage Ctrl-C behavior ([#150](https://github.com/terror/kotomori/pull/150) by [terror](https://github.com/terror))
- Test resume restores persisted model ([#149](https://github.com/terror/kotomori/pull/149) by [terror](https://github.com/terror))
- Test session tool and interruption round trips ([#148](https://github.com/terror/kotomori/pull/148) by [terror](https://github.com/terror))
- Test agent provider error recovery ([#147](https://github.com/terror/kotomori/pull/147) by [terror](https://github.com/terror))
- Add integration test step context ([#145](https://github.com/terror/kotomori/pull/145) by [terror](https://github.com/terror))
- Return errors for unexpected test exits ([#144](https://github.com/terror/kotomori/pull/144) by [terror](https://github.com/terror))
- Make mock provider instant by default ([#143](https://github.com/terror/kotomori/pull/143) by [terror](https://github.com/terror))
- Avoid graceful quit in integration tests ([#142](https://github.com/terror/kotomori/pull/142) by [terror](https://github.com/terror))
- Parallelize integration tests ([#141](https://github.com/terror/kotomori/pull/141) by [terror](https://github.com/terror))
- Refactor agent tests ([#140](https://github.com/terror/kotomori/pull/140) by [terror](https://github.com/terror))
- Consolidate event channel ([#139](https://github.com/terror/kotomori/pull/139) by [terror](https://github.com/terror))
- Factor out trailing blank line check ([#138](https://github.com/terror/kotomori/pull/138) by [terror](https://github.com/terror))
- Inline frame helper ([#137](https://github.com/terror/kotomori/pull/137) by [terror](https://github.com/terror))
- Move patch calculations into render plan ([#136](https://github.com/terror/kotomori/pull/136) by [terror](https://github.com/terror))
- Collapse row movement helpers ([#135](https://github.com/terror/kotomori/pull/135) by [terror](https://github.com/terror))
- Derive viewport state ([#134](https://github.com/terror/kotomori/pull/134) by [terror](https://github.com/terror))
- Simplify viewport initialization ([#133](https://github.com/terror/kotomori/pull/133) by [terror](https://github.com/terror))
- Inline full renderer ([#132](https://github.com/terror/kotomori/pull/132) by [terror](https://github.com/terror))
- Remove unreachable viewport catch-up ([#131](https://github.com/terror/kotomori/pull/131) by [terror](https://github.com/terror))
- Centralize full render viewport state ([#130](https://github.com/terror/kotomori/pull/130) by [terror](https://github.com/terror))
- Centralize logical row movement ([#129](https://github.com/terror/kotomori/pull/129) by [terror](https://github.com/terror))
- Simplify render planning ([#128](https://github.com/terror/kotomori/pull/128) by [terror](https://github.com/terror))
- Remove unused message component ([#127](https://github.com/terror/kotomori/pull/127) by [terror](https://github.com/terror))
- Consolidate line replacement ([#126](https://github.com/terror/kotomori/pull/126) by [terror](https://github.com/terror))
- Make `draw_frame` own render transactions ([#125](https://github.com/terror/kotomori/pull/125) by [terror](https://github.com/terror))
- Collapse `Diff` into `ChangedRange` ([#124](https://github.com/terror/kotomori/pull/124) by [terror](https://github.com/terror))
- Store only viewport top ([#123](https://github.com/terror/kotomori/pull/123) by [terror](https://github.com/terror))
- Simplify renderer cursor state ([#122](https://github.com/terror/kotomori/pull/122) by [terror](https://github.com/terror))
- Pass frames by ownership consistently ([#121](https://github.com/terror/kotomori/pull/121) by [terror](https://github.com/terror))
- Simplify cursor tracking ([#120](https://github.com/terror/kotomori/pull/120) by [terror](https://github.com/terror))
- Unify tail deletion with patch rendering ([#119](https://github.com/terror/kotomori/pull/119) by [terror](https://github.com/terror))
- Move concrete renderer impl before generic impl ([#118](https://github.com/terror/kotomori/pull/118) by [terror](https://github.com/terror))
- Move terminal ownership into renderer ([#117](https://github.com/terror/kotomori/pull/117) by [terror](https://github.com/terror))
- Collapse resize checks ([#116](https://github.com/terror/kotomori/pull/116) by [terror](https://github.com/terror))
- Simplify writable line iteration ([#115](https://github.com/terror/kotomori/pull/115) by [terror](https://github.com/terror))
- Remove redundant viewport arguments ([#114](https://github.com/terror/kotomori/pull/114) by [terror](https://github.com/terror))
- Remove redundant append condition ([#113](https://github.com/terror/kotomori/pull/113) by [terror](https://github.com/terror))
- Make line feed update cursor atomically ([#112](https://github.com/terror/kotomori/pull/112) by [terror](https://github.com/terror))
- Return early for unchanged frames ([#111](https://github.com/terror/kotomori/pull/111) by [terror](https://github.com/terror))
- Collapse `RenderPlanner` into `RenderPlan` ([#110](https://github.com/terror/kotomori/pull/110) by [terror](https://github.com/terror))
- Remove previous viewport ([#109](https://github.com/terror/kotomori/pull/109) by [terror](https://github.com/terror))
- Remove max lines rendered state ([#108](https://github.com/terror/kotomori/pull/108) by [terror](https://github.com/terror))
- Simplify render plans ([#107](https://github.com/terror/kotomori/pull/107) by [terror](https://github.com/terror))
- Remove `PatchPlan` enum ([#106](https://github.com/terror/kotomori/pull/106) by [terror](https://github.com/terror))
- Profile time to first draw ([#105](https://github.com/terror/kotomori/pull/105) by [terror](https://github.com/terror))

## [0.1.0](https://github.com/terror/kotomori/releases/tag/0.1.0) - 2026-08-06

### Added

- Stream conversations with reasoning and tool calls in a responsive terminal interface
- Run shell commands with explicit approval for privileged operations
- Persist sessions and resume previous conversations
- Configure models from a broad range of hosted and local providers
- Load project instructions and working directory context automatically
- Render incremental updates efficiently with frame-based differential output
