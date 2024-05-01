use super::{Context, Module, ModuleConfig};
use crate::segment::FillSegment;
use crate::segment::Segment;
use crate::segment::TextSegment;
use nu_ansi_term::Style;
use unicode_segmentation::UnicodeSegmentation;

use crate::configs::username::UsernameConfig;
use crate::formatter::StringFormatter;

fn gradientify(
    segment: &Segment,
    gradient: colorgrad::Gradient,
    n: usize,
    k: usize,
) -> Vec<Segment> {
    let st = match segment.style() {
        Some(style) => style,
        None => Style::default(),
    };

    gradient
        .colors(n)
        .iter()
        .skip(k)
        .map(|color| color.to_linear_rgba_u8())
        .zip(segment.value().graphemes(true))
        .map(|((r, g, b, _), val)| match segment {
            Segment::Text(_) => Segment::Text(TextSegment {
                value: val.into(),
                style: Some(nu_ansi_term::Style {
                    foreground: Some(nu_ansi_term::Color::Rgb(r, g, b)),
                    ..st
                }),
            }),
            Segment::Fill(_) => Segment::Fill(FillSegment {
                value: val.into(),
                style: Some(nu_ansi_term::Style { ..st }),
            }),
            _ => Segment::Text(TextSegment {
                value: val.into(),
                style: Some(nu_ansi_term::Style { ..st }),
            }),
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
const USERNAME_ENV_VAR: &str = "USER";

#[cfg(target_os = "windows")]
const USERNAME_ENV_VAR: &str = "USERNAME";

/// Creates a module with the current user's username
///
/// Will display the username if any of the following criteria are met:
///     - The current user is root (UID = 0) [1]
///     - The current user isn't the same as the one that is logged in (`$LOGNAME` != `$USER`) [2]
///     - The user is currently connected as an SSH session (`$SSH_CONNECTION`) [3]
pub fn module<'a>(context: &'a Context) -> Option<Module<'a>> {
    let mut username = context.get_env(USERNAME_ENV_VAR)?;

    let mut module = context.new_module("gradient_username");
    let config: UsernameConfig = UsernameConfig::try_load(module.config);

    let is_root = is_root_user();
    if cfg!(target_os = "windows") && is_root {
        username = "Administrator".to_string();
    }
    let show_username = config.show_always
        || is_root // [1]
        || !is_login_user(context, &username) // [2]
        || is_ssh_session(context); // [3]

    if !show_username {
        return None;
    }

    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_style(|variable| match variable {
                "style" => {
                    let module_style = if is_root {
                        config.style_root
                    } else {
                        config.style_user
                    };
                    Some(Ok(module_style))
                }
                _ => None,
            })
            .map(|variable| match variable {
                "user" => Some(Ok(&username)),
                _ => None,
            })
            .parse(None, Some(context))
    });

    module.set_segments(match parsed {
        Ok(segments) => {
            let mut total = 0;

            segments
                .iter()
                .flat_map(|segment| {
                    let w = gradientify(
                        segment,
                        match colorgrad::CustomGradient::new()
                            //.html_colors(&["#D2AC47", "#F7EF8a", "#EDC967"]) // Gold
                            //.domain(&[0.0, 5.0, 100.0]) // Gold
                            .html_colors(&["#C7D2FE", "#FECACA", "#FEF9C3"]) // Sunset
                            .domain(&[0.0, 50.0, 100.0]) // Sunset
                            .build()
                        {
                            Ok(g) => g,
                            Err(error) => {
                                log::warn!("Error in module `gradient`:\n{}", error);
                                colorgrad::magma()
                            }
                        },
                        144,
                        total,
                    );
                    total += segment.value().len();
                    w
                })
                .collect()
        }
        Err(error) => {
            log::warn!("Error in module `gradient_username`:\n{}", error);
            return None;
        }
    });

    Some(module)
}

fn is_login_user(context: &Context, username: &str) -> bool {
    context
        .get_env("LOGNAME")
        .map_or(true, |logname| logname == username)
}

