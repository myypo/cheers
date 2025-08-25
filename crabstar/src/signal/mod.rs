pub trait Signal {
    type Signals;

    fn signals() -> Self::Signals;
}

impl<T> Signal for Option<T>
where
    T: Signal,
{
    type Signals = Option<T::Signals>;

    fn signals() -> Self::Signals {
        Some(T::signals())
    }
}
