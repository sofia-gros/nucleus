## Nucleus core strings (English, required).
## Keys are written with "-" in Fluent; code uses "." (core.menu.file → core-menu-file).

core-app-name = Nucleus
core-window-title = { $file } — { $project }
core-window-title-empty = Nucleus

## Menus
core-menu-file = File
core-menu-edit = Edit
core-menu-view = View
core-menu-selection = Selection
core-menu-find = Find
core-menu-extensions = Extensions
core-menu-help = Help

## Docks and panels
core-panel-project = Project
core-panel-search = Search
core-panel-git = Git
core-panel-outline = Outline
core-panel-terminal = Terminal
core-panel-problems = Problems
core-panel-output = Output
core-dock-toggle-left = Toggle Left Dock
core-dock-toggle-bottom = Toggle Bottom Dock
core-dock-toggle-right = Toggle Right Dock

## Project panel
core-project-empty-title = No project open
core-project-empty-body = Open a folder to browse its files here.
core-project-open-folder = Open Folder

## Welcome
core-welcome-tab = Welcome
core-welcome-tagline = A hackable, blazing fast editor — built with Rust & gpui-kit
core-welcome-start = Start
core-welcome-recent = Recent Projects
core-welcome-open-folder = Open Folder…
core-welcome-new-file = New File
core-welcome-clone = Clone Repository…
core-welcome-command-palette = Command Palette
core-welcome-no-recent = No recent projects yet
core-welcome-show-on-startup = Show welcome page on startup
core-welcome-tip-find-file = Find file
core-welcome-tip-search = Search project
core-welcome-tip-terminal = Toggle terminal
core-welcome-tip-settings = Settings

## Status bar
core-status-cursor = { $line }:{ $column }
core-status-errors = { $count }
core-status-warnings = { $count }
core-status-extensions-loading =
    { $count ->
        [one] Loading { $count } extension…
       *[other] Loading { $count } extensions…
    }

## Placeholders for panels that are not implemented yet
core-panel-coming-soon = { $panel } is not available yet.

## Project tree and files
core-project-loading = Loading…
core-project-read-error = Could not read this folder: { $error }
core-dialog-open-folder = Open Folder
core-notify-open-failed = Could not open { $file }: { $error }
core-notify-binary = { $file } looks like a binary file and was not opened.
core-notify-too-large = { $file } is too large to open.
core-notify-save-failed = Could not save { $file }: { $error }
core-status-language-text = Plain Text

## Commands
core-command-find-file = Go to File
core-command-open-folder = Open Folder…
core-command-save = Save
core-command-find-in-file = Find in File
core-command-close-tab = Close Tab
core-command-toggle-left-dock = Toggle Left Dock
core-command-toggle-bottom-dock = Toggle Bottom Dock
core-command-toggle-right-dock = Toggle Right Dock
core-command-toggle-theme = Theme: Toggle Light / Dark
core-command-command-palette = Show All Commands
core-command-quit = Quit

## Palette
core-palette-commands-placeholder = Type a command…
core-palette-files-placeholder = Search files by name…
core-palette-no-results = No matching results
core-palette-no-project = Open a folder to search its files.
core-palette-indexing = Indexing files…

## Unsaved changes
core-prompt-unsaved-one = Do you want to save the changes you made to { $file }?
core-prompt-unsaved-many = You have unsaved changes in { $count } files. Save them before closing?
core-prompt-unsaved-detail = Your changes will be lost if you don't save them.
core-prompt-save = Save
core-prompt-save-all = Save All
core-prompt-dont-save = Don't Save
core-prompt-cancel = Cancel

## Settings
core-command-open-settings = Preferences: Open Settings
core-command-open-settings-json = Preferences: Open Settings (JSON)
core-command-open-workspace-settings = Preferences: Open Folder Settings (JSON)
core-notify-settings-error = Settings were not applied: { $error }
core-notify-keymap-restart = Keymap changes take effect after restarting Nucleus.

