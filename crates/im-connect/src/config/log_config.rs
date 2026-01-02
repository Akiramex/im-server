use super::default_true;
use serde::Deserialize;
use time::{UtcOffset, macros::format_description};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::{self, time::OffsetTime};

const FORMAT_PRETTY: &str = "pretty";
const FORMAT_COMPACT: &str = "compact";
const FORMAT_JSON: &str = "json";
const FORMAT_FULL: &str = "full";

#[derive(Deserialize, Clone, Debug)]
pub struct LogConfig {
    #[serde(default = "default_filter_level")]
    pub filter_level: String,
    #[serde(default = "default_true")]
    pub with_ansi: bool,
    #[serde(default = "default_true")]
    pub stdout: bool,
    #[serde(default = "default_directory")]
    pub directory: String,
    #[serde(default = "default_file_name")]
    pub file_name: String,
    #[serde(default = "default_rolling")]
    pub rolling: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_true")]
    pub with_level: bool,
    #[serde(default = "default_true")]
    pub with_target: bool,
    #[serde(default = "default_true")]
    pub with_thread_ids: bool,
    #[serde(default = "default_true")]
    pub with_thread_names: bool,
    #[serde(default = "default_true")]
    pub with_source_location: bool,
}

fn default_filter_level() -> String {
    "info".to_string()
}

fn default_directory() -> String {
    "./log".to_string()
}

fn default_file_name() -> String {
    "app.log".to_string()
}

fn default_rolling() -> String {
    "daily".to_string()
}

fn default_format() -> String {
    FORMAT_FULL.to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            filter_level: default_filter_level(),
            with_ansi: true,
            stdout: false,
            directory: default_directory(),
            file_name: default_file_name(),
            rolling: default_rolling(),
            format: default_format(),
            with_level: true,
            with_target: true,
            with_thread_ids: true,
            with_thread_names: true,
            with_source_location: true,
        }
    }
}

#[allow(dead_code)]
impl LogConfig {
    pub fn filter_level(mut self, filter_level: &str) -> Self {
        self.filter_level = filter_level.to_owned();
        self
    }

    pub fn with_ansi(mut self, with_ansi: bool) -> Self {
        self.with_ansi = with_ansi;
        self
    }

    pub fn stdout(mut self, stdout: bool) -> Self {
        self.stdout = stdout;
        self
    }

    pub fn directory(mut self, directory: impl Into<String>) -> Self {
        self.directory = directory.into();
        self
    }

    pub fn file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = file_name.into();
        self
    }

    /// Valid values: minutely | hourly | daily | never
    ///
    /// Will panic on other values.
    pub fn rolling(mut self, rolling: impl Into<String>) -> Self {
        let rolling = rolling.into();
        if !["minutely", "hourly", "daily", "never"].contains(&&*rolling) {
            panic!("Unknown rolling")
        }
        self.rolling = rolling;
        self
    }

    /// Valid values: pretty | compact | json | full
    ///
    /// Will panic on other values.
    pub fn format(mut self, format: impl Into<String>) -> Self {
        let format = format.into();
        if format != FORMAT_PRETTY
            && format != FORMAT_COMPACT
            && format != FORMAT_JSON
            && format != FORMAT_FULL
        {
            panic!("Unknown format")
        }
        self.format = format;
        self
    }

    pub fn with_level(mut self, with_level: bool) -> Self {
        self.with_level = with_level;
        self
    }

    pub fn with_target(mut self, with_target: bool) -> Self {
        self.with_target = with_target;
        self
    }

    pub fn with_thread_ids(mut self, with_thread_ids: bool) -> Self {
        self.with_thread_ids = with_thread_ids;
        self
    }

    pub fn with_thread_names(mut self, with_thread_names: bool) -> Self {
        self.with_thread_names = with_thread_names;
        self
    }

    pub fn with_source_location(mut self, with_source_location: bool) -> Self {
        self.with_source_location = with_source_location;
        self
    }

    pub fn guard(&self) -> WorkerGuard {
        let file_appender = match self.rolling.as_str() {
            "minutely" => tracing_appender::rolling::minutely(&self.directory, &self.file_name),
            "hourly" => tracing_appender::rolling::hourly(&self.directory, &self.file_name),
            "daily" => tracing_appender::rolling::daily(&self.directory, &self.file_name),
            "never" => tracing_appender::rolling::never(&self.directory, &self.file_name),
            _ => tracing_appender::rolling::never(&self.directory, &self.file_name),
        };

        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

        let offset = UtcOffset::from_hms(8, 0, 0).expect("error parsing offset");
        let timer = OffsetTime::new(
            offset,
            format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
            ),
        );

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or(tracing_subscriber::EnvFilter::new(&self.filter_level)),
            )
            //.with_timer(timer) disabled
            .with_ansi(self.with_ansi);

        if self.format == FORMAT_PRETTY {
            let subscriber = subscriber
                .event_format(
                    fmt::format()
                        .pretty()
                        .with_level(self.with_level)
                        .with_target(self.with_target)
                        .with_thread_ids(self.with_thread_ids)
                        .with_thread_names(self.with_thread_names)
                        .with_source_location(self.with_source_location),
                )
                .with_timer(timer);
            if self.stdout {
                subscriber.with_writer(std::io::stdout).init();
            } else {
                subscriber.with_writer(file_writer).init();
            }
        } else if self.format == FORMAT_COMPACT {
            let subscriber = subscriber
                .event_format(
                    fmt::format()
                        .compact()
                        .with_level(self.with_level)
                        .with_target(self.with_target)
                        .with_thread_ids(self.with_thread_ids)
                        .with_thread_names(self.with_thread_names)
                        .with_source_location(self.with_source_location),
                )
                .with_timer(timer);
            if self.stdout {
                subscriber.with_writer(std::io::stdout).init();
            } else {
                subscriber.with_writer(file_writer).init();
            }
        } else if self.format == FORMAT_JSON {
            let subscriber = subscriber
                .event_format(
                    fmt::format()
                        .json()
                        .with_level(self.with_level)
                        .with_target(self.with_target)
                        .with_thread_ids(self.with_thread_ids)
                        .with_thread_names(self.with_thread_names)
                        .with_source_location(self.with_source_location),
                )
                .with_timer(timer);
            if self.stdout {
                subscriber.with_writer(std::io::stdout).init();
            } else {
                subscriber.with_writer(file_writer).init();
            }
        } else if self.format == FORMAT_FULL {
            let subscriber = subscriber
                .event_format(
                    fmt::format()
                        .with_level(self.with_level)
                        .with_target(self.with_target)
                        .with_thread_ids(self.with_thread_ids)
                        .with_thread_names(self.with_thread_names)
                        .with_source_location(self.with_source_location),
                )
                .with_timer(timer);
            if self.stdout {
                subscriber.with_writer(std::io::stdout).init();
            } else {
                subscriber.with_writer(file_writer).init();
            }
        }

        guard
    }
}
