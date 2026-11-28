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
core-command-open-settings = Preferences: Open Settings (JSON)
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