## Find & replace (editor)
core-find-title = Find in Current Buffer
core-find-hint = Type to search. Enter: next, Shift+Enter: previous, Esc: close
core-find-results = { $current } of { $count } for “{ $query }”
core-find-no-results = No results for “{ $query }”
core-find-find-placeholder = Find in current buffer
core-find-replace-placeholder = Replace in current buffer
core-find-find = Find
core-find-replace = Replace
core-find-replace-all = Replace All
core-find-case-sensitive = Match case

## Terminal
core-terminal-exited = Process exited. Press Enter to restart.
core-terminal-spawn-failed = Could not start the terminal: { $error }
core-command-new-terminal = Terminal: New Terminal
core-command-toggle-terminal = Terminal: Toggle Terminal

## Extensions, themes and languages
core-status-extensions-starting = Loading extensions…
core-notify-extension-failed = An extension could not be loaded: { $error }
core-extension-view-failed = This view could not be shown.
core-notify-extension-not-running = The extension { $id } is not running.
core-notify-extension-command-failed = The command { $command } failed: { $error }
core-command-select-theme = Theme: Select Theme…
core-command-select-language = Display Language: Select…
core-palette-themes-placeholder = Select a theme…
core-palette-languages-placeholder = Select the display language…
core-palette-current = current

## Language servers
core-notify-lsp-failed = Could not start the language server { $server }: { $error }
core-problems-none = No problems have been detected.

# Project search
core-command-project-search = Search in Project
core-search-placeholder = Search
core-search-replace-placeholder = Replace
core-search-include-placeholder = Files to include (e.g. src/**/*.rs)
core-search-searching = Searching…
core-search-no-folder = Open a folder to search in it.
core-search-no-results = No results.
core-search-summary = { $count ->
        [one] 1 result
       *[other] { $count } results
    } in { $files ->
        [one] 1 file
       *[other] { $files } files
    }
core-search-summary-truncated = More than { $count } results in { $files ->
        [one] 1 file
       *[other] { $files } files
    } (showing the first ones)
core-search-regex = Use Regular Expression
core-search-whole-word = Match Whole Word
core-search-replace-all = Replace All
core-search-confirm-replace = Replace { $count ->
        [one] 1 occurrence
       *[other] { $count } occurrences
    } in { $files ->
        [one] 1 file
       *[other] { $files } files
    } with “{ $replacement }”?
core-search-replaced = Replaced { $count ->
        [one] 1 occurrence
       *[other] { $count } occurrences
    } in { $files ->
        [one] 1 file
       *[other] { $files } files
    }.

# Diff
core-diff-no-changes = No changes.
core-diff-previous-hunk = Previous Change
core-diff-next-hunk = Next Change
core-diff-open-file = Open File
core-diff-hunks = { $count ->
        [one] 1 hunk
       *[other] { $count } hunks
    }
core-diff-stage-hunk = Stage Hunk
core-diff-revert-hunk = Revert Hunk

# Extensions
core-command-open-extensions = Extensions: Manage Extensions
core-extensions-title = Extensions
core-extensions-tab-installed = Installed
core-extensions-tab-browse = Browse
core-extensions-search = Search extensions
core-extensions-builtin = Built-in
core-extensions-installed = Installed
core-extensions-disabled = Disabled
core-extensions-restart-required = Restart to apply
core-extensions-enable = Enable
core-extensions-disable = Disable
core-extensions-install = Install
core-extensions-installing = Installing…
core-extensions-update = Update to { $version }
core-extensions-uninstall = Uninstall
core-extensions-loading = Loading the extension list…
core-extensions-load-failed = Could not load the extension list: { $error }
core-extensions-retry = Reload
core-extensions-empty = No extensions found.
core-extensions-confirm-install = Install { $name } { $version }?
core-extensions-permissions = This extension asks for:
core-extensions-no-permissions = • No permissions
core-extensions-strong-note = ⚠ marks strong permissions (editing files, running programs, network).
core-extensions-installed-notify = { $name } was installed.
core-extensions-install-failed = Could not install { $name }: { $error }
core-extensions-confirm-uninstall = Uninstall { $name }? It is removed when Nucleus restarts.
core-extensions-uninstalled-notify = { $name } was uninstalled. Restart Nucleus to finish.
core-extensions-settings = Settings
core-extensions-on = On
core-extensions-off = Off
core-extensions-open = Open Extensions
core-extensions-updates-available = { $count ->
        [one] An extension update is available
       *[other] { $count } extension updates are available
    }: { $list }
