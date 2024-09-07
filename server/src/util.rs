pub trait WithOpt {
    fn with_opt<T, F>(self, opt: Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, T) -> Self,
        Self: Sized;
}

impl<T> WithOpt for T {
    fn with_opt<U, F>(self, opt: Option<U>, f: F) -> Self
    where
        F: FnOnce(Self, U) -> Self,
        Self: Sized,
    {
        match opt {
            Some(value) => f(self, value),
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_opt_some() {
        let opt = Some(2);
        let result = 3.with_opt(opt, |x, y| x + y);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_with_opt_none() {
        let opt: Option<i32> = None;
        let result = 3.with_opt(opt, |x, y| x + y);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_with_opt_string() {
        let opt = Some(String::from("world"));
        let result = String::from("hello ").with_opt(opt, |s1, s2| s1 + &s2);
        assert_eq!(result, String::from("hello world"));
    }
}
