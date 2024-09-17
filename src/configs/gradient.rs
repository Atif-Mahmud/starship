use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(
    feature = "config-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(default)]
pub struct GradientConfig<'a> {
    pub format: &'a str,
    pub gradient: &'a str,
    pub show_always: bool,
    pub disabled: bool,
}

impl<'a> Default for GradientConfig<'a> {
    fn default() -> Self {
        GradientConfig {
            format: "$module",
            gradient: "",
            show_always: false,
            disabled: false,
        }
    }
}
