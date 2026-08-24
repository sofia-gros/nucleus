# Architecture & Coding Guidelines for AI

## Core Design Principles
- **Vertical Slice Architecture**: Group files by feature/domain (e.g., `src/features/auth/`), NOT by layer (e.g., `controllers/`, `services/`, `models/`).
- **High Colocation**: Keep types, database access, business logic, and UI components close together within the same feature directory or file when possible.
- **Explicit & Simple over Abstract**:
  - Prefer flat, simple, and self-contained code.
  - Avoid unnecessary abstractions, Clean Architecture layers, DI containers, or overly complex generic traits/interfaces unless explicitly requested.
  - Duplication is better than incorrect or premature abstraction.
- **Strict Typing & Single Source of Truth**:
  - Always rely on strict type systems and schema definitions.
  - Ensure the compiler/type-checker can instantly catch breakage.

## GPUI Specific Rules
- **Component Usage**: Always use `gpui-component`. Do not reinvent components.
- **Documentation**: Do not read source code unnecessarily. Always refer to `https://docs.rs/<crate_name>/latest/`.
- **Design Base**: Use shadcn design initially. Adjust to near-future design only after functionality is complete.
- **Memory Management**: Strictly adhere to Rust's ownership model. Do not use unnecessary clones.

## Coding Style & Constraints
1. **Modularity**: When creating new features, place all relevant logic inside a dedicated feature directory (`src/features/<feature-name>/`).
2. **File Scope**: Keep individual files focused. If a feature grows, break it into smaller sub-modules within the same feature folder.
3. **Refactoring**: When changing a feature, modify files only within that feature's scope whenever possible. Do not create global utilities unless the code is used across 3+ distinct features.
4. **Error Handling**: Fail fast with clear type constraints rather than complex error-handling middleware.
