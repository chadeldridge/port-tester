use log::LevelFilter;

#[derive(Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd)]
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

    pub fn to_filter_level(&self) -> LevelFilter {
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