core-status-extension-updates = { $count ->
        [one] 1 update
       *[other] { $count } updates
    }
core-extensions-updated-notify = { $name } was updated to { $version }.
core-extensions-number-invalid = Enter a number ({ $range })

# Menus
core-menu-item-new-file = New File
core-menu-item-open-recent = Open Recent
core-menu-item-undo = Undo
core-menu-item-redo = Redo
core-menu-item-cut = Cut
core-menu-item-copy = Copy
core-menu-item-paste = Paste
core-menu-item-indent = Indent
core-menu-item-outdent = Outdent
core-menu-item-select-all = Select All
core-menu-item-select-to-line-start = Select to Start of Line
core-menu-item-select-to-line-end = Select to End of Line
core-menu-item-find = Find
core-menu-item-replace = Replace
core-menu-item-find-next = Find Next
core-menu-item-find-previous = Find Previous
core-menu-item-find-replace = Find / Replace in File
core-menu-item-find-next-previous = Find Next / Previous
core-menu-item-go-to-definition = Go to Definition
core-menu-item-toggle-docks = Toggle Left / Bottom / Right Dock
core-menu-item-documentation = Documentation
core-menu-item-keyboard-shortcuts = Keyboard Shortcuts
core-menu-item-report-issue = Report an Issue
core-menu-item-about = About Nucleus
core-clone-choose-folder = Choose Folder…
core-clone-hint = The repository is cloned into a new folder inside the folder you choose next.
core-clone-started = Cloning { $url }…
core-clone-failed = Could not clone: { $error }
core-recent-missing = { $path } no longer exists.

