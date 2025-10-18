use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::ops::Deref;

pub trait NestedSignal: for<'de> Deserialize<'de> + Serialize {
    type Id: Serialize + for<'de> Deserialize<'de>;

    fn id(&self) -> &Self::Id;
    fn id_field_name() -> &'static str;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Nested<T: NestedSignal> {
    value: Vec<T>,
}

impl<T: NestedSignal> Serialize for Nested<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.value.len()))?;
        for item in &self.value {
            map.serialize_entry(&item.id(), &item)?;
        }
        map.end()
    }
}

impl<'de, T: NestedSignal> Deserialize<'de> for Nested<T>
where
    T::Id: Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NestedVisitor<T: NestedSignal>(std::marker::PhantomData<T>);

        impl<'de, T: NestedSignal> Visitor<'de> for NestedVisitor<T> {
            type Value = Nested<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a signal map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut value = Vec::with_capacity(map.size_hint().unwrap_or(0));

                while let Some((id, mut signal_value)) = map.next_entry::<T::Id, Value>()? {
                    if let Value::Object(ref mut obj) = signal_value {
                        obj.insert(
                            T::id_field_name().to_owned(),
                            serde_json::to_value(&id).map_err(serde::de::Error::custom)?,
                        );
                    }

                    let signal: T =
                        serde_json::from_value(signal_value).map_err(serde::de::Error::custom)?;
                    value.push(signal);
                }

                Ok(Nested { value })
            }
        }

        deserializer.deserialize_map(NestedVisitor(std::marker::PhantomData))
    }
}

impl<T: NestedSignal> From<Vec<T>> for Nested<T> {
    fn from(value: Vec<T>) -> Self {
        Self { value }
    }
}

impl<T: NestedSignal> Deref for Nested<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