#[cfg(all(target_os = "windows", not(test)))]
fn is_root_user() -> bool {
    use deelevate::{PrivilegeLevel, Token};
    let token = match Token::with_current_process() {
        Ok(token) => token,
        Err(e) => {
            log::warn!("Failed to get process token: {e:?}");
            return false;
        }
    };
    matches!(
        match token.privilege_level() {
            Ok(level) => level,
            Err(e) => {
                log::warn!("Failed to get privilege level: {e:?}");
                return false;
            }
        },
        PrivilegeLevel::Elevated | PrivilegeLevel::HighIntegrityAdmin
    )
}

#[cfg(all(target_os = "windows", test))]
fn is_root_user() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
fn is_root_user() -> bool {
    nix::unistd::geteuid() == nix::unistd::ROOT
}

fn is_ssh_session(context: &Context) -> bool {
    let ssh_env = ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"];
    ssh_env.iter().any(|env| context.get_env_os(env).is_some())
}

#[cfg(test)]
mod tests {
    use crate::test::ModuleRenderer;

    // TODO: Add tests for if root user (UID == 0)
    // Requires mocking

    #[test]
    fn no_env_variables() {
        let actual = ModuleRenderer::new("username").collect();
        let expected = None;

        assert_eq!(expected, actual);
    }

    #[test]
    #[ignore]
    fn no_logname_env_variable() {
        let actual = ModuleRenderer::new("username")
            .env(super::USERNAME_ENV_VAR, "astronaut")
            .collect();
        let expected = None;

        assert_eq!(expected, actual);
    }

    #[test]
    #[ignore]
    fn logname_equals_user() {
        let actual = ModuleRenderer::new("username")
            .env("LOGNAME", "astronaut")
            .env(super::USERNAME_ENV_VAR, "astronaut")
            .collect();
        let expected = None;

        assert_eq!(expected, actual);
    }

    #[test]
    fn ssh_wo_username() {
        // SSH connection w/o username
        let actual = ModuleRenderer::new("username")
            .env("SSH_CONNECTION", "192.168.223.17 36673 192.168.223.229 22")
            .collect();
        let expected = None;

        assert_eq!(expected, actual);
    }

    #[test]
    fn current_user_not_logname() {
        let actual = ModuleRenderer::new("username")
            .env("LOGNAME", "astronaut")
            .env(super::USERNAME_ENV_VAR, "cosmonaut")
            // Test output should not change when run by root/non-root user
            .config(toml::toml! {
                [username]
                style_root = ""
                style_user = ""
            })
            .collect();
        let expected = Some("cosmonaut in ");

        assert_eq!(expected, actual.as_deref());
    }

    #[test]
    fn ssh_connection() {
        let actual = ModuleRenderer::new("username")
            .env(super::USERNAME_ENV_VAR, "astronaut")
            .env("SSH_CONNECTION", "192.168.223.17 36673 192.168.223.229 22")
            // Test output should not change when run by root/non-root user
            .config(toml::toml! {
                [username]
                style_root = ""
                style_user = ""
            })
            .collect();
        let expected = Some("astronaut in ");

        assert_eq!(expected, actual.as_deref());
    }

    #[test]
    fn ssh_connection_tty() {
        let actual = ModuleRenderer::new("username")
            .env(super::USERNAME_ENV_VAR, "astronaut")
            .env("SSH_TTY", "/dev/pts/0")
            // Test output should not change when run by root/non-root user
            .config(toml::toml! {
                [username]
                style_root = ""
                style_user = ""
            })
            .collect();
        let expected = Some("astronaut in ");

        assert_eq!(expected, actual.as_deref());
    }

    #[test]
    fn ssh_connection_client() {
        let actual = ModuleRenderer::new("username")
            .env(super::USERNAME_ENV_VAR, "astronaut")
            .env("SSH_CLIENT", "192.168.0.101 39323 22")
            // Test output should not change when run by root/non-root user
            .config(toml::toml! {
                [username]
                style_root = ""
                style_user = ""
            })
            .collect();
        let expected = Some("astronaut in ");

        assert_eq!(expected, actual.as_deref());
    }

    #[test]
    fn show_always() {
        let actual = ModuleRenderer::new("username")
            .env(super::USERNAME_ENV_VAR, "astronaut")
            // Test output should not change when run by root/non-root user
            .config(toml::toml! {
                [username]
                show_always = true

                style_root = ""
                style_user = ""
            })
            .collect();
        let expected = Some("astronaut in ");

        assert_eq!(expected, actual.as_deref());
    }
}
