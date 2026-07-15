use std::time::Duration;
use std::sync::Arc;

#[derive(Clone)]
pub struct MiddlewareChain {
    middlewares: Arc<Vec<Box<dyn Fn(&str) -> String + Send + Sync>>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self { middlewares: Arc::new(Vec::new()) }
    }

    pub fn add<F>(&mut self, middleware: F)
    where F: Fn(&str) -> String + Send + Sync + 'static {
        Arc::get_mut(&mut self.middlewares)
            .expect("Middleware chain already in use")
            .push(Box::new(middleware));
    }

    pub fn execute(&self, input: &str) -> String {
        let mut result = input.to_string();
        for middleware in self.middlewares.iter() {
            result = middleware(&result);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_chain() {
        let mut chain = MiddlewareChain::new();
        chain.add(|s| format!("prefix_{}", s));
        chain.add(|s| format!("{}", s.to_uppercase()));
        assert_eq!(chain.execute("test"), "prefix_TEST");
    }

    #[test]
    fn test_logging_middleware() {
        let mut chain = MiddlewareChain::new();
        chain.add(|s| {
            println!("[Middleware] Input length: {}", s.len());
            s.to_string()
        });
        let result = chain.execute("hello");
        assert_eq!(result, "hello");
    }
}