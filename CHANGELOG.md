# Changelog

## [0.1.1](https://github.com/terror/kotomori/releases/tag/0.1.1) - 2026-08-08

### Added

- Add resume `--last` flag (#156)
- Store session data in SQLite (#155)

### Fixed

- Run command tool through shell (#168)
- Fix stderr result semantics (#165)
- Limit agent tool loop (#164)
- Enforce tool approval in agent (#162)
- Isolate agent events by run identifier (#158)

### Misc

- Update demo image in readme (#178)
- Add spacing before agent activity (#177)
- Add horizontal component padding (#176)
- Add spacing above footer (#175)
- Replace message rails with left gutter (#174)
- Standardize key labels (#173)
- Simplify approval choices (#172)
- Use semantic style roles (#171)
- Remove `ToolResult` message helper (#170)
- Remove `ToolResult` constructors (#169)
- Centralize transcript spacing (#167)
- Extract resume picker component (#166)
- Remove tool invocation indirection (#163)
- Inject executor from agent (#161)
- Enforce consistent field ordering (#160)
- Add `assert_matches` test helper (#159)
- Fix Japanese translation in readme (#157)
- Run integration tests on Windows (#153)
- Test resume picker cancellation (#152)
- Test resume with no saved sessions (#151)
- Test two-stage Ctrl-C behavior (#150)
- Test resume restores persisted model (#149)
- Test session tool and interruption round trips (#148)
- Test agent provider error recovery (#147)
- Add integration test step context (#145)
- Return errors for unexpected test exits (#144)
- Make mock provider instant by default (#143)
- Avoid graceful quit in integration tests (#142)
- Parallelize integration tests (#141)
- Refactor agent tests (#140)
- Consolidate event channel (#139)
- Factor out trailing blank line check (#138)
- Inline frame helper (#137)
- Move patch calculations into render plan (#136)
- Collapse row movement helpers (#135)
- Derive viewport state (#134)
- Simplify viewport initialization (#133)
- Inline full renderer (#132)
- Remove unreachable viewport catch-up (#131)
- Centralize full render viewport state (#130)
- Centralize logical row movement (#129)
- Simplify render planning (#128)
- Remove unused message component (#127)
- Consolidate line replacement (#126)
- Make `draw_frame` own render transactions (#125)
- Collapse `Diff` into `ChangedRange` (#124)
- Store only viewport top (#123)
- Simplify renderer cursor state (#122)
- Pass frames by ownership consistently (#121)
- Simplify cursor tracking (#120)
- Unify tail deletion with patch rendering (#119)
- Move concrete renderer impl before generic impl (#118)
- Move terminal ownership into renderer (#117)
- Collapse resize checks (#116)
- Simplify writable line iteration (#115)
- Remove redundant viewport arguments (#114)
- Remove redundant append condition (#113)
- Make line feed update cursor atomically (#112)
- Return early for unchanged frames (#111)
- Collapse `RenderPlanner` into `RenderPlan` (#110)
- Remove previous viewport (#109)
- Remove max lines rendered state (#108)
- Simplify render plans (#107)
- Remove `PatchPlan` enum (#106)
- Profile time to first draw (#105)

## [0.1.0](https://github.com/terror/kotomori/releases/tag/0.1.0) - 2026-08-06

### Added

- Stream conversations with reasoning and tool calls in a responsive terminal interface
- Run shell commands with explicit approval for privileged operations
- Persist sessions and resume previous conversations
- Configure models from a broad range of hosted and local providers
- Load project instructions and working directory context automatically
- Render incremental updates efficiently with frame-based differential output
