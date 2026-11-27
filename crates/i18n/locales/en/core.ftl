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
