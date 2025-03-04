use crate::structs::{Chain, Filter, Skip, Take, Map, Zip, Dedup, Flatten};
use crate::{Generator, GeneratorResult, ValueResult};

pub trait Sealed {}

impl<T> Sealed for T where T: Generator {}

/// Provides extension-methods for all generators.
///
/// This allows generators to be composed to new generators, or consumed.
///
/// ## Example
/// ```
/// use pushgen::{SliceGenerator, GeneratorExt};
/// let data = [1, 2, 3, 4];
/// let mut output: Vec<i32> = Vec::new();
/// SliceGenerator::new(&data).map(|x| x*3).for_each(|x| output.push(x));
/// assert_eq!(output, [3,6,9,12]);
/// ```
pub trait GeneratorExt: Sealed + Generator {
    /// Creates a generator by chaining two generators, running them one after the other.
    ///
    /// ## Example
    /// ```
    /// use pushgen::{SliceGenerator, GeneratorExt};
    /// let data = [1, 2, 3];
    /// let mut output: Vec<i32> = Vec::new();
    /// SliceGenerator::new(&data).chain(SliceGenerator::new(&data)).for_each(|x| output.push(*x));
    /// assert_eq!(output, [1, 2, 3, 1, 2, 3]);
    /// ```
    fn chain<Gen>(self, other: Gen) -> Chain<Self, Gen>
    where
        Self: Sized,
        Gen: Generator<Output = Self::Output>,
    {
        Chain::new(self, other)
    }

    /// Create a filtered generator. Only values for which the predicate returns true will be passed on.
    ///
    /// The predicate must implement `FnMut(&Gen::Output) -> bool`.
    ///
    /// ## Example
    /// ```
    /// # use pushgen::*;
    /// let input = [1,2,3,4];
    /// let mut output: Vec<i32> = Vec::new();
    /// let run_result = SliceGenerator::new(&input).filter(|x| *x % 2 == 0).for_each(|x| output.push(*x));
    /// assert_eq!(run_result, GeneratorResult::Complete);
    /// assert_eq!(output, [2,4]);
    /// ```
    fn filter<Pred>(self, predicate: Pred) -> Filter<Self, Pred>
    where
        Self: Sized,
        Pred: FnMut(&Self::Output) -> bool,
    {
        Filter::new(self, predicate)
    }

    /// Takes a closure and creates a generator which  calls the closure on each value.
    ///
    /// ## Example
    /// ```
    /// use pushgen::{SliceGenerator, GeneratorExt};
    /// let data = [1, 2, 3];
    /// let mut output: Vec<String> = Vec::new();
    /// SliceGenerator::new(&data).map(|x| x.to_string()).for_each(|x| output.push(x));
    /// assert_eq!(output, ["1", "2", "3"]);
    /// ```
    fn map<Trans, Out>(self, transform_fn: Trans) -> Map<Self, Trans, Out>
    where
        Self: Sized,
        Trans: FnMut(Self::Output) -> Out,
    {
        Map::new(self, transform_fn)
    }

    /// Skips over `n` values, consuming and ignoring them.
    ///
    /// ## Example
    ///```
    /// # use pushgen::{GeneratorExt, SliceGenerator};
    /// # use pushgen::structs::Skip;
    /// let input = [1,2,3,4];
    /// let mut skipped_generator = SliceGenerator::new(&input).skip(2);
    /// let mut output: Vec<i32> = Vec::new();
    /// skipped_generator.for_each(|x| output.push(*x));
    /// assert_eq!(output, [3,4]);
    /// ```
    fn skip(self, n: usize) -> Skip<Self>
    where
        Self: Sized,
    {
        Skip::new(self, n)
    }

