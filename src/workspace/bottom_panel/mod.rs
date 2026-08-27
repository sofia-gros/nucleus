use gpui::*;
use gpui_component::theme::ActiveTheme;
use crate::plugin_manager::PluginManagerGlobal;

pub struct BottomPanel {
    pub logs: Vec<String>,
}

impl BottomPanel {
    pub fn new() -> Self {
        Self { logs: Vec::new() }
    }

    pub fn write_log(&mut self, text: String, cx: &mut Context<Self>) {
        self.logs.push(text);
        cx.notify();
    }
}

impl Render for BottomPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut tabs = vec![
            div().px_4().py_1().border_r_1().border_color(cx.theme().border).text_sm().text_color(cx.theme().foreground).child("TERMINAL").into_any_element(),
            div().px_4().py_1().border_r_1().border_color(cx.theme().border).text_sm().text_color(cx.theme().muted_foreground).child("OUTPUT").into_any_element(),
            div().px_4().py_1().border_r_1().border_color(cx.theme().border).text_sm().text_color(cx.theme().muted_foreground).child("PROBLEMS").into_any_element(),
        ];
        
        let mut contents = vec![];
        if self.logs.is_empty() {
            contents.push(div().p_4().text_sm().text_color(cx.theme().muted_foreground).child("> _").into_any_element());
        } else {
            let mut log_list = div().flex().flex_col().p_4().overflow_hidden().h_full();
            for log in &self.logs {
                log_list = log_list.child(
                    div().text_sm().text_color(cx.theme().foreground).child(log.clone())
                );
            }
            contents.push(log_list.into_any_element());
        }

        if cx.has_global::<PluginManagerGlobal>() {
            let pm_global = cx.global::<PluginManagerGlobal>().0.clone();
            let pm = pm_global.read(cx);
            
            for item in &pm.ui_registry.panel_items {
                let title = item.title.clone();
                tabs.push(div().px_4().py_1().border_r_1().border_color(cx.theme().border).text_sm().text_color(cx.theme().muted_foreground).child(title).into_any_element());
                
                // Active panel logic isn't fully implemented yet, so we just append their content
                let ui_ast = item.ui_ast.clone();
                let content = if let serde_json::Value::Object(map) = ui_ast {
                    if let Some(t) = map.get("type").and_then(|t| t.as_str()) {
                        if t == "text" {
                            let val = map.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            div().p_4().text_sm().text_color(cx.theme().foreground).child(val).into_any_element()
                        } else {
                            div().p_4().text_sm().text_color(gpui::rgb(0xef4444)).child(format!("Unsupported UI type: {}", t)).into_any_element()
                        }
                    } else {
                        div().p_4().text_sm().text_color(cx.theme().foreground).child("Empty Plugin UI").into_any_element()
                    }
                } else {
                    div().p_4().text_sm().text_color(cx.theme().foreground).child("Invalid UI AST").into_any_element()
                };
                contents.push(content);
            }
        }

        div()
            .w_full()
            .h(px(200.0))
            .bg(cx.theme().background)
            .border_t_1()
            .border_color(cx.theme().border)
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .flex()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .children(tabs)
            )
            .child(
                div().w_full().flex_1().children(contents)
            )
    }
}
