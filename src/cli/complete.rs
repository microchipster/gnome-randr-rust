use std::{ffi::OsString, time::Duration};

use dbus::blocking::Connection;
use gnome_randr::{display_config::physical_monitor::PhysicalMonitor, DisplayConfig};

use super::{brightness::FILTER_VALUES, common::format_scale};

const COMPLETE_COMMAND: &str = "__complete";
const BRIGHTNESS_VALUES: &[&str] = &["0", "0.25", "0.5", "0.75", "1", "1.25", "1.5", "2"];
const ROTATION_VALUES: &[&str] = &["normal", "left", "right", "inverted"];

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingValue {
    Rotate,
    Mode,
    Scale,
    Brightness,
    Filter,
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedContext {
    connector: Option<String>,
    pending_value: Option<PendingValue>,
}

#[derive(Debug, PartialEq, Eq)]
enum CompletionKind {
    Connector,
    Rotate,
    Mode { connector: Option<String> },
    Scale { connector: Option<String> },
    Brightness,
    Filter,
}

#[derive(Debug, PartialEq, Eq)]
struct CompletionRequest {
    kind: CompletionKind,
    current: String,
    prefix: String,
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn select_monitors<'a>(
    config: &'a DisplayConfig,
    connector: Option<&str>,
) -> Vec<&'a PhysicalMonitor> {
    match connector {
        Some(connector) => config
            .search(connector)
            .map(|(_, physical_monitor)| vec![physical_monitor])
            .unwrap_or_default(),
        None => config.monitors.iter().collect(),
    }
}

fn filter_current(values: Vec<String>, current: &str) -> Vec<String> {
    if current.is_empty() {
        return values;
    }

    values
        .into_iter()
        .filter(|value| value.starts_with(current))
        .collect()
}

fn parse_context(words: &[String], subcommand: &str) -> ParsedContext {
    let mut pending_value = None;
    let mut connector = None;
    let mut positional_only = false;

    for word in words {
        if pending_value.take().is_some() {
            continue;
        }

        if positional_only {
            if connector.is_none() {
                connector = Some(word.clone());
            }
            continue;
        }

        pending_value = match (subcommand, word.as_str()) {
            (_, "--") => {
                positional_only = true;
                None
            }
            ("modify", "--rotate") | ("modify", "-r") => Some(PendingValue::Rotate),
            ("modify", "--mode") | ("modify", "-m") => Some(PendingValue::Mode),
            ("modify", "--scale") => Some(PendingValue::Scale),
            ("modify", "--brightness") => Some(PendingValue::Brightness),
            ("modify", "--filter") => Some(PendingValue::Filter),
            _ if word.starts_with('-') => None,
            _ => {
                if connector.is_none() {
                    connector = Some(word.clone());
                }
                None
            }
        };
    }

    ParsedContext {
        connector,
        pending_value,
    }
}

fn connector_completion_requested(context: &ParsedContext, current: &str) -> bool {
    !current.is_empty() && !current.starts_with('-') && context.connector.is_none()
}

fn completion_request(words: &[String], current: &str) -> Option<CompletionRequest> {
    let (subcommand, subcommand_words) = words.split_first()?;
    let context = parse_context(subcommand_words, subcommand);
    let dynamic_option = match (subcommand.as_str(), current.split_once('=')) {
        ("modify", Some(("--rotate", fragment))) => Some((CompletionKind::Rotate, fragment)),
        ("modify", Some(("--mode", fragment))) => Some((
            CompletionKind::Mode {
                connector: context.connector.clone(),
            },
            fragment,
        )),
        ("modify", Some(("--scale", fragment))) => Some((
            CompletionKind::Scale {
                connector: context.connector.clone(),
            },
            fragment,
        )),
        ("modify", Some(("--brightness", fragment))) => {
            Some((CompletionKind::Brightness, fragment))
        }
        ("modify", Some(("--filter", fragment))) => Some((CompletionKind::Filter, fragment)),
        _ => None,
    };

    if let Some((kind, fragment)) = dynamic_option {
        return Some(CompletionRequest {
            kind,
            current: fragment.to_string(),
            prefix: current[..current.len() - fragment.len()].to_string(),
        });
    }

    let kind = match subcommand.as_str() {
        "query" => {
            if connector_completion_requested(&context, current) {
                Some(CompletionKind::Connector)
            } else {
                None
            }
        }
        "modify" => match context.pending_value {
            Some(PendingValue::Mode) => Some(CompletionKind::Mode {
                connector: context.connector,
            }),
            Some(PendingValue::Scale) => Some(CompletionKind::Scale {
                connector: context.connector,
            }),
            Some(PendingValue::Rotate) => Some(CompletionKind::Rotate),
            Some(PendingValue::Brightness) => Some(CompletionKind::Brightness),
            Some(PendingValue::Filter) => Some(CompletionKind::Filter),
            _ => {
                if connector_completion_requested(&context, current) {
                    Some(CompletionKind::Connector)
                } else {
                    None
                }
            }
        },
        _ => None,
    }?;

    Some(CompletionRequest {
        kind,
        current: current.to_string(),
        prefix: String::new(),
    })
}

