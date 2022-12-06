use core::fmt;
use log::{Level, Log, Metadata, Record, SetLoggerError};
use serde::{ser::Serialize, ser::SerializeMap, Serializer};
use std::sync::Mutex;

pub static SL: SimpleLogger = SimpleLogger {
    log_level: Level::Trace,
    message: Mutex::new(vec![]),
};

pub struct SimpleLogger {
    log_level: Level,
    message: Mutex<Vec<String>>,
}

impl SimpleLogger {
    pub fn last_message(&self) -> Option<String> {
        self.message.lock().unwrap().last().map(|s| s.to_owned())
    }
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = Message {
                message: record.args().to_string(),
                level: record.level(),
                target: record.target().to_string(),
                mdc: Mdc,
            };

            let log = serde_json::to_string(&message).unwrap();
            let mut message = self.message.lock().unwrap();
            println!("{}", log);
            message.push(log);
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<&'static SimpleLogger, SetLoggerError> {
    log::set_max_level(log::LevelFilter::Trace);
    log::set_logger(&SL)?;
    return Ok(&SL);
}

#[derive(serde::Serialize)]
struct Message {
    message: String,
    #[serde(serialize_with = "ser_display")]
    level: Level,
    target: String,
    mdc: Mdc,
}

fn ser_display<T, S>(v: &T, s: S) -> Result<S::Ok, S::Error>
where
    T: fmt::Display,
    S: Serializer,
{
    s.collect_str(v)
}

struct Mdc;

impl Serialize for Mdc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;

        let mut err = Ok(());
        log_mdc::iter(|k, v| {
            if let Ok(()) = err {
                err = map.serialize_key(k).and_then(|()| map.serialize_value(v));
            }
        });
        err?;

        map.end()
    }
}
