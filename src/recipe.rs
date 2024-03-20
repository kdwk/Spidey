use std::any::Any;

pub trait Run<Arg> {
    fn run(self, arg: Arg);
}

impl<Closure, Arg, Return> Run<Arg> for Closure
where
    Closure: FnOnce(Arg) -> Return,
{
    fn run(self, arg: Arg) {
        _ = self(arg);
    }
}

pub trait Replicate<Arg: Clone> {
    fn replicate(self, arg: Arg) -> impl FnMut();
}

impl<Closure, Arg: Clone, Return> Replicate<Arg> for Closure
where
    Closure: FnOnce(Arg) -> Return + Clone,
{
    fn replicate(self, arg: Arg) -> impl FnMut() {
        move || {
            _ = self.clone().run(arg.clone());
        }
    }
}

pub trait Discard {
    fn discard(self);
}

impl<T> Discard for T
where
    T: Any,
{
    fn discard(self) {}
}
