use serde::Deserialize;

#[doc(hidden)]
pub fn deserialize_nested_option<'de, D, T>(
    deserializer: D,
) -> std::result::Result<std::option::Option<std::option::Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let opt = std::option::Option::<T>::deserialize(deserializer)?;
    std::result::Result::Ok(std::option::Option::Some(opt))
}
