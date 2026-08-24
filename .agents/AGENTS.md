# Architecture & Coding Guidelines for AI

## Core Design Principles
- **Domain-Driven Vertical Slice**: Group files by core IDE domains rather than generic layers. The top-level hierarchy must strictly align with the plugin system architecture:
  - `src/workspace/`: Contains `activity_bar`, `left_sidebar`, `editor_area`, `right_sidebar`, `bottom_panel`.
  - `src/editor/`: Core editor logic (independent of the workspace UI).
  - `src/file_system/`: File system abstractions.
  - `src/terminal/`: Terminal integration.
  - `src/process/`: External process execution.
  - `src/lsp/`: Language Server Protocol integration.
  - `src/plugin_manager/`: WASM runtime, manifest parsing, and plugin lifecycle.
- **High Colocation**: Keep types, business logic, and UI components close together within their respective domain directories.
- **Explicit & Simple over Abstract**:
  - Prefer flat, simple, and self-contained code.
  - Avoid unnecessary abstractions, Clean Architecture layers, or DI containers.
  - Duplication is better than incorrect or premature abstraction.
- **Strict Typing & Single Source of Truth**:
  - Always rely on strict type systems and schema definitions.
  - Ensure the compiler/type-checker can instantly catch breakage.
- **Host/Plugin Boundary Strictness**:
  - Internal Rust/GPUI types must NEVER leak to the Plugin ABI.
  - The `plugin_manager` must act as the sole bridge between WASM plugins and the rest of the IDE.

## Zed-Level Performance Constraints
To achieve Zed's speed and responsiveness, the following strict performance rules apply:
1. **Zero UI Thread Blocking**: ネットワーク通信、ファイルI/O、LSP解析、プラグインのWASM実行などの重い処理は、絶対にメインのUIスレッドで実行してはいけません。必ずGPUIの非同期タスク (`cx.spawn(...)`) またはバックグラウンドタスクにオフロードします。
2. **Zero-Cost State Sharing (Cloneの排除)**: `render` 関数内や状態の受け渡しにおいて、安易な `.clone()` を禁止します。巨大なデータ構造は `Arc<T>` や参照 (`&T`) を用いて、ポインタのコピーのみで完結させます。
3. **Granular Re-rendering (局所的な更新)**: グローバルな状態をむやみに更新してUI全体を再描画させないこと。変更があった具体的な `Model` や `View` に対してのみ `cx.notify()` を呼び出し、再描画の範囲を最小限に抑えます。
4. **Efficient Data Structures**: エディタのテキストバッファなどには `String` を直接使わず、Ropeデータ構造や効率的なメモリ割り当てを使用します。
5. **Flat UI Hierarchy**: 不要なDOMノードの深いネスト（無駄な `div()` やフレックスボックスの過剰な階層）を避け、レイアウト計算のオーバーヘッドを最小化します。

## GPUI Specific Rules
- **Component Usage**: Always use `gpui-component`. Do not reinvent components.
- **Documentation**: Do not read source code unnecessarily. Always refer to `https://docs.rs/<crate_name>/latest/`.
- **Design Base**: Use shadcn design initially. Adjust to near-future design only after functionality is complete.
- **Memory Management**: Strictly adhere to Rust's ownership model. Do not use unnecessary clones.

## Coding Style & Constraints
1. **Modularity**: When creating new features, place them in the correct domain directory (e.g., `src/workspace/activity_bar/`).
2. **File Scope**: Keep individual files focused. Break them into smaller sub-modules within the same domain folder if they grow large.
3. **Refactoring**: Modify files only within the specific domain's scope whenever possible. Do not create global utilities unless the code is used across 3+ distinct domains.
4. **Error Handling**: Fail fast with clear type constraints. Plugins must not crash the host (trap handling required).
