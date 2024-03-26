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

pub fn attempt<Closure, Arg, Return>(closure: Closure) -> Closure
where
    Closure: Fn(Arg) -> Return,
    Return: IntoWhoops,
{
    closure
}

pub trait Catch<Arg> {
    fn catch<HandleErrorClosure, HandleErrorClosureReturn>(
        self,
        closure: HandleErrorClosure,
    ) -> impl Fn(Arg) -> Whoops
    where
        HandleErrorClosure: Fn(&Box<dyn Error>) -> HandleErrorClosureReturn,
        HandleErrorClosureReturn: IntoWhoops;
}

impl<Closure, Arg, Return> Catch<Arg> for Closure
where
    Closure: FnOnce(Arg) -> Return + Clone,
    Return: IntoWhoops,
{
    fn catch<HandleErrorClosure, HandleErrorClosureReturn>(
        self,
        closure: HandleErrorClosure,
    ) -> impl Fn(Arg) -> Whoops
    where
        HandleErrorClosure: Fn(&Box<dyn Error>) -> HandleErrorClosureReturn,
        HandleErrorClosureReturn: IntoWhoops,
    {
        move |arg| match self.clone()(arg).into_whoops() {
            Ok(_) => Ok(()),
            Err(error) => {
                closure(&error);
                Err(error)
            }
        }
    }
}
