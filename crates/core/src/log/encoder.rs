//! An encoder which writes a JSON object adapted from log4rs.
//!
//! Each log event will be written as a JSON object on its own line.
//!
//! # Contents
//!
//! An example object (note that real output will not be pretty-printed):
//!
//! ```json
//! {
//!     "time": "2016-03-20T14:22:20.644420340-08:00",
//!     "message": "the log message",
//!     "module_path": "foo::bar",
//!     "file": "foo/bar/mod.rs",
//!     "line": 100,
//!     "level": "INFO",
//!     "target": "foo::bar",
//!     "thread": "main",
//!     "thread_id": 123,
//!     "mdc": {
//!         "request_id": "123e4567-e89b-12d3-a456-426655440000"
//!     }
//!     "node_info": {
//!         
//!     }
//! }
//! ```

use chrono::{
    format::{DelayedFormat, Fixed, Item},
    DateTime, Local,
};
use lazy_static::lazy_static;
use log::{Level, Record};
use log4rs::encode::{Encode, Write};
use parking_lot::RwLock;
use serde::ser::{self, Serialize, SerializeMap};
use std::{
    fmt::{self, Debug},
    option, thread,
};

lazy_static! {
    pub static ref CONTEXT_INFO: RwLock<Vec<String>> =
        RwLock::new(vec!["".to_string(), "".to_string()]);
}

/// An `Encode`r which writes a JSON object.
#[derive(Clone, Debug, Default)]
pub struct JsonEncoder {
    node_id: String,
    show_context: bool,
}

impl JsonEncoder {
    /// Returns a new `JsonEncoder` with a default configuration.
    pub fn new(node_id: String) -> Self {
        JsonEncoder {
            node_id,
            show_context: false,
        }
    }

    pub fn context_logging(mut self, show_context: bool) -> Self {
        self.show_context = show_context;
        self
    }
}

impl JsonEncoder {
    fn encode_inner(
        &self,
        w: &mut dyn Write,
        time: DateTime<Local>,
        record: &Record,
    ) -> anyhow::Result<()> {
        let thread = thread::current();
        let node_info = if self.show_context {
            CONTEXT_INFO.read()[0].clone()
        } else {
            "".to_string()
        };
        let group_info = if self.show_context {
            CONTEXT_INFO.read()[1].clone()
        } else {
            "".to_string()
        };

        let message = Message {
            time: time.format_with_items(Some(Item::Fixed(Fixed::RFC3339)).into_iter()),
            message: record.args(),
            level: record.level(),
            module_path: record.module_path(),
            file: record.file(),
            line: record.line(),
            target: record.target(),
            thread: thread.name(),
            thread_id: thread_id::get(),
            node_id: &self.node_id,
            mdc: Mdc,
            node_info: &node_info,
            group_info: &group_info,
        };
        message.serialize(&mut serde_json::Serializer::new(&mut *w))?;
        w.write_all("\n".as_bytes())?;
        Ok(())
    }
}

impl Encode for JsonEncoder {
    fn encode(&self, w: &mut dyn Write, record: &Record) -> anyhow::Result<()> {
        self.encode_inner(w, Local::now(), record)
    }
}

#[derive(serde::Serialize)]
struct Message<'a> {
    #[serde(serialize_with = "ser_display")]
    time: DelayedFormat<option::IntoIter<Item<'a>>>,
    #[serde(serialize_with = "ser_display")]
    message: &'a fmt::Arguments<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    level: Level,
    target: &'a str,
    thread: Option<&'a str>,
    thread_id: usize,
    node_id: &'a str,
    mdc: Mdc,
    node_info: &'a str,
    group_info: &'a str,
}

fn ser_display<T, S>(v: &T, s: S) -> Result<S::Ok, S::Error>
where
    T: fmt::Display,
    S: ser::Serializer,
{
    s.collect_str(v)
}

struct Mdc;

impl ser::Serialize for Mdc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
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

#[cfg(test)]
mod test {
    use super::*;
    use log::Level;
    use log4rs::encode::writer::simple::SimpleWriter;

    #[test]
    fn default() {
        let time = DateTime::parse_from_rfc3339("2016-03-20T14:22:20.644420340-08:00")
            .unwrap()
            .with_timezone(&Local);
        let level = Level::Debug;
        let target = "target";
        let module_path = "module_path";
        let file = "file";
        let line = 100;
        let message = "message";
        let thread = "log::encoder::test::default";
        let node_id = "test";
        log_mdc::insert("foo", "bar");

        let encoder = JsonEncoder::new(node_id.to_string());

        let mut buf = vec![];
        encoder
            .encode_inner(
                &mut SimpleWriter(&mut buf),
                time,
                &Record::builder()
                    .level(level)
                    .target(target)
                    .module_path(Some(module_path))
                    .file(Some(file))
                    .line(Some(line))
                    .args(format_args!("{}", message))
                    .build(),
            )
            .unwrap();

        let expected = format!(
            "{{\"time\":\"{}\",\"message\":\"{}\",\"module_path\":\"{}\",\
             \"file\":\"{}\",\"line\":{},\"level\":\"{}\",\"target\":\"{}\",\
             \"thread\":\"{}\",\"thread_id\":{},\"node_id\":\"{}\",\"mdc\":{{\"foo\":\"bar\"}},\
             \"node_info\":\"\",\"group_info\":\"\"}}",
            time.to_rfc3339(),
            message,
            module_path,
            file,
            line,
            level,
            target,
            thread,
            thread_id::get(),
            node_id
        );
        assert_eq!(expected, String::from_utf8(buf).unwrap().trim());
    }
}
