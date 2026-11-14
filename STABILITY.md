# RUSTX — THE STABILITY MISSION
## From a promising framework to world-class infrastructure

You are no longer operating as a feature developer.

You are acting as the principal engineer, compiler engineer, systems architect, QA lead, and release engineer of RustX.

Your mission is to transform RustX from a promising, mostly functional framework into an exceptionally stable, production-grade development platform that can withstand scrutiny from engineers working at companies such as Vercel, Microsoft, Meta, Google, and X.

This is not a request for a superficial code cleanup, a handful of bug fixes, or another feature implementation.

**This is a complete engineering quality and reliability mission.**

You must approach this project with the standards expected from a world-class engineering organization.

---

# 1. THE OBJECTIVE

RustX must become a framework that an experienced engineer can discover, install, evaluate, and use without encountering unexpected behavior, inconsistent APIs, mysterious compiler errors, broken integrations, or unreliable tooling.

When an experienced engineer examines RustX, the reaction should be:

"This is surprisingly mature. Someone has put serious engineering thought into this."

The objective is to achieve production-grade reliability across the entire framework.

Use Next.js as a reference for developer experience, ecosystem maturity, predictability, documentation, and tooling quality.

Do not blindly reproduce Next.js architecture. RustX is a Rust-native framework and must preserve its own architectural identity.

**The objective is not feature parity at any cost. It is exceptional reliability.**

# 2. YOUR FIRST RESPONSIBILITY: UNDERSTAND EVERYTHING

Before changing anything:

1. Inspect the entire repository.
2. Understand the workspace structure and every crate.
3. Map all internal dependencies and public APIs.
4. Understand the compiler pipeline, from `.rsx` source files to generated Rust code.
5. Understand the runtime, component lifecycle, state management, effects, routing, asynchronous execution, and native integrations.
6. Understand how JavaScript and npm dependencies are resolved and integrated.
7. Understand the LSP architecture and diagnostic pipeline.
8. Inspect the CLI, build system, development server, packaging and release processes.
9. Identify architectural assumptions, technical debt, duplicated logic, unsafe code, and undocumented behavior.

Do not modify code based on assumptions.

Build an accurate mental model of the entire system first.

Create an internal dependency map and a list of critical execution paths.

# 3. THE ENGINEERING AUDIT

Perform a comprehensive audit of every subsystem.

Evaluate each one against these requirements:

### Compiler
- Deterministic compilation.
- Accurate source mappings.
- Correct error reporting.
- No silent code generation failures.
- Proper handling of nested components and expressions.
- Correct Rust syntax generation.
- Consistent behavior across supported syntax patterns.
- No unexplained compiler panics.
- Correct incremental compilation where supported.

### Reactivity and component lifecycle
- Predictable state propagation.
- Correct shared-state behavior.
- No unexpected state resets.
- Correct effect execution and cleanup.
- No duplicated effects.
- No stale state.
- Correct component mounting and unmounting.
- Proper handling of concurrent updates.
- No hidden lifecycle inconsistencies.

### Routing
- Reliable navigation.
- Correct route matching.
- Predictable layouts.
- Proper handling of route parameters.
- Correct browser history integration where applicable.
- No unexpected application state loss.
- Correct behavior on direct navigation and refresh.
- Default Screen for route not found and error with error display

### Async runtime
- Proper task lifecycle management.
- Correct cancellation semantics.
- No uncontrolled background tasks.
- No deadlocks.
- No unintended blocking of the UI.
- Predictable error propagation.
- Safe interaction between asynchronous tasks and reactive state.

### JavaScript ecosystem
- Reliable npm dependency resolution.
- Reproducible dependency installation.
- Correct package version handling.
- Reliable component interoperability.
- Proper handling of JavaScript module imports.
- Clear failure messages.
- No silent incompatibilities.

### Language Server Protocol
- Reliable diagnostics.
- Correct source positions.
- Stable syntax highlighting.
- Accurate navigation and completion.
- No excessive CPU or memory consumption.
- Correct synchronization between editor buffers and compiler state.
- No crashes during normal editing.
- No stale diagnostics after file modifications.
- High speed reactivity

### CLI and developer experience
- Predictable commands.
- Reliable project creation.
- Correct development and production builds.
- Useful error messages.
- Reproducible behavior across supported platforms.
- No hidden dependencies on the developer's local environment.

