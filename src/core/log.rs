use log::LevelFilter;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Verbosity {
    Silent,
    Quiet,
    #[default]
    Normal,
    Verbose(u8),
}

impl std::fmt::Display for Verbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Verbosity {
    pub fn as_str(&self) -> String {
        match self {
            Verbosity::Normal => "normal".to_string(),
            Verbosity::Quiet => "quiet".to_string(),
            Verbosity::Silent => "silent".to_string(),
            Verbosity::Verbose(level) => format!("verbose {}", level),
        }
    }

    pub const fn to_filter_level(&self) -> LevelFilter {
        match self {
            Verbosity::Normal => LevelFilter::Error,
            Verbosity::Quiet => LevelFilter::Off,
            Verbosity::Silent => LevelFilter::Off,
            Verbosity::Verbose(level) => match level {
                0 => LevelFilter::Error,
                1 => LevelFilter::Warn,
                2 => LevelFilter::Info,
                3 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn get_m_string() -> HashMap<Verbosity, String> {
        let m_strings: HashMap<Verbosity, String> = [
            (Verbosity::Normal, "normal".to_string()),
            (Verbosity::Quiet, "quiet".to_string()),
            (Verbosity::Silent, "silent".to_string()),
            (Verbosity::Verbose(0), "verbose 0".to_string()),
            (Verbosity::Verbose(1), "verbose 1".to_string()),
            (Verbosity::Verbose(2), "verbose 2".to_string()),
            (Verbosity::Verbose(3), "verbose 3".to_string()),
            (Verbosity::Verbose(4), "verbose 4".to_string()),
            (Verbosity::Verbose(8), "verbose 8".to_string()),
        ]
        .iter()
        .cloned()
        .collect();
        m_strings
    }

    fn get_m_filter() -> HashMap<Verbosity, LevelFilter> {
        let m_strings: HashMap<Verbosity, LevelFilter> = [
            (Verbosity::Normal, LevelFilter::Error),
            (Verbosity::Quiet, LevelFilter::Off),
            (Verbosity::Silent, LevelFilter::Off),
            (Verbosity::Verbose(0), LevelFilter::Error),
            (Verbosity::Verbose(1), LevelFilter::Warn),
            (Verbosity::Verbose(2), LevelFilter::Info),
            (Verbosity::Verbose(3), LevelFilter::Debug),
            (Verbosity::Verbose(4), LevelFilter::Trace),
            (Verbosity::Verbose(8), LevelFilter::Trace),
        ]
        .iter()
        .cloned()
        .collect();
        m_strings
    }

    fn get_m_u8() -> HashMap<Verbosity, u8> {
        let m_strings: HashMap<Verbosity, u8> = [
            (Verbosity::Verbose(0), 0),
            (Verbosity::Verbose(1), 1),
            (Verbosity::Verbose(2), 2),
            (Verbosity::Verbose(3), 3),
            (Verbosity::Verbose(4), 4),
            (Verbosity::Verbose(8), 8),
        ]
        .iter()
        .cloned()
        .collect();
        m_strings
    }

    #[test]
    fn test_default() {
        assert_eq!(Verbosity::default(), Verbosity::Normal);
    }

    #[test]
    fn test_values() {
        for (k, v) in &get_m_u8() {
            match k {
                Verbosity::Normal => {}
                Verbosity::Quiet => {}
                Verbosity::Silent => {}
                Verbosity::Verbose(u) => assert_eq!(u, v),
            }
        }
    }

    #[test]
    fn test_as_str() {
        for (k, v) in &get_m_string() {
            assert_eq!(&k.as_str(), v);
        }
    }

    #[test]
    fn test_to_filter_level() {
        for (k, v) in &get_m_filter() {
            assert_eq!(&k.to_filter_level(), v);
        }
    }
}
