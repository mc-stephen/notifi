//! Shared `{{var}}` substitution for every rendered message in the
//! codebase — system mail subjects/bodies (`infra::system_templates`) and
//! channel template content (schedules, inbound fires, campaigns) all
//! funnel through [`fill`], so a completed body — never a raw `{{name}}`
//! placeholder — is what reaches a provider.
//!
//! Semantics: placeholders are exactly `{{key}}` (two braces each side).
//! A known key is replaced by its value; an unknown placeholder is left
//! intact in the output (visible and debuggable, never silently dropped).
//! Empty/missing values with an ever-present key map behave like the other
//! values — an empty string replaces the placeholder (caller decides).
//!
//! Replacement is deterministic for disjoint keys because needles are
//! brace-delimited exact strings (`{{a}}` cannot match inside `{{aa}}`).
//! One caveat: a *value* that itself contains a placeholder may be
//! re-consumed depending on map iteration order — don't template inside
//! your template values.

use std::collections::HashMap;

/// Replaces each `{{key}}` in `template` with its `values` entry.
pub fn fill(template: &str, values: &HashMap<String, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in values {
        let needle = ["{{", key.as_str(), "}}"].concat();
        out = out.replace(&needle, value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn fills_known_placeholders() {
        let out = fill(
            "Hi {{name}}, reset link: {{link}}",
            &vars(&[("name", "Ada"), ("link", "https://n.io/abc")]),
        );
        assert_eq!(out, "Hi Ada, reset link: https://n.io/abc");
    }

    #[test]
    fn fills_repeated_placeholder() {
        let out = fill("{{x}} and {{x}}", &vars(&[("x", "y")]));
        assert_eq!(out, "y and y");
    }

    #[test]
    fn unknown_placeholder_stays_visible() {
        let out = fill("Hi {{name}}, you owe {{amount}}", &vars(&[("name", "Ada")]));
        assert_eq!(out, "Hi Ada, you owe {{amount}}");
    }

    #[test]
    fn empty_values_leaves_template_untouched() {
        assert_eq!(fill("Hi {{name}}", &HashMap::new()), "Hi {{name}}");
    }

    #[test]
    fn brace_count_is_exact() {
        // `{{a}}` must not match inside `{{aa}}`; odd-brace wrappers are
        // literal text that stays around the filled placeholder.
        let out = fill("{{aa}} {{a}}", &vars(&[("a", "one"), ("aa", "two")]));
        assert_eq!(out, "two one");
        let out = fill("{{{a}}}", &vars(&[("a", "x")]));
        assert_eq!(out, "{x}"); // inner {{a}} fills; the extra braces stay
    }

    #[test]
    fn empty_value_still_replaces_placeholder() {
        let out = fill("tag:{{empty}}end", &vars(&[("empty", "")]));
        assert_eq!(out, "tag:end");
    }
}