fn completion_values(kind: CompletionKind, config: &DisplayConfig, current: &str) -> Vec<String> {
    let mut values = Vec::new();

    match kind {
        CompletionKind::Connector => {
            for monitor in &config.monitors {
                push_unique(&mut values, monitor.connector.clone());
            }
        }
        CompletionKind::Rotate => {
            values.extend(ROTATION_VALUES.iter().map(|value| value.to_string()));
        }
        CompletionKind::Mode { connector } => {
            for monitor in select_monitors(config, connector.as_deref()) {
                for mode in &monitor.modes {
                    push_unique(&mut values, mode.id.clone());
                }
            }
        }
        CompletionKind::Scale { connector } => {
            for monitor in select_monitors(config, connector.as_deref()) {
                for mode in &monitor.modes {
                    for scale in &mode.supported_scales {
                        push_unique(&mut values, format_scale(*scale));
                    }
                }
            }
        }
        CompletionKind::Brightness => {
            values.extend(BRIGHTNESS_VALUES.iter().map(|value| value.to_string()));
        }
        CompletionKind::Filter => {
            values.extend(FILTER_VALUES.iter().map(|value| value.to_string()));
        }
    }

    filter_current(values, current)
}

pub fn try_handle(args: &[OsString]) -> Result<bool, Box<dyn std::error::Error>> {
    if args
        .get(1)
        .map(|arg| arg == COMPLETE_COMMAND)
        .unwrap_or(false)
    {
        let current = args
            .get(2)
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        let words = args
            .iter()
            .skip(3)
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<String>>();

        if let Some(request) = completion_request(&words, &current) {
            let values = match request.kind {
                CompletionKind::Brightness => filter_current(
                    BRIGHTNESS_VALUES
                        .iter()
                        .map(|value| value.to_string())
                        .collect(),
                    &request.current,
                ),
                CompletionKind::Rotate => filter_current(
                    ROTATION_VALUES
                        .iter()
                        .map(|value| value.to_string())
                        .collect(),
                    &request.current,
                ),
                CompletionKind::Filter => filter_current(
                    FILTER_VALUES
                        .iter()
                        .map(|value| value.to_string())
                        .collect(),
                    &request.current,
                ),
                kind => {
                    let conn = Connection::new_session()?;
                    let proxy = conn.with_proxy(
                        "org.gnome.Mutter.DisplayConfig",
                        "/org/gnome/Mutter/DisplayConfig",
                        Duration::from_millis(5000),
                    );
                    let config = DisplayConfig::get_current_state(&proxy)?;
                    completion_values(kind, &config, &request.current)
                }
            };

            for value in values {
                println!("{}{}", request.prefix, value);
            }
        }

        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{
        completion_request, parse_context, CompletionKind, CompletionRequest, ParsedContext,
        PendingValue,
    };

    fn words(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_context_tracks_connector_and_pending_value() {
        assert_eq!(
            parse_context(&words(&["eDP-1", "--mode"]), "modify"),
            ParsedContext {
                connector: Some("eDP-1".to_string()),
                pending_value: Some(PendingValue::Mode),
            }
        );
    }

    #[test]
    fn query_completes_connector_after_flags() {
        assert_eq!(
            completion_request(&words(&["query", "--summary"]), "e"),
            Some(CompletionRequest {
                kind: CompletionKind::Connector,
                current: "e".to_string(),
                prefix: String::new(),
            })
        );
    }

    #[test]
    fn empty_positional_does_not_hide_static_completion() {
        assert_eq!(completion_request(&words(&["modify"]), ""), None);
        assert_eq!(completion_request(&words(&["query"]), ""), None);
    }

    #[test]
    fn modify_completes_mode_values_for_selected_connector() {
        assert_eq!(
            completion_request(&words(&["modify", "eDP-1", "--mode"]), ""),
            Some(CompletionRequest {
                kind: CompletionKind::Mode {
                    connector: Some("eDP-1".to_string()),
                },
                current: String::new(),
                prefix: String::new(),
            })
        );
    }

    #[test]
    fn modify_completes_scale_values_without_connector() {
        assert_eq!(
            completion_request(&words(&["modify", "--scale"]), ""),
            Some(CompletionRequest {
                kind: CompletionKind::Scale { connector: None },
                current: String::new(),
                prefix: String::new(),
            })
        );
    }

    #[test]
    fn modify_completes_brightness_values() {
        assert_eq!(
            completion_request(&words(&["modify", "--brightness"]), ""),
            Some(CompletionRequest {
                kind: CompletionKind::Brightness,
                current: String::new(),
                prefix: String::new(),
            })
        );
    }

    #[test]
    fn modify_completes_filter_values() {
        assert_eq!(
            completion_request(&words(&["modify", "--filter"]), ""),
            Some(CompletionRequest {
                kind: CompletionKind::Filter,
                current: String::new(),
                prefix: String::new(),
            })
        );
    }

    #[test]
    fn options_do_not_trigger_dynamic_connector_completion() {
        assert_eq!(completion_request(&words(&["modify"]), "--"), None);
    }

    #[test]
    fn long_option_equals_form_is_completed() {
        assert_eq!(
            completion_request(&words(&["modify"]), "--brightness=1"),
            Some(CompletionRequest {
                kind: CompletionKind::Brightness,
                current: "1".to_string(),
                prefix: "--brightness=".to_string(),
            })
        );

        assert_eq!(
            completion_request(&words(&["modify"]), "--filter=g"),
            Some(CompletionRequest {
                kind: CompletionKind::Filter,
                current: "g".to_string(),
                prefix: "--filter=".to_string(),
            })
        );
    }
}