    /// Takes `n` values and then completes the generator.
    ///
    /// ## Example
    /// ```
    /// use pushgen::{SliceGenerator, GeneratorExt};
    /// let data = [1, 2, 3, 4];
    /// let mut output: Vec<i32> = Vec::new();
    /// SliceGenerator::new(&data).take(2).for_each(|x| output.push(*x));
    /// assert_eq!(output, [1, 2]);
    /// ```
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take::new(self, n)
    }

    /// Run a generator to completion, or until it is stopped, and call a closure for each value
    /// produced by the generator.
    ///
    /// The closure will be called for as long as the generator produces values, it is not possible
    /// to abort processing early. If early abort is needed, use [`Generator::run`](crate::Generator::run)
    /// ## Example
    /// ```
    /// # use pushgen::{GeneratorExt, GeneratorResult, SliceGenerator};
    /// let mut sum = 0i32;
    /// let data = [1,2,3];
    /// let result = SliceGenerator::new(&data).for_each(|x| sum += x);
    /// assert_eq!(sum, 6);
    /// assert_eq!(result, GeneratorResult::Complete);
    /// ```
    #[inline]
    fn for_each<Func>(&mut self, mut func: Func) -> GeneratorResult
    where
        Self: Sized,
        Func: FnMut(Self::Output)
    {
        self.run(move |value| {
            func(value);
            ValueResult::MoreValues
        })
    }

    /// Creates a generator that flattens nested structures using a generator adaptor.
    ///
    /// The adaptor closure must implement `FnMut(Self::Output) -> impl Generator`.
    ///
    /// This is useful if you have a range of ranges, and you want to remove on or several levels
    /// of indirection.
    ///
    /// ## Example
    /// ```
    /// use pushgen::{SliceGenerator, GeneratorExt};
    /// let data = vec![vec![1, 2, 3, 4], vec![5, 6]];
    /// let mut flatted = Vec::new();
    /// SliceGenerator::new(data.as_slice())
    ///     .flatten(|x| SliceGenerator::new(x.as_slice()))
    ///     .for_each(|x| flatted.push(*x));
    /// assert_eq!(flatted, [1, 2, 3, 4, 5, 6]);
    /// ```
    fn flatten<AdaptorFn, Adaptor>(self, adaptor: AdaptorFn) -> Flatten<Self, AdaptorFn, Adaptor>
    where
        Self: Sized,
        Adaptor: Generator,
        AdaptorFn: FnMut(Self::Output) -> Adaptor {
        Flatten::new(self, adaptor)
    }

    /// Zips the output of two generators into a single generator of pairs.
    ///
    /// `zip()` returns a new generator that will use values from two generators, outputting
    /// a tuple where the first element comes from the first generator, and the second element comes
    /// from the second generator.
    ///
    /// The zip generator will complete when either generator completes.
    ///
    /// ## Example
    /// ```
    /// use pushgen::{SliceGenerator, GeneratorExt};
    /// let left = [1, 2, 3];
    /// let right = [4, 5, 6];
    /// let mut output: Vec<(i32, i32)> = Vec::new();
    /// SliceGenerator::new(&left).zip(SliceGenerator::new(&right)).for_each(|(a, b)| output.push((*a, *b)));
    /// assert_eq!(output, [(1,4), (2, 5), (3, 6)]);
    /// ```
    #[inline]
    fn zip<Right>(self, right: Right) -> Zip<Self, Right>
    where
        Self: Sized,
        Right: Generator,
    {
        Zip::new(self, right)
    }

    /// Create a de-duplicating generator, removing consecutive duplicate values.
    ///
    /// Values will be made available when a non-duplicate is detected. If the up-stream generator generates
    /// the following sequence: `[1, 2, 3, 3, 4]` then the value `1` will be generated from the
    /// `Dedup` generator once the value `2` has been generated by the upstream generator and so
    /// on.
    ///
    /// | Upstream value | Dedup-generated value |
    /// |----------------|-----------------------|
    /// | 1              | *None*                |
    /// | 2              | 1                     |
    /// | 3              | 2                     |
    /// | 3              | *Ignored*             |
    /// | 4              | 3                     |
    /// | *Complete*     | 4                     |
    /// | *Complete*     | *Complete*            |
    ///
    /// ## Example
    /// ```
    /// # use pushgen::{SliceGenerator, GeneratorExt};
    /// let data = [1, 2, 3, 3, 3, 3, 4, 3];
    /// let mut output: Vec<i32> = Vec::new();
    /// SliceGenerator::new(&data).dedup().for_each(|x| output.push(*x));
    /// assert_eq!(output, [1, 2, 3, 4, 3]);
    /// ```
    #[inline]
    fn dedup(self) -> Dedup<Self>
    where
        Self: Sized,
        Self::Output: PartialEq,
    {
        Dedup::new(self)
    }
}

impl<T: Generator> GeneratorExt for T {}

#[cfg(test)]
mod tests
{
    use crate::{Generator, ValueResult, GeneratorResult, GeneratorExt};

    #[test]
    fn for_each_stopped()
    {
        struct StoppingGen;
        impl Generator for StoppingGen
        {
            type Output = i32;

            fn run(&mut self, _output: impl FnMut(Self::Output) -> ValueResult) -> GeneratorResult {
                GeneratorResult::Stopped
            }
        }

        let mut gen = StoppingGen;
        assert_eq!(gen.for_each(|_| ()), GeneratorResult::Stopped);
    }
}
