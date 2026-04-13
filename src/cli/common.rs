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

pub fn format_refresh(refresh: f64) -> String {
    let formatted = format!("{:.2}", refresh);
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

pub fn parse_resolution(value: &str) -> Option<(i32, i32)> {
    let (width, height) = value.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

pub fn parse_position(value: &str) -> Option<(i32, i32)> {
    let (x, y) = value
        .split_once(',')
        .or_else(|| value.rsplit_once('x'))
        .or_else(|| value.rsplit_once('X'))?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

const DISPLAYED_SCALE_TOLERANCE: f64 = 0.005_001;

pub fn match_supported_scale(requested: f64, supported_scales: &[f64]) -> Option<f64> {
    supported_scales
        .iter()
        .copied()
        .find(|scale| *scale == requested)
        .or_else(|| {
            supported_scales
                .iter()
                .copied()
                .filter(|scale| (*scale - requested).abs() <= DISPLAYED_SCALE_TOLERANCE)
                .min_by(|left, right| {
                    let left_distance = (*left - requested).abs();
                    let right_distance = (*right - requested).abs();

                    left_distance
                        .partial_cmp(&right_distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::{
        format_refresh, format_scale, match_supported_scale, parse_position, parse_resolution,
        resolve_connector, ConnectorError,
    };

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

    #[test]
    fn refresh_formatting_trims_trailing_zeroes() {
        assert_eq!(format_refresh(60.0), "60");
        assert_eq!(format_refresh(59.93), "59.93");
    }

    #[test]
    fn resolution_parser_accepts_width_x_height() {
        assert_eq!(parse_resolution("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_resolution("1920xfoo"), None);
        assert_eq!(parse_resolution("1920"), None);
    }

    #[test]
    fn position_parser_accepts_comma_and_x_forms() {
        assert_eq!(parse_position("100,200"), Some((100, 200)));
        assert_eq!(parse_position("-1920x0"), Some((-1920, 0)));
        assert_eq!(parse_position("100xfoo"), None);
    }

    #[test]
    fn supported_scale_match_prefers_exact_match() {
        let matched = match_supported_scale(1.75, &[1.0, 1.75, 1.7518248]).unwrap();
        assert_eq!(matched, 1.75);
    }

    #[test]
    fn supported_scale_match_accepts_displayed_value_within_tolerance() {
        let matched = match_supported_scale(1.75, &[1.0, 1.7518248, 2.0]).unwrap();
        assert_eq!(matched, 1.7518248);
    }

    #[test]
    fn supported_scale_match_rejects_values_outside_tolerance() {
        let matched = match_supported_scale(1.75, &[1.0, 1.76, 2.0]);
        assert_eq!(matched, None);
    }
}
