use crate::{Generator, ValueResult, GeneratorResult};

pub struct Filter<Gen, Pred>
{
    generator: Gen,
    predicate: Pred,
}

impl<Gen, Pred> Filter<Gen, Pred>
where
    Gen: Generator,
    Pred: FnMut(&Gen::Output) -> bool
{
    pub fn new(generator: Gen, predicate: Pred) -> Self {
        Self {
            generator,
            predicate
        }
    }
}

impl<Gen, Pred> Generator for Filter<Gen, Pred>
where
    Gen: Generator,
    Pred: FnMut(&Gen::Output) -> bool
{
    type Output = Gen::Output;

    #[inline]
    fn run(&mut self, mut output: impl FnMut(Self::Output) -> ValueResult) -> GeneratorResult {
        let (generator, predicate) = (&mut self.generator, &mut self.predicate);
        generator.run(|x| {
            if predicate(&x) {
                output(x)
            }
            else {
                ValueResult::MoreValues
            }
        })
    }
}