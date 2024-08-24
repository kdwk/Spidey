use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

#[derive(Clone)]
pub struct Step<'a, Input, Output>(String, Arc<dyn Runnable<Input, Output> + 'a>);

impl<'a, Input, Output> Runnable<Input, Output> for Step<'a, Input, Output> {
    fn run(&self, arg: Input) -> Output {
        self.1.run(arg)
    }
}

impl<'a, Input, Output> Step<'a, Input, Output> {
    pub fn action<Closure>(alias: impl Display, closure: Closure) -> Self
    where
        Closure: Runnable<Input, Output> + 'a,
    {
        Self(alias.to_string(), Arc::new(closure))
    }
}

pub fn identity<Input>() -> impl Runnable<Input, Input> {
    |input: Input| input
}

#[derive(Clone)]
pub struct Recipe<'a, Ingredients, Outcome> {
    pub initial_step: Step<'a, Ingredients, Outcome>,
    pub steps: Vec<Step<'a, Outcome, Outcome>>,
}

impl<'a, Ingredients, Outcome> Debug for Recipe<'a, Ingredients, Outcome> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Step: {}{}",
            self.initial_step.0.clone() + " → ",
            self.steps
                .iter()
                .map(|step| step.0.clone() + " → ")
                .collect::<Vec<String>>()
                .concat()
        )
    }
}

impl<'a, Ingredients, Outcome> Recipe<'a, Ingredients, Outcome> {
    pub fn initially<Closure: Runnable<Ingredients, Outcome> + 'a>(
        alias: impl Display,
        action: Closure,
    ) -> Self {
        Self {
            initial_step: Step::action(alias.to_string(), action),
            steps: vec![],
        }
    }
    pub fn then<Closure: Runnable<Outcome, Outcome> + 'a>(
        mut self,
        alias: impl Display,
        action: Closure,
    ) -> Self {
        self.steps.push(Step::action(alias.to_string(), action));
        self
    }
    pub fn replace_first<Closure: Runnable<Outcome, Outcome> + 'a>(
        &mut self,
        alias: impl Display,
        action: Closure,
    ) -> &mut Self {
        let mut index = None;
        for (i, step) in &mut self.steps.iter().enumerate() {
            if step.0 == alias.to_string() {
                index = Some(i);
                break;
            }
        }
        if let Some(index) = index {
            self.steps[index] = Step::action(alias, action);
        }
        self
    }
    pub fn replace<Closure: Runnable<Outcome, Outcome> + 'a + Clone>(
        &mut self,
        alias: impl Display,
        action: Closure,
    ) -> &mut Self {
        let mut indices = vec![];
        for (i, step) in &mut self.steps.iter().enumerate() {
            if step.0 == alias.to_string() {
                indices.push(i);
            }
        }
        for index in indices {
            self.steps[index] = Step::action(alias.to_string().clone(), action.clone());
        }
        self
    }
    pub fn replace_initial<Closure: Runnable<Ingredients, Outcome> + 'a>(
        &mut self,
        action: Closure,
    ) -> &mut Self {
        self.initial_step = Step::action(self.initial_step.0.clone(), action);
        self
    }
    pub fn remove_first(&mut self, alias: impl Display) -> &mut Self {
        let mut index = None;
        for (i, step) in &mut self.steps.iter().enumerate() {
            if step.0 == alias.to_string() {
                index = Some(i);
                break;
            }
        }
        if let Some(index) = index {
            self.steps.remove(index);
        }
        self
    }
    pub fn remove(&mut self, alias: impl Display) -> &mut Self {
        let mut indices = vec![];
        for (i, step) in &mut self.steps.iter().enumerate() {
            if step.0 == alias.to_string() {
                indices.push(i);
            }
        }
        for index in indices {
            self.steps.remove(index);
        }
        self
    }
    pub fn get(&self, alias: impl Display) -> Step<'a, Outcome, Outcome>
    where
        Outcome: Clone + 'a,
    {
        let mut index = None;
        for (i, step) in &mut self.steps.iter().enumerate() {
            if step.0 == alias.to_string() {
                index = Some(i);
                break;
            }
        }
        if let Some(index) = index {
            self.steps[index].clone()
        } else {
            Step::action("identity", identity())
        }
    }
}

impl<'a, Ingredients, Outcome> Runnable<Ingredients, Outcome> for Recipe<'a, Ingredients, Outcome> {
    fn run(&self, ingredients: Ingredients) -> Outcome {
        let mut intermediate = self.initial_step.run(ingredients);
        for step in &self.steps {
            intermediate = step.run(intermediate);
        }
        intermediate
    }
}

pub trait Pipe<Closure, Return>
where
    Self: Sized,
    Closure: FnOnce(Self) -> Return,
{
    fn pipe(self, closure: Closure) -> Return;
}

impl<T, Closure, Return> Pipe<Closure, Return> for T
where
    Self: Sized,
    Closure: FnOnce(Self) -> Return,
{
    fn pipe(self, closure: Closure) -> Return {
        closure(self)
    }
}

pub trait Apply<Closure>
where
    Self: Sized,
    Closure: FnOnce(&mut Self),
{
    fn apply(self, closure: Closure) -> Self;
}

impl<T, Closure> Apply<Closure> for T
where
    Self: Sized,
    Closure: FnOnce(&mut Self),
{
    fn apply(mut self, closure: Closure) -> Self {
        closure(&mut self);
        self
    }
}

pub trait Log {
    fn log(self) -> Self;
}

impl<T> Log for T
where
    T: Debug,
{
    fn log(self) -> Self {
        println!("{self:?}");
        self
    }
}

pub trait ELog {
    fn elog(self) -> Self;
}

impl<E: Error> ELog for E {
    fn elog(self) -> Self {
        eprintln!("{self}");
        self
    }
}

pub trait Runnable<Arg, Return> {
    fn run(&self, arg: Arg) -> Return;
}

impl<Closure, Arg, Return> Runnable<Arg, Return> for Closure
where
    Closure: Fn(Arg) -> Return,
{
    fn run(&self, arg: Arg) -> Return {
        self(arg)
    }
}

pub trait Pass<Arg: Clone> {
    fn pass(self, arg: Arg) -> impl Fn();
}

impl<Closure, Arg: Clone, Return> Pass<Arg> for Closure
where
    Closure: Fn(Arg) -> Return + Clone + 'static,
{
    fn pass(self, arg: Arg) -> impl Fn() {
        move || {
            _ = self.clone().run(arg.clone());
        }
    }
}

pub trait Discard {
    fn discard(self);
}

impl<T> Discard for T {
    fn discard(self) {
        _ = self;
    }
}

pub mod example {
    use std::fmt::{Debug, Display};

    use super::{identity, Apply, Discard, Log, Pipe, Recipe, Runnable};
    #[derive(Debug, PartialEq)]
    pub struct BoxInternal(i32, i32, f32);
    pub struct Boxy<'a>(Recipe<'a, (), BoxInternal>);
    impl<'a> Boxy<'a> {
        fn new() -> Self {
            Self(
                Recipe::initially("new", |_| BoxInternal(0, 0, 0.0))
                    .then("width", identity())
                    .then("height", identity())
                    .then("rotation", identity()),
            )
        }
        fn width(&mut self, value: i32) -> &mut Self {
            self.0.replace("width", move |mut b: BoxInternal| {
                b.0 = value;
                b
            });
            self
        }
        fn height(&mut self, value: i32) -> &mut Self {
            self.0.replace("height", move |mut b: BoxInternal| {
                b.1 = value;
                b
            });
            self
        }
        fn rotate(&mut self, degrees: f32) -> &mut Self {
            self.0.replace("rotation", move |mut b: BoxInternal| {
                b.2 = degrees;
                b
            });
            self
        }
    }
    impl<'a> Debug for Boxy<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let b = self.0.run(());
            f.pad(format!("{:?}", b).as_str())
        }
    }
    pub fn test() {
        Boxy::new().width(6).height(7).rotate(45.0).log().discard();
        Boxy::new()
            .width(6)
            .height(7)
            .log()
            .rotate(86.45)
            .log()
            .discard();
        Boxy::new()
            .0
            .replace("width", move |b: BoxInternal| b.apply(|b| b.0 = b.1 * 2))
            .log()
            .discard();
    }
}
