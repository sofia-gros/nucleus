use crate::color::Color;

/// テーマやマニフェストに書かれた色の値。
#[derive(Clone, Debug, PartialEq)]
pub enum ValueExpr {
    /// `#rrggbb` / `#rrggbbaa`
    Hex(Color),
    /// `$key` または `$key/NN%`
    Ref { key: String, alpha_percent: Option<f32> },
}

impl ValueExpr {
    pub fn parse(s: &str) -> Result<ValueExpr, String> {
        let s = s.trim();
        if let Some(rest) = s.strip_prefix('$') {
            let (key, alpha) = match rest.split_once('/') {
                Some((k, a)) => {
                    let pct = a
                        .strip_suffix('%')
                        .and_then(|n| n.trim().parse::<f32>().ok())
                        .filter(|n| (0.0..=100.0).contains(n))
                        .ok_or_else(|| format!("透明度は `/0%`〜`/100%` で書く: {s}"))?;
                    (k, Some(pct))
                }
                None => (rest, None),
            };
            if key.is_empty() {
                return Err(format!("参照先の Token 名がない: {s}"));
            }
            return Ok(ValueExpr::Ref {
                key: key.to_string(),
                alpha_percent: alpha,
            });
        }
        Color::parse_hex(s)
            .map(ValueExpr::Hex)
            .ok_or_else(|| format!("色は `#rrggbb` / `#rrggbbaa` / `$token` / `$token/NN%` で書く: {s}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_values() {
        assert!(matches!(ValueExpr::parse("#fff000"), Ok(ValueExpr::Hex(_))));
        assert_eq!(
            ValueExpr::parse("$primary.background/15%").unwrap(),
            ValueExpr::Ref {
                key: "primary.background".into(),
                alpha_percent: Some(15.0)
            }
        );
        assert_eq!(
            ValueExpr::parse("$background").unwrap(),
            ValueExpr::Ref {
                key: "background".into(),
                alpha_percent: None
            }
        );
        assert!(ValueExpr::parse("$x/150%").is_err());
        assert!(ValueExpr::parse("$").is_err());
        assert!(ValueExpr::parse("red").is_err());
    }
}
