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

impl<T> IntoWhoops for Result<T, Box<dyn Error>> {
    fn into_whoops(self) -> Whoops {
        match self {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
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

pub fn attempt<Closure, Return>(closure: Closure) -> Whoops
where
    Closure: FnOnce() -> Return,
    Return: IntoWhoops,
{
    closure().into_whoops()
}

pub trait Catch {
    fn catch(self, closure: impl FnOnce(Box<dyn Error>));
}

impl Catch for Whoops {
    fn catch(self, closure: impl FnOnce(Box<dyn Error>)) {
        match self {
            Ok(_) => {}
            Err(error) => closure(error),
        }
    }
}
