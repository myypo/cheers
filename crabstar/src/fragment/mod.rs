pub mod suspense;

pub trait Fragment {
    type Signals;

    fn signals() -> Self::Signals;
}

impl<T> Fragment for Option<T>
where
    T: Fragment,
{
    type Signals = Option<T::Signals>;

    fn signals() -> Self::Signals {
        Some(T::signals())
    }
}
