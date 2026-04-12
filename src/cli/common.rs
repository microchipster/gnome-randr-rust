#[derive(Debug)]
pub enum ConnectorError {
    NoOutputs,
    Required(Vec<String>),
}

impl std::fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectorError::NoOutputs => write!(f, "fatal: unable to find output."),
            ConnectorError::Required(connectors) => write!(
                f,
                "fatal: please specify an output. Available outputs: {}",
                connectors.join(", ")
            ),
        }
    }
}

impl std::error::Error for ConnectorError {}

pub fn resolve_connector<I, S>(
    provided: Option<&str>,
    connectors: I,
) -> Result<String, ConnectorError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if let Some(connector) = provided {
        return Ok(connector.to_string());
    }

    let connectors = connectors
        .into_iter()
        .map(|connector| connector.as_ref().to_string())
        .collect::<Vec<String>>();

    match connectors.len() {
        0 => Err(ConnectorError::NoOutputs),
        1 => Ok(connectors[0].clone()),
        _ => Err(ConnectorError::Required(connectors)),
    }
}

pub fn format_scale(scale: f64) -> String {
    let formatted = format!("{:.2}", scale);
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{format_scale, resolve_connector, ConnectorError};

    #[test]
    fn connector_defaults_when_only_one_output_exists() {
        let connector = resolve_connector(None, ["HDMI-1"]).unwrap();
        assert_eq!(connector, "HDMI-1");
    }

    #[test]
    fn connector_errors_when_multiple_outputs_exist() {
        let error = resolve_connector::<_, &str>(None, ["HDMI-1", "DP-1"]).unwrap_err();
        match error {
            ConnectorError::Required(connectors) => {
                assert_eq!(connectors, vec!["HDMI-1", "DP-1"])
            }
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn scale_formatting_trims_trailing_zeroes() {
        assert_eq!(format_scale(1.0), "1");
        assert_eq!(format_scale(1.25), "1.25");
        assert_eq!(format_scale(1.50), "1.5");
    }
}
