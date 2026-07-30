use std::path::Path;

use serde_json::{Map, Value as JsonValue, json};

use crate::broker::Broker;
use crate::error::{Error, Result};
use crate::niri::NiriClient;
use crate::shell::ShellController;

use super::protocol::object;

pub const BUILTIN_METHODS: &[&str] = &[
    "weyriva.ping",
    "weyriva.info",
    "weyriva.methods",
    "weyriva.niri.outputs",
    "weyriva.niri.windows",
    "weyriva.launcher.open",
    "weyriva.notifications.dismiss_all",
    "weyriva.notifications.dnd",
    "weyriva.panel.toggle",
    "weyriva.panel.reload",
    "weyriva.plugin.v1.source.list",
    "weyriva.plugin.v1.source.add",
    "weyriva.plugin.v1.source.remove",
    "weyriva.plugin.v1.install",
    "weyriva.plugin.v1.status",
    "weyriva.plugin.v1.enable",
    "weyriva.plugin.v1.disable",
    "weyriva.plugin.v1.reload",
    "weyriva.plugin.v1.uninstall",
    "weyriva.plugin.v1.query",
    "weyriva.plugin.v1.activate",
    "weyriva.plugin.v1.ipc",
];

pub struct Dispatcher<'a> {
    broker: &'a mut Broker,
    shell: &'a ShellController,
    niri: &'a NiriClient,
}

impl<'a> Dispatcher<'a> {
    #[must_use]
    pub const fn new(
        broker: &'a mut Broker,
        shell: &'a ShellController,
        niri: &'a NiriClient,
    ) -> Self {
        Self {
            broker,
            shell,
            niri,
        }
    }

    /// Dispatches one validated control method.
    ///
    /// # Errors
    ///
    /// Returns structured method, parameter, runtime, or plugin errors.
    pub fn dispatch(&mut self, method: &str, params: &JsonValue) -> Result<JsonValue> {
        match method {
            "weyriva.ping" => {
                no_params(params, "ping")?;
                Ok(json!({"pong": true, "protocol": 1}))
            }
            "weyriva.info" => {
                no_params(params, "info")?;
                Ok(
                    json!({"name": "Weyriva Shell", "version": env!("CARGO_PKG_VERSION"), "protocol": 1}),
                )
            }
            "weyriva.methods" => {
                no_params(params, "methods")?;
                Ok(json!({"builtin": BUILTIN_METHODS, "plugin": []}))
            }
            "weyriva.niri.outputs" => {
                no_params(params, "niri.outputs")?;
                self.niri.json("outputs")
            }
            "weyriva.niri.windows" => {
                no_params(params, "niri.windows")?;
                self.niri.json("windows")
            }
            "weyriva.launcher.open" => {
                no_params(params, "launcher.open")?;
                self.shell.call("route", &["launcher"])
            }
            "weyriva.notifications.dismiss_all" => {
                no_params(params, "notifications.dismiss_all")?;
                self.shell.call("clearNotifications", &[])
            }
            "weyriva.notifications.dnd" => self.dnd(params),
            "weyriva.panel.toggle" => {
                no_params(params, "panel.toggle")?;
                self.shell.call("toggleBar", &[])
            }
            "weyriva.panel.reload" => {
                no_params(params, "panel.reload")?;
                self.shell.call("reload", &[])
            }
            _ if method.starts_with("weyriva.plugin.v1.") => self.plugin(method, params),
            _ => Err(Error::new(
                "method_not_found",
                format!("unknown method: {method}"),
            )),
        }
    }

    fn dnd(&self, params: &JsonValue) -> Result<JsonValue> {
        if params.is_null() || params.as_object().is_some_and(Map::is_empty) {
            return self.shell.call("toggleDnd", &[]);
        }
        let Some(object) = params.as_object() else {
            return Err(invalid_dnd());
        };
        if object.len() != 1 {
            return Err(invalid_dnd());
        }
        let Some(enabled) = object.get("enabled").and_then(JsonValue::as_bool) else {
            return Err(invalid_dnd());
        };
        self.shell
            .call("setDnd", &[if enabled { "true" } else { "false" }])
    }

    fn plugin(&mut self, method: &str, params: &JsonValue) -> Result<JsonValue> {
        let operation = method
            .strip_prefix("weyriva.plugin.v1.")
            .ok_or_else(|| Error::new("method_not_found", "unknown plugin method"))?;
        let object = object(params)?;
        let (required, optional): (&[&str], &[&str]) = match operation {
            "source.list" => (&[], &[]),
            "source.add" => (&["name", "path"], &[]),
            "source.remove" => (&["name"], &[]),
            "install" | "enable" | "disable" | "reload" | "uninstall" => (&["id"], &[]),
            "status" => (&[], &["id"]),
            "query" => (&["provider", "query"], &[]),
            "activate" => (&["provider", "result_id"], &[]),
            "ipc" => (&["entry", "event", "payload"], &[]),
            _ => {
                return Err(Error::new(
                    "method_not_found",
                    format!("unknown method: {method}"),
                ));
            }
        };
        validate_keys(object, required, optional)?;
        match operation {
            "source.list" => self.broker.source_list(),
            "source.add" => self
                .broker
                .source_add(string(object, "name")?, Path::new(string(object, "path")?)),
            "source.remove" => self.broker.source_remove(string(object, "name")?),
            "install" => self.broker.install(string(object, "id")?),
            "status" => serde_json::to_value(self.broker.status(optional_string(object, "id")?)?)
                .map_err(Into::into),
            "enable" => self.broker.enable(string(object, "id")?),
            "disable" => self.broker.disable(string(object, "id")?),
            "reload" => self.broker.reload(string(object, "id")?),
            "uninstall" => self.broker.uninstall(string(object, "id")?),
            "query" => self
                .broker
                .query(string(object, "provider")?, string(object, "query")?),
            "activate" => self
                .broker
                .activate(string(object, "provider")?, string(object, "result_id")?),
            "ipc" => self.broker.ipc(
                string(object, "entry")?,
                string(object, "event")?,
                object
                    .get("payload")
                    .ok_or_else(|| Error::new("invalid_params", "payload is required"))?,
            ),
            _ => Err(Error::new("method_not_found", "unknown plugin method")),
        }
    }
}

fn no_params(params: &JsonValue, method: &str) -> Result<()> {
    if params.is_null() || params.as_object().is_some_and(Map::is_empty) {
        Ok(())
    } else {
        Err(Error::new(
            "invalid_params",
            format!("{method} takes no parameters"),
        ))
    }
}

fn invalid_dnd() -> Error {
    Error::new(
        "invalid_params",
        "notifications.dnd takes no parameters or {\"enabled\": bool}",
    )
}

fn validate_keys(
    object: &Map<String, JsonValue>,
    required: &[&str],
    optional: &[&str],
) -> Result<()> {
    if required.iter().any(|key| !object.contains_key(*key))
        || object
            .keys()
            .any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        Err(Error::new(
            "invalid_params",
            "method parameters do not match",
        ))
    } else {
        Ok(())
    }
}

fn string<'a>(object: &'a Map<String, JsonValue>, key: &str) -> Result<&'a str> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| Error::new("invalid_params", format!("{key} must be a string")))
}

fn optional_string<'a>(object: &'a Map<String, JsonValue>, key: &str) -> Result<Option<&'a str>> {
    object.get(key).map_or(Ok(None), |value| {
        value
            .as_str()
            .map(Some)
            .ok_or_else(|| Error::new("invalid_params", format!("{key} must be a string")))
    })
}
