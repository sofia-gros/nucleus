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
core-command-select-theme = Theme: Select Theme…
core-command-select-language = Display Language: Select…
core-palette-themes-placeholder = Select a theme…
core-palette-languages-placeholder = Select the display language…
core-palette-current = current
