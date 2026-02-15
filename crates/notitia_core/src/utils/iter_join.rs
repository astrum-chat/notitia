use std::fmt::Display;

pub trait Join {
    fn join(self, sep: &str) -> String;
}

impl<I, T> Join for I
where
    I: Iterator<Item = T>,
    T: Display,
{
    fn join(self, sep: &str) -> String {
        let mut iter = self;
        let mut result = String::new();

        if let Some(first) = iter.next() {
            result.push_str(&first.to_string());
            for item in iter {
                result.push_str(sep);
                result.push_str(&item.to_string());
            }
        }

        result
    }
}
