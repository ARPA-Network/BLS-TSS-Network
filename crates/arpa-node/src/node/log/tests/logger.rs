use core::fmt;
use log::{Level, Log, Metadata, Record, SetLoggerError};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use serde::{ser::Serialize, ser::SerializeMap, Serializer};

pub static SL: OnceCell<SimpleLogger> = OnceCell::new();

#[derive(Debug)]
pub struct SimpleLogger {
    log_level: Level,
    message: Mutex<Vec<String>>,
}

impl SimpleLogger {
    pub fn last_message(&self) -> Option<String> {
        self.message.lock().last().map(|s| s.to_owned())
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
            let mut message = self.message.lock();
            println!("{}", log);
            message.push(log);
        }
    }

    fn flush(&self) {}
}

pub fn init(log_level: Level) -> Result<(), SetLoggerError> {
    log::set_max_level(log::LevelFilter::Trace);
    let logger = SimpleLogger {
        log_level,
        message: Mutex::new(vec![]),
    };
    SL.set(logger).unwrap();
    log::set_logger(SL.get().unwrap())?;
    Ok(())
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