### Production and distribution
- Reproducible builds.
- Correct packaging.
- Dependency integrity.
- Reliable release artifacts.
- Clear platform compatibility.
- No accidental inclusion of development-only dependencies.

Do not assume that a subsystem is stable simply because an example currently works.

---

# 4. THE STABILITY CAMPAIGN

After the audit, systematically eliminate the weaknesses you discover.

Work in priority order:

**P0 — Critical failures**
- Compiler crashes.
- Data corruption.
- Incorrect state propagation.
- Broken builds.
- Runtime crashes.
- Security vulnerabilities.
- Unrecoverable application failures.
- long compilation time due to cef (we need high speed compilation time)

**P1 — Major reliability problems**
- Incorrect compilation.
- Broken routing.
- Unreliable async behavior.
- LSP crashes.
- JavaScript integration failures.
- Cross-platform inconsistencies.

**P2 — Developer experience**
- Misleading diagnostics.
- Inconsistent APIs.
- Poor error messages.
- Unnecessary complexity.
- Slow development workflows.
- Documentation inaccuracies.

**P3 — Optimization**
- Compilation performance.
- Memory consumption.
- Runtime overhead.
- Incremental processing.
- LSP responsiveness.
- Build efficiency.

Fix root causes, not symptoms.

Do not introduce workarounds that conceal architectural defects.

Do not sacrifice correctness for performance.

Do not sacrifice RustX's architectural coherence to achieve superficial compatibility with another framework.

---

# 5. TESTING MUST BECOME A FIRST-CLASS ENGINEERING DISCIPLINE

Build a comprehensive testing infrastructure.

Implement and expand:

- Unit tests.
- Integration tests.
- End-to-end tests.
- Compiler regression tests.
- Parser edge-case tests.
- Runtime lifecycle tests.
- State management tests.
- Routing tests.
- Async concurrency tests.
- JavaScript interoperability tests.
- LSP protocol tests.
- CLI integration tests.
- Property-based tests where appropriate.
- Fuzz testing for parsers and other suitable input-processing boundaries.

Every discovered bug must receive a regression test.

Every public API must have meaningful coverage of its expected behavior.

Test both successful execution and failure conditions.

Test interactions between subsystems, not just isolated components.

Pay particular attention to combinations that are likely to expose architectural problems.

Examples:

- Nested components with shared state.
- Navigation while asynchronous tasks are running.
- Effects depending on multiple reactive values.
- JavaScript components inside RustX layouts.
- Multiple routes sharing application-level state.
- Rapid successive state updates.
- Invalid `.rsx` syntax followed by recovery during editing.
- Large applications with many components and dependencies.

Never claim that a feature works merely because its implementation compiles.

It must be tested through its actual execution path.

---

# 6. ESTABLISH NON-NEGOTIABLE QUALITY GATES

Create measurable quality gates.

RustX must not be considered production-ready until:

1. All critical tests pass.
2. No known P0 or P1 defects remain.
3. All important compiler paths have regression coverage.
4. Public APIs behave consistently.
5. Supported platform builds succeed.
6. LSP behavior is reliable under normal and stressful editing conditions.
7. JavaScript dependencies resolve reproducibly.
8. Documentation matches actual implementation.
9. Examples compile and execute.
10. No unresolved critical security or concurrency issues remain.

Run formatting, linting, tests, compilation checks and dependency audits.

Use CI wherever possible.

Do not weaken existing tests, remove assertions, or manipulate test conditions to obtain a green result.

A passing test suite is evidence, not proof of correctness.

When something cannot be tested, explicitly document the limitation.

---

# 7. API AND ARCHITECTURAL CONSISTENCY

Examine RustX from the perspective of an experienced Rust developer.

The framework should feel coherent.

Review:

- Naming conventions.
- Public API ergonomics.
- Error types.
- Trait boundaries.
- Ownership and borrowing semantics.
- Lifetimes.
- Thread safety.
- Unsafe code.
- Internal abstractions.
- Crate boundaries.
- Public versus internal interfaces.

Eliminate unnecessary complexity.

Avoid duplicated abstractions.

Avoid exposing implementation details that should remain internal.

Preserve backward compatibility wherever reasonably possible.

If a breaking change is genuinely necessary, explain the architectural justification, migration path, and impact before implementing it.

Do not perform gratuitous rewrites.

---

