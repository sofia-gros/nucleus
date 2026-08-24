use gpui::*;

pub mod title_bar;
pub mod status_bar;
pub mod activity_bar;
pub mod left_sidebar;
pub mod right_sidebar;
pub mod bottom_panel;
pub mod editor_area;

use title_bar::TitleBar;
use status_bar::StatusBar;
use activity_bar::ActivityBar;
use left_sidebar::LeftSidebar;
use right_sidebar::RightSidebar;
use bottom_panel::BottomPanel;
use editor_area::EditorArea;

pub struct Workspace {
    title_bar: Entity<TitleBar>,
    status_bar: Entity<StatusBar>,
    activity_bar: Entity<ActivityBar>,
    left_sidebar: Entity<LeftSidebar>,
    right_sidebar: Entity<RightSidebar>,
    bottom_panel: Entity<BottomPanel>,
    editor_area: Entity<EditorArea>,
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            title_bar: cx.new(|_| TitleBar::new()),
            status_bar: cx.new(|_| StatusBar::new()),
            activity_bar: cx.new(|_| ActivityBar::new()),
            left_sidebar: cx.new(|_| LeftSidebar::new()),
            right_sidebar: cx.new(|_| RightSidebar::new()),
            bottom_panel: cx.new(|_| BottomPanel::new()),
            editor_area: cx.new(|_| EditorArea::new()),
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::rgb(0x0f172a)) // Slate 900
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            .child(
                div().flex_grow(1.).flex().flex_row()
                    .child(self.activity_bar.clone())
                    .child(self.left_sidebar.clone())
                    .child(
                        div().flex_grow(1.).flex().flex_col()
                            .child(self.editor_area.clone())
                            .child(self.bottom_panel.clone())
                    )
                    .child(self.right_sidebar.clone())
            )
            .child(self.status_bar.clone())
    }
}
