use mail_decoder::Mail;
use serde_json::Value;

pub fn detect_time(mail: &Mail) -> Option<i64> {
    for section in &mail.sections {
        if let Some(obj) = section.as_object()
            && let Some(Value::Number(n)) = obj.get("time")
            && let Some(i) = n.as_i64()
        {
            return Some(i);
        }
    }
    None
}