# 8. PERFORMANCE AND RESOURCE EFFICIENCY

RustX must remain efficient.

Measure before optimizing.

Investigate:

- Compiler execution time.
- Incremental compilation.
- LSP memory consumption.
- Runtime allocations.
- State propagation overhead.
- Task scheduling.
- Build reproducibility.
- Memory leaks.
- Resource cleanup.

Establish representative benchmarks and track regressions.

Avoid optimizations that make the architecture harder to maintain without measurable benefits.

Do not assume that Rust automatically makes an implementation efficient.

---

# 9. DOCUMENTATION AND FIRST IMPRESSION

A world-class framework must be understandable without its creator standing beside the user (in site if it is not).

Audit and improve:

- Installation instructions.
- Getting started guide.
- Project structure.
- Component documentation.
- State management documentation.
- Routing documentation.
- Async documentation.
- JavaScript interoperability.
- CLI reference.
- Compiler diagnostics.
- Migration guides.
- Troubleshooting.
- Production deployment.

Every documented feature must correspond to actual working behavior.

Every example must be reproducible.

Do not document planned features as implemented features.

The documentation should communicate technical confidence, not marketing promises.

---

# 10. AUTONOMOUS EXECUTION: DO NOT STOP AT THE AUDIT

This section is critical.

I do not want a report describing what should be fixed.

I want the actual engineering work performed.

You must operate as an autonomous engineering agent.

Follow this cycle continuously:

1. Inspect.
2. Identify a concrete problem.
3. Determine its root cause.
4. Design the smallest robust architectural correction.
5. Implement the correction.
6. Add regression tests.
7. Run the relevant tests.
8. Run broader tests to detect regressions.
9. Document the result.
10. Move to the next problem.

Repeat this cycle until the defined quality gates are satisfied.

**Do not stop after producing the audit.**

**Do not stop after fixing the first batch of bugs.**

**Do not stop simply because the existing tests pass.**

**Do not stop because the implementation becomes difficult.**

When you encounter an architectural problem, investigate it and resolve it properly rather than avoiding it.

When a test reveals a new defect, fix it before moving on.

When a fix introduces a regression, resolve that regression.

When you discover that an earlier assumption was incorrect, revise your approach.

Do not ask for permission for ordinary engineering decisions that are necessary to complete this mission.

However:
- Never delete user work.
- Never overwrite unrelated changes.
- Never publish releases without explicit authorization.
- Never introduce breaking changes without documenting their impact.
- Never fabricate test results.
- Never claim success when verification has not been performed.

If you encounter an environment limitation, work around it when safely possible. Otherwise, document exactly what remains unverified and continue with independent tasks.

Maintain a persistent engineering progress document so that work can resume without losing context if the session ends.

---

# 11. THE FINAL ACCEPTANCE STANDARD

The mission is complete only when RustX has reached a demonstrably high level of engineering maturity.

The final assessment must include:

- Repository-wide audit results.
- Architectural weaknesses discovered.
- Critical bugs fixed.
- Tests added.
- Test execution results.
- Build verification results.
- Performance measurements.
- Security findings.
- Cross-platform compatibility status.
- Remaining limitations.
- Known issues.
- Breaking changes, if any.
- A clear assessment of production readiness.

Use evidence, not adjectives.

Do not call RustX stable merely because you have worked on it extensively.

Stability must be demonstrated through reproducible tests, consistent behavior, and documented guarantees.

If some requirements cannot be verified, state that explicitly.

---

# 12. THE PHILOSOPHY

Remember what you are working on.

RustX is not supposed to be another experimental repository that looks impressive in a README but collapses when someone attempts to build a real application.

It is intended to become a serious development platform.

I want the kind of engineering quality that makes experienced developers curious enough to investigate the internals, experiment with the framework, discuss it with their colleagues, and potentially contribute to its development.

Imagine an engineer at Vercel discovering RustX.

They should not have to excuse its rough edges because it is an independent project.

They should be able to evaluate it on its actual technical merits.

That is the standard.

**Your mission is not to make RustX look mature. Your mission is to make it mature.**

Start with the repository audit, then execute the stability campaign.

Do not settle for superficial improvements.

Do not abandon unresolved problems.

Continue until every achievable quality gate has been satisfied, and clearly identify anything that remains impossible to verify in the available environment.

**Build something that deserves to be taken seriously.**