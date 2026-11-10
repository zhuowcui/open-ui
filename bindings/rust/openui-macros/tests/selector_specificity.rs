//! The selector parser against pinned Chromium 147.
//!
//! `tools/qualification/capture_selector_specificity_oracle.py` recorded,
//! in `docs/progress/evidence/selector-specificity-v1/chromium-windows.json`,
//! the specificity Blink computed for each accepted selector (after nesting
//! in its parent rules) and the selectors Chromium dropped. Every accepted
//! selector must parse to the same specificities, and every dropped one must
//! be an error.

use openui_style::selector::Selector;
use openui_style::selector_parser::parse_selector_list;

const ORACLE: &str = include_str!(
    "../../../../docs/progress/evidence/selector-specificity-v1/chromium-windows.json"
);

fn oracle() -> serde_json::Value {
    serde_json::from_str(ORACLE).expect("oracle JSON")
}

fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item.as_str().expect("string").to_owned())
        .collect()
}

#[test]
fn accepted_selectors_have_chromium_specificity() {
    let oracle = oracle();
    let cases = oracle["accepted"].as_array().expect("accepted cases");
    assert!(cases.len() >= 90, "{} cases", cases.len());
    for case in cases {
        let selector = case["selector"].as_str().expect("selector");
        let mut parent: Option<Vec<Selector>> = None;
        for text in strings(&case["parents"]) {
            let list = parse_selector_list(&text, parent.as_deref())
                .unwrap_or_else(|error| panic!("parent `{text}`: {error:?}"));
            parent = Some(list);
        }
        let list = parse_selector_list(selector, parent.as_deref())
            .unwrap_or_else(|error| panic!("`{selector}`: {error:?}"));
        let actual: Vec<[u8; 3]> = list
            .iter()
            .map(|selector| {
                let (a, b, c) = selector.specificity().tuple();
                [a, b, c]
            })
            .collect();
        let expected: Vec<[u8; 3]> = case["specificity"]
            .as_array()
            .expect("specificity list")
            .iter()
            .map(|tuple| {
                let tuple = tuple.as_array().expect("tuple");
                [0, 1, 2].map(|index| tuple[index].as_u64().expect("component") as u8)
            })
            .collect();
        assert_eq!(actual, expected, "`{selector}` in {:?}", case["parents"]);
    }
}

#[test]
fn selectors_chromium_drops_are_errors() {
    let oracle = oracle();
    let rejected = strings(&oracle["rejected"]);
    assert!(rejected.len() >= 15);
    for selector in rejected {
        assert!(
            parse_selector_list(&selector, None).is_err(),
            "`{selector}` parsed, but Chromium drops it"
        );
    }
}
