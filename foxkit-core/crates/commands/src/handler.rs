//! Command handlers

use crate::{CommandArgs, CommandResult, CommandError};
use std::any::Any;

/// Command handler trait
pub trait CommandHandler: Send + Sync {
    /// Execute command
    fn execute(&self, args: CommandArgs) -> CommandResult;

    /// Can execute (precondition check)
    fn can_execute(&self, args: &CommandArgs) -> bool {
        true
    }
}

/// Simple function-based handler
pub struct FnHandler<F>
where
    F: Fn(CommandArgs) -> CommandResult + Send + Sync,
{
    handler: F,
}

impl<F> FnHandler<F>
where
    F: Fn(CommandArgs) -> CommandResult + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> CommandHandler for FnHandler<F>
where
    F: Fn(CommandArgs) -> CommandResult + Send + Sync,
{
    fn execute(&self, args: CommandArgs) -> CommandResult {
        (self.handler)(args)
    }
}

/// Async command handler (for future use)
#[cfg(feature = "async")]
pub trait AsyncCommandHandler: Send + Sync {
    fn execute(&self, args: CommandArgs) -> impl std::future::Future<Output = CommandResult> + Send;
}

/// Handler that requires specific argument
pub struct RequiredArgHandler<T, F>
where
    T: for<'de> serde::Deserialize<'de>,
    F: Fn(T) -> CommandResult + Send + Sync,
{
    arg_name: String,
    handler: F,
    _marker: std::marker::PhantomData<T>,
}

impl<T, F> RequiredArgHandler<T, F>
where
    T: for<'de> serde::Deserialize<'de>,
    F: Fn(T) -> CommandResult + Send + Sync,
{
    pub fn new(arg_name: &str, handler: F) -> Self {
        Self {
            arg_name: arg_name.to_string(),
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T, F> CommandHandler for RequiredArgHandler<T, F>
where
    T: for<'de> serde::Deserialize<'de> + Send + Sync,
    F: Fn(T) -> CommandResult + Send + Sync,
{
    fn execute(&self, args: CommandArgs) -> CommandResult {
        match args.get::<T>(&self.arg_name) {
            Some(value) => (self.handler)(value),
            None => Err(CommandError::InvalidArgs(format!(
                "Missing required argument: {}",
                self.arg_name
            ))),
        }
    }

    fn can_execute(&self, args: &CommandArgs) -> bool {
        args.get::<T>(&self.arg_name).is_some()
    }
}

/// Handler with return value
pub fn returning<T: Any + Send + 'static>(value: T) -> CommandResult {
    Ok(Some(Box::new(value)))
}

/// Handler that returns nothing
pub fn ok() -> CommandResult {
    Ok(None)
}

/// Handler that fails
pub fn fail(message: &str) -> CommandResult {
    Err(CommandError::ExecutionFailed(message.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_handler() {
        let handler = FnHandler::new(|_| ok());
        assert!(handler.execute(CommandArgs::new()).is_ok());
    }

    #[test]
    fn test_returning() {
        let result = returning(42i32);
        let boxed = result.unwrap().unwrap();
        assert_eq!(*boxed.downcast::<i32>().unwrap(), 42);
    }
}
