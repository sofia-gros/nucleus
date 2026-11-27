use std::fmt;

/// 8bit RGBA の色。テーマの値はこの型で持ち、描画時にだけ gpui の色に変換する。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };

    /// `#rrggbb` または `#rrggbbaa`。
    pub fn parse_hex(s: &str) -> Option<Color> {
        let h = s.strip_prefix('#')?;
        if !(h.len() == 6 || h.len() == 8) || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some(Color {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
            a: if h.len() == 8 { byte(6)? } else { 255 },
        })
    }

    /// 透明度を `percent`（0〜100）倍にした色。
    pub fn scale_alpha(self, percent: f32) -> Color {
        let a = (self.a as f32 * (percent / 100.0)).round().clamp(0.0, 255.0) as u8;
        Color { a, ..self }
    }

    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// WCAG の相対輝度。
    pub fn luminance(self) -> f32 {
        let ch = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(self.r) + 0.7152 * ch(self.g) + 0.0722 * ch(self.b)
    }

    /// 背景 `bg` の上に重ねたときの見た目の色（不透明）。
    pub fn over(self, bg: Color) -> Color {
        let a = self.a as f32 / 255.0;
        let mix = |f: u8, b: u8| (f as f32 * a + b as f32 * (1.0 - a)).round() as u8;
        Color {
            r: mix(self.r, bg.r),
            g: mix(self.g, bg.g),
            b: mix(self.b, bg.b),
            a: 255,
        }
    }

    /// WCAG のコントラスト比（1〜21）。
    pub fn contrast(self, other: Color) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex() {
        assert_eq!(
            Color::parse_hex("#282c34"),
            Some(Color {
                r: 0x28,
                g: 0x2c,
                b: 0x34,
                a: 255
            })
        );
        assert_eq!(Color::parse_hex("#00000066").unwrap().a, 0x66);
        assert_eq!(Color::parse_hex("282c34"), None);
        assert_eq!(Color::parse_hex("#28c34"), None);
        assert_eq!(Color::parse_hex("#zzzzzz"), None);
    }

    #[test]
    fn scales_alpha() {
        let c = Color::parse_hex("#98c379").unwrap().scale_alpha(14.0);
        assert_eq!(c.a, 36);
        assert_eq!(c.to_hex(), "#98c37924");
    }

    #[test]
    fn contrast_of_black_and_white_is_21() {
        let w = Color::parse_hex("#ffffff").unwrap();
        let b = Color::parse_hex("#000000").unwrap();
        assert!((w.contrast(b) - 21.0).abs() < 0.01);
    }
}