# Settings
core-settings-title = Settings
core-settings-general = General
core-settings-editor = Text Editor
core-settings-extensions = Extensions
core-settings-search = Search settings
core-settings-open-json = Open settings.json
core-settings-invalid = This value is not allowed.
core-settings-system-language = System
core-settings-more-in-json = Key bindings can be edited in settings.json.
core-settings-item-theme = Color Theme
core-settings-item-theme-description = The colors of the whole window. Extensions can add themes.
core-settings-item-language = Display Language
core-settings-item-language-description = The language of menus and messages. Language packs add more.
core-settings-item-restore-session = Restore Session
core-settings-item-restore-session-description = Reopen the last folder and files when Nucleus starts.
core-settings-item-editor-font-family = Font Family
core-settings-item-editor-font-family-description = The font used in editors.
core-settings-item-editor-font-size = Font Size
core-settings-item-editor-font-size-description = In pixels (8–40).
core-settings-item-editor-tab-size = Tab Size
core-settings-item-editor-tab-size-description = The number of spaces a tab is shown as (1–16).
core-settings-item-editor-hard-tabs = Insert Tabs
core-settings-item-editor-hard-tabs-description = Insert a tab character instead of spaces.
core-settings-item-editor-soft-wrap = Word Wrap
core-settings-item-editor-soft-wrap-description = Wrap long lines at the edge of the editor.
core-settings-item-editor-line-numbers = Line Numbers
core-settings-item-editor-line-numbers-description = Show line numbers in the gutter.
core-settings-item-editor-indent-guides = Indent Guides
core-settings-item-editor-indent-guides-description = Draw vertical lines at each indentation level.
core-settings-item-check-extension-updates = Check for Extension Updates
core-settings-item-check-extension-updates-description = When Nucleus starts, look for newer versions of the extensions you installed.
core-settings-item-extensions-registry = Extension Registry
core-settings-item-extensions-registry-description = The index.json to browse extensions from. Leave empty for the official registry.
core-settings-files = Files
core-settings-workbench = Workbench
core-settings-explorer = Explorer
core-settings-search-category = Search
core-settings-terminal = Terminal
core-settings-list-add = Add a pattern and press Enter
core-settings-value-none = None
core-settings-value-same = Same as the line
core-settings-value-all = All
core-settings-value-blink = Blink
core-settings-value-solid = Solid
core-settings-value-off = Off
core-settings-value-after-delay = After a delay
core-settings-value-on-focus-change = When focus changes
core-settings-value-auto = Auto (by OS)
core-settings-value-lf = LF
core-settings-value-crlf = CRLF
core-settings-value-default = Folders first
core-settings-value-mixed = Mixed
core-settings-value-files-first = Files first
core-settings-value-type = By type
core-settings-item-window-zoom-level = Zoom Level
core-settings-item-window-zoom-level-description = The size of the whole window. 0 is the default; each step up or down is 20% larger or smaller.
core-settings-item-editor-line-height = Line Height
core-settings-item-editor-line-height-description = The height of each line, as a multiple of the font size (1–3).
core-settings-item-editor-detect-indentation = Detect Indentation
core-settings-item-editor-detect-indentation-description = When a file opens, set the tab size and whether to use tabs from its contents.
core-settings-item-editor-wrapping-indent = Wrapping Indent
core-settings-item-editor-wrapping-indent-description = Where wrapped lines start.
core-settings-item-editor-render-whitespace = Render Whitespace
core-settings-item-editor-render-whitespace-description = Show spaces and tabs as symbols.
core-settings-item-editor-folding = Folding
core-settings-item-editor-folding-description = Allow folding blocks of code.
core-settings-item-editor-auto-closing-brackets = Auto Closing Brackets
core-settings-item-editor-auto-closing-brackets-description = When you type an opening bracket or quote, add the closing one.
core-settings-item-editor-auto-indent = Auto Indent
core-settings-item-editor-auto-indent-description = Indent new lines to suit the language when you press Enter.
core-settings-item-editor-scroll-beyond-last-line = Scroll Beyond Last Line
core-settings-item-editor-scroll-beyond-last-line-description = Allow scrolling past the last line.
core-settings-item-editor-cursor-surrounding-lines = Cursor Surrounding Lines
core-settings-item-editor-cursor-surrounding-lines-description = The number of lines kept visible above and below the cursor (0 uses the default).
core-settings-item-editor-cursor-blinking = Cursor Blinking
core-settings-item-editor-cursor-blinking-description = Whether the cursor blinks.
core-settings-item-editor-highlight-line-only-when-focused = Highlight Line Only When Focused
core-settings-item-editor-highlight-line-only-when-focused-description = Highlight the cursor's line only while you are in that editor.
core-settings-item-editor-format-on-save = Format On Save
core-settings-item-editor-format-on-save-description = Format the file with its language server when saving.
core-settings-item-editor-hover = Hover
core-settings-item-editor-hover-description = Show information from the language server when the mouse rests on code.
core-settings-item-files-auto-save = Auto Save
core-settings-item-files-auto-save-description = Save files without pressing Ctrl+S.
core-settings-item-files-auto-save-delay = Auto Save Delay
core-settings-item-files-auto-save-delay-description = With "After a delay", how long to wait after the last change, in milliseconds.
core-settings-item-files-trim-trailing-whitespace = Trim Trailing Whitespace
core-settings-item-files-trim-trailing-whitespace-description = Remove spaces and tabs at the end of lines when saving.
core-settings-item-files-insert-final-newline = Insert Final Newline
core-settings-item-files-insert-final-newline-description = Add a newline at the end of the file when saving, if there is none.
core-settings-item-files-trim-final-newlines = Trim Final Newlines
core-settings-item-files-trim-final-newlines-description = Keep only one newline at the end of the file when saving.
core-settings-item-files-eol = End of Line
core-settings-item-files-eol-description = The line ending for new files. Existing files keep their own.
core-settings-item-files-exclude = Exclude
core-settings-item-files-exclude-description = Glob patterns for files and folders to hide from the file tree, Ctrl+P and search (for example **/node_modules).
core-settings-item-workbench-status-bar = Status Bar
core-settings-item-workbench-status-bar-description = Show the status bar at the bottom of the window.
core-settings-item-workbench-tree-indent = Tree Indent
core-settings-item-workbench-tree-indent-description = The indentation of each level in the file tree, in pixels (4–40).
core-settings-item-workbench-tree-indent-guides = Tree Indent Guides
core-settings-item-workbench-tree-indent-guides-description = Draw vertical lines for each level in the file tree.
core-settings-item-workbench-startup-welcome = Welcome Page on Startup
core-settings-item-workbench-startup-welcome-description = Show the welcome page when Nucleus starts.
core-settings-item-explorer-confirm-delete = Confirm Delete
core-settings-item-explorer-confirm-delete-description = Ask before moving a file or folder to the trash.
core-settings-item-explorer-sort-order = Sort Order
core-settings-item-explorer-sort-order-description = How files and folders are ordered in the file tree.
core-settings-item-explorer-auto-reveal = Auto Reveal
core-settings-item-explorer-auto-reveal-description = Select the active editor's file in the file tree, opening its folders.
core-settings-item-search-exclude = Exclude
core-settings-item-search-exclude-description = Glob patterns for files and folders not to search, in addition to Files: Exclude.
core-settings-item-search-use-ignore-files = Use Ignore Files
core-settings-item-search-use-ignore-files-description = Do not search files listed in .gitignore and similar files.
core-settings-item-search-smart-case = Smart Case
core-settings-item-search-smart-case-description = Match case only when the search contains capital letters.
core-settings-item-search-max-results = Max Results
core-settings-item-search-max-results-description = Stop searching after this many matches.
core-settings-item-terminal-shell = Shell
core-settings-item-terminal-shell-description = The program to run in new terminals (for example pwsh, cmd or bash). Leave empty for the system default.
core-settings-item-terminal-shell-args = Shell Arguments
core-settings-item-terminal-shell-args-description = Arguments passed to the shell.
core-settings-item-terminal-font-family = Font Family
core-settings-item-terminal-font-family-description = The terminal's font. Leave empty to use the editor's.
core-settings-item-terminal-font-size = Font Size
core-settings-item-terminal-font-size-description = In pixels (6–72). Leave empty to use the editor's.
core-settings-item-terminal-line-height = Line Height
core-settings-item-terminal-line-height-description = The height of each line, as a multiple of the font size (1–3).
core-settings-item-terminal-scrollback = Scrollback
core-settings-item-terminal-scrollback-description = The number of lines you can scroll back to. Applies to new terminals.

# Project tree
core-tree-new-file = New File…
core-tree-new-folder = New Folder…
core-tree-rename = Rename…
core-tree-delete = Move to Trash
core-tree-copy-path = Copy Path
core-tree-copy-relative-path = Copy Relative Path
core-tree-open-in-terminal = Open in Terminal
core-tree-ok = OK
core-tree-exists = { $name } already exists.
core-tree-invalid-name = Enter a name without / or \\.
core-tree-confirm-delete = Move { $name } to the trash?
core-tree-delete-detail = You can restore it from the trash.
core-tree-exists-folder = A folder named { $name } already exists in { $dir }.
core-tree-exists-file = A file named { $name } already exists in { $dir }.
