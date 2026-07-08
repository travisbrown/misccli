use simplelog::LevelFilter;

fn select_log_level_filter(verbosity: u8) -> LevelFilter {
    match verbosity {
        0 => LevelFilter::Error,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

pub fn init(verbosity: u8) -> Result<(), log::SetLoggerError> {
    simplelog::TermLogger::init(
        select_log_level_filter(verbosity),
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_log_level_filter_maps_each_verbosity() {
        let expected = [
            (0, LevelFilter::Error),
            (1, LevelFilter::Error),
            (2, LevelFilter::Warn),
            (3, LevelFilter::Info),
            (4, LevelFilter::Debug),
            (5, LevelFilter::Trace),
            (u8::MAX, LevelFilter::Trace),
        ];

        for (verbosity, level) in expected {
            assert_eq!(select_log_level_filter(verbosity), level);
        }
    }
}
