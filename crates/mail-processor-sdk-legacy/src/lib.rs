pub mod chain;
pub mod error;
pub mod resolver;

pub use crate::chain::ResolverChain;
pub use crate::error::{BoxError, ResolverError};
pub use crate::resolver::Resolver;

#[cfg(test)]
mod tests {
    use super::{Resolver, ResolverChain};
    use std::convert::Infallible;
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    struct Push(&'static str);

    impl Resolver<(), Vec<&'static str>> for Push {
        type Error = Infallible;

        fn resolve(&self, _ctx: &(), output: &mut Vec<&'static str>) -> Result<(), Self::Error> {
            output.push(self.0);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct Fail;

    #[derive(Debug)]
    struct TestError;

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "fail")
        }
    }

    impl Error for TestError {}

    impl Resolver<(), ()> for Fail {
        type Error = TestError;

        fn resolve(&self, _ctx: &(), _output: &mut ()) -> Result<(), Self::Error> {
            Err(TestError)
        }
    }

    #[test]
    fn chain_applies_steps_in_order() {
        let chain = ResolverChain::new()
            .with(Push("first"))
            .with(Push("second"));
        let mut output = Vec::new();

        chain.apply(&(), &mut output).unwrap();

        assert_eq!(output, vec!["first", "second"]);
    }

    #[test]
    fn chain_uses_custom_step_name_in_errors() {
        let chain = ResolverChain::new().with_named("custom", Fail);
        let mut output = ();

        let err = chain.apply(&(), &mut output).unwrap_err();

        assert_eq!(err.step(), "custom");
        assert_eq!(err.to_string(), "resolver step 'custom' failed: fail");
        assert_eq!(err.source().unwrap().to_string(), "fail");
    }

    #[test]
    fn chain_uses_type_name_by_default() {
        let chain = ResolverChain::new().with(Fail);
        let mut output = ();

        let err = chain.apply(&(), &mut output).unwrap_err();

        assert_eq!(err.step(), std::any::type_name::<Fail>());
    }

    #[test]
    fn chain_push_methods_build_sequence() {
        let mut chain = ResolverChain::new();
        chain.push(Push("alpha")).push_named("beta", Push("beta"));
        let mut output = Vec::new();

        chain.apply(&(), &mut output).unwrap();

        assert_eq!(output, vec!["alpha", "beta"]);
    }
}
