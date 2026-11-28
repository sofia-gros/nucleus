//! 宣言的な UI（ui@1）を組み立てる道具。
//!
//! ```ignore
//! use nucleus_extension_api::ui::*;
//! v_stack().gap(Size::Sm).padding(Size::Sm).child(text("Hello")).child(button("ok").label("OK"))
//! ```

use crate::bindings::nucleus::extension::ui::IconProps;
pub use crate::bindings::nucleus::extension::ui::{
    ButtonProps, ButtonVariant, Direction, InputProps, Kind, ListItemProps, Node, SectionProps, Size, StackProps,
    TextProps, Tone, UiEvent,
};

/// 要素（子を持てる）。`flatten` で WIT の平らな配列にする。
pub struct El {
    kind: Kind,
    children: Vec<El>,
}

impl El {
    fn new(kind: Kind) -> El {
        El {
            kind,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, c: El) -> El {
        self.children.push(c);
        self
    }

    pub fn children(mut self, cs: impl IntoIterator<Item = El>) -> El {
        self.children.extend(cs);
        self
    }

    /// 条件つきで子を足す。
    pub fn child_if(self, cond: bool, c: impl FnOnce() -> El) -> El {
        if cond { self.child(c()) } else { self }
    }

    fn stack(mut self, f: impl FnOnce(&mut StackProps)) -> El {
        if let Kind::Stack(p) = &mut self.kind {
            f(p);
        }
        self
    }
    pub fn gap(self, s: Size) -> El {
        self.stack(|p| p.gap = s)
    }
    pub fn padding(self, s: Size) -> El {
        self.stack(|p| p.padding = s)
    }
    pub fn grow(self) -> El {
        self.stack(|p| p.grow = true)
    }
    pub fn spread(self) -> El {
        self.stack(|p| p.spread = true)
    }

    /// 文字・アイコン・行のアイコン・印の色。
    pub fn tone(mut self, t: Tone) -> El {
        match &mut self.kind {
            Kind::Text(p) => p.tone = t,
            Kind::Icon(p) => p.tone = t,
            Kind::ListItem(p) => p.icon_tone = t,
            _ => {}
        }
        self
    }
    pub fn size(mut self, s: Size) -> El {
        if let Kind::Text(p) = &mut self.kind {
            p.size = s;
        }
        self
    }
    pub fn bold(mut self) -> El {
        if let Kind::Text(p) = &mut self.kind {
            p.bold = true;
        }
        self
    }
    pub fn mono(mut self) -> El {
        if let Kind::Text(p) = &mut self.kind {
            p.mono = true;
        }
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> El {
        match &mut self.kind {
            Kind::Button(p) => p.label = Some(l.into()),
            Kind::ListItem(p) => p.label = l.into(),
            _ => {}
        }
        self
    }
    pub fn icon(mut self, name: impl Into<String>) -> El {
        match &mut self.kind {
            Kind::Button(p) => p.icon = Some(name.into()),
            Kind::ListItem(p) => p.icon = Some(name.into()),
            _ => {}
        }
        self
    }
    pub fn variant(mut self, v: ButtonVariant) -> El {
        if let Kind::Button(p) = &mut self.kind {
            p.style = v;
        }
        self
    }
    pub fn disabled(mut self, d: bool) -> El {
        if let Kind::Button(p) = &mut self.kind {
            p.disabled = d;
        }
        self
    }
    pub fn tooltip(mut self, t: impl Into<String>) -> El {
        if let Kind::Button(p) = &mut self.kind {
            p.tooltip = Some(t.into());
        }
        self
    }

    pub fn placeholder(mut self, t: impl Into<String>) -> El {
        if let Kind::Input(p) = &mut self.kind {
            p.placeholder = t.into();
        }
        self
    }
    pub fn value(mut self, t: impl Into<String>) -> El {
        if let Kind::Input(p) = &mut self.kind {
            p.value = t.into();
        }
        self
    }
    /// 増やすと入力欄の中身を `value` で置き換える。
    pub fn revision(mut self, r: u32) -> El {
        if let Kind::Input(p) = &mut self.kind {
            p.revision = r;
        }
        self
    }
    pub fn multiline(mut self) -> El {
        if let Kind::Input(p) = &mut self.kind {
            p.multiline = true;
        }
        self
    }

    pub fn detail(mut self, t: impl Into<String>) -> El {
        if let Kind::ListItem(p) = &mut self.kind {
            p.detail = Some(t.into());
        }
        self
    }
    pub fn badge(mut self, t: impl Into<String>, tone: Tone) -> El {
        if let Kind::ListItem(p) = &mut self.kind {
            p.badge = Some(t.into());
            p.badge_tone = tone;
        }
        self
    }
    pub fn selected(mut self, s: bool) -> El {
        if let Kind::ListItem(p) = &mut self.kind {
            p.selected = s;
        }
        self
    }
    pub fn count(mut self, n: u32) -> El {
        if let Kind::Section(p) = &mut self.kind {
            p.count = Some(n);
        }
        self
    }
    pub fn collapsed(mut self, c: bool) -> El {
        if let Kind::Section(p) = &mut self.kind {
            p.collapsed = c;
        }
        self
    }
    /// 行・見出しに付けるボタン（[`button`] で作ったもの）。
    pub fn action(mut self, b: El) -> El {
        let Kind::Button(b) = b.kind else { return self };
        match &mut self.kind {
            Kind::ListItem(p) => p.actions.push(b),
            Kind::Section(p) => p.actions.push(b),
            _ => {}
        }
        self
    }

    /// WIT の平らな配列にする（0 番が根）。
    pub fn flatten(self) -> Vec<Node> {
        let mut out = Vec::new();
        fn push(el: El, out: &mut Vec<Node>) -> u32 {
            let ix = out.len();
            out.push(Node {
                kind: el.kind,
                children: Vec::new(),
            });
            let kids: Vec<u32> = el.children.into_iter().map(|c| push(c, out)).collect();
            out[ix].children = kids;
            ix as u32
        }
        push(self, &mut out);
        out
    }
}

fn stack(direction: Direction) -> El {
    El::new(Kind::Stack(StackProps {
        direction,
        gap: Size::None,
        padding: Size::None,
        grow: false,
        spread: false,
    }))
}

pub fn v_stack() -> El {
    stack(Direction::Vertical)
}
pub fn h_stack() -> El {
    stack(Direction::Horizontal)
}
pub fn scroll() -> El {
    El::new(Kind::Scroll)
}
pub fn divider() -> El {
    El::new(Kind::Divider)
}
pub fn skeleton() -> El {
    El::new(Kind::Skeleton)
}

pub fn text(content: impl Into<String>) -> El {
    El::new(Kind::Text(TextProps {
        content: content.into(),
        tone: Tone::Default,
        size: Size::Sm,
        bold: false,
        mono: false,
    }))
}

pub fn icon(name: impl Into<String>) -> El {
    El::new(Kind::Icon(IconProps {
        name: name.into(),
        tone: Tone::Default,
    }))
}

pub fn button(key: impl Into<String>) -> El {
    El::new(Kind::Button(ButtonProps {
        key: key.into(),
        label: None,
        icon: None,
        style: ButtonVariant::Secondary,
        disabled: false,
        tooltip: None,
    }))
}

pub fn input(key: impl Into<String>) -> El {
    El::new(Kind::Input(InputProps {
        key: key.into(),
        placeholder: String::new(),
        value: String::new(),
        revision: 0,
        multiline: false,
    }))
}

pub fn list_item(key: impl Into<String>, label: impl Into<String>) -> El {
    El::new(Kind::ListItem(ListItemProps {
        key: key.into(),
        label: label.into(),
        detail: None,
        icon: None,
        icon_tone: Tone::Default,
        badge: None,
        badge_tone: Tone::Default,
        selected: false,
        actions: Vec::new(),
    }))
}

pub fn section(key: impl Into<String>, title: impl Into<String>) -> El {
    El::new(Kind::Section(SectionProps {
        key: key.into(),
        title: title.into(),
        count: None,
        collapsed: false,
        actions: Vec::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_puts_the_root_first_and_links_children() {
        let nodes = v_stack()
            .child(text("a"))
            .child(h_stack().child(text("b")).child(text("c")))
            .flatten();
        assert_eq!(nodes.len(), 5);
        assert_eq!(nodes[0].children, vec![1, 2]);
        assert_eq!(nodes[2].children, vec![3, 4]);
    }
}
