use std::ops::{Deref, DerefMut};

use serde_json::{Map, Value};
use zeroize::Zeroize;

pub(crate) struct SensitiveValue(pub(crate) Value);

impl SensitiveValue {
    pub(crate) fn take(&mut self) -> Value {
        std::mem::take(&mut self.0)
    }
}

impl Drop for SensitiveValue {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

pub(crate) struct SensitiveItems(pub(crate) Vec<Value>);

impl SensitiveItems {
    pub(crate) fn take(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.0)
    }
}

impl Deref for SensitiveItems {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SensitiveItems {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for SensitiveItems {
    fn drop(&mut self) {
        for value in &mut self.0 {
            wipe(value);
        }
    }
}

pub(crate) struct SensitiveMap(pub(crate) Map<String, Value>);

impl SensitiveMap {
    pub(crate) fn take(&mut self) -> Map<String, Value> {
        std::mem::take(&mut self.0)
    }
}

impl Drop for SensitiveMap {
    fn drop(&mut self) {
        wipe(&mut Value::Object(self.take()));
    }
}

fn wipe(value: &mut Value) {
    match std::mem::take(value) {
        Value::String(mut string) => {
            #[cfg(test)]
            WIPED_STRINGS.with(|count| count.set(count.get() + 1));
            string.zeroize();
        }
        Value::Array(mut values) => {
            for value in &mut values {
                wipe(value);
            }
        }
        Value::Object(values) => {
            for (mut key, mut value) in values {
                key.zeroize();
                wipe(&mut value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static WIPED_STRINGS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
