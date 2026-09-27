pub mod cluster;
pub mod config;
pub mod edit;
pub mod injector;
pub mod table;

pub use cluster::{parse, serialize, Cluster, GroupState};
pub use config::Config;
pub use edit::{apply_command, normalize, Edit};
pub use table::{Axis, CharTable, Group, Kind};

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn cfg() -> Config {
        Config::from_str(
            r#"{
              "groups": {
                "top": {
                  "controls": { "x": "U+06DF", "x_neg": "U+06E0", "y": "U+06EB", "y_neg": "U+06EC" }
                },
                "bottom": {
                  "controls": { "x": "U+08ED", "x_neg": "U+08EE", "y": "U+08EF", "y_neg": "U+08F2" }
                }
              },
              "cgj": "U+034F"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn roundtrip_minimal() {
        let t = CharTable::from_config(&cfg()).unwrap();
        let chars: Vec<char> = "\u{0628}\u{034F}\u{06DF}".chars().collect();
        let c = parse(&chars, chars.len(), &t);
        let out = serialize(&c, &t);
        let out_chars: Vec<char> = out.chars().collect();
        let p = parse(&out_chars, out_chars.len(), &t);
        assert_eq!(p.base, c.base);
        assert_eq!(p.top.state, c.top.state);
        assert_eq!(p.bottom.state, c.bottom.state);
    }

    #[test]
    fn duplicate_control_is_rejected() {
        let bad = r#"{
          "groups": {
            "top": {
              "controls": { "x": "U+06DF", "x_neg": "U+06DF", "y": "U+06EB", "y_neg": "U+06EC" }
            },
            "bottom": {
              "controls": { "x": "U+08ED", "x_neg": "U+08EE", "y": "U+08EF", "y_neg": "U+08F2" }
            }
          },
          "cgj": "U+034F"
        }"#;
        let c = Config::from_str(bad).unwrap();
        assert!(CharTable::from_config(&c).is_err());
    }

    #[test]
    fn scan_stops_at_base() {
        let t = CharTable::from_config(&cfg()).unwrap();
        let chars: Vec<char> = "\u{0628}\u{034F}\u{06DF}".chars().collect();
        let c = parse(&chars, chars.len(), &t);
        assert_eq!(c.base, Some('\u{0628}'));
        assert_eq!(c.consumed, 3);
    }
}
