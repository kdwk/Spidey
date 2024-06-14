use std::{error::Error, fmt::Display};

#[derive(Debug, Clone, Copy)]
pub struct NoneError;

impl Error for NoneError {
    fn description(&self) -> &str {
        "NoneError: Expected Some(...), got None."
    }
}

impl Display for NoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("NoneError: expected Some(...), got None.")
    }
}

pub type Whoops = Result<(), Box<dyn Error>>;

pub trait IntoWhoops {
    fn into_whoops(self) -> Whoops;
}

impl IntoWhoops for () {
    fn into_whoops(self) -> Whoops {
        Ok(())
    }
}

impl IntoWhoops for Whoops {
    fn into_whoops(self) -> Whoops {
        self
    }
}

impl<T> IntoWhoops for Option<T> {
    fn into_whoops(self) -> Whoops {
        match self {
            Some(_) => Ok(()),
            None => Err(NoneError)?,
        }
    }
}

// pub fn attempt_fn<Closure, Arg, Return>(closure: Closure) -> Closure
// where
//     Closure: Fn(Arg) -> Return,
//     Return: IntoWhoops,
// {
//     closure
// }

pub fn attempt<Closure, Return>(closure: Closure) -> Whoops
where
    Closure: FnOnce() -> Return,
    Return: IntoWhoops,
{
    closure().into_whoops()
}

pub trait Catch {
    fn catch<HandleErrorClosure: Fn(Box<dyn Error>)>(self, closure: HandleErrorClosure);
}

impl Catch for Whoops {
    fn catch<HandleErrorClosure: Fn(Box<dyn Error>)>(self, closure: HandleErrorClosure) {
        match self {
            Ok(_) => {}
            Err(error) => closure(error),
        }
    }
}
