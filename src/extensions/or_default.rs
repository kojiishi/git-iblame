use std::borrow::Cow;

pub trait OrDefault<U> {
    fn or_default(&self) -> U;
}

impl OrDefault<Cow<'static, str>> for Option<&String> {
    fn or_default(&self) -> Cow<'static, str> {
        (*self).map_or("".into(), |s| s.to_string().into())
    }
}

impl OrDefault<Cow<'static, str>> for Option<String> {
    fn or_default(&self) -> Cow<'static, str> {
        self.as_ref().or_default()
    }
}
