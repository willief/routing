// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::xor_name::XorName;
use maidsafe_utilities::serialisation;
use safe_crypto;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Formatter};

/// Maximum allowed size for a serialised Immutable Data (ID) to grow to
pub const MAX_IMMUTABLE_DATA_SIZE_IN_BYTES: u64 = 1024 * 1024 + 10 * 1024;

/// An immutable chunk of data.
///
/// Note that the `name` member is omitted when serialising `ImmutableData` and is calculated from
/// the `value` when deserialising.
#[derive(Hash, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImmutableData {
    name: XorName,
    value: Vec<u8>,
}

impl ImmutableData {
    /// Creates a new instance of `ImmutableData`
    pub fn new(value: Vec<u8>) -> ImmutableData {
        ImmutableData {
            name: XorName(safe_crypto::hash(&value)),
            value: value,
        }
    }

    /// Returns the value
    pub fn value(&self) -> &Vec<u8> {
        &self.value
    }

    /// Returns name ensuring invariant.
    pub fn name(&self) -> &XorName {
        &self.name
    }

    /// Returns size of contained value.
    pub fn payload_size(&self) -> usize {
        self.value.len()
    }

    /// Returns size of this data after serialisation.
    pub fn serialised_size(&self) -> u64 {
        serialisation::serialised_size(self)
    }

    /// Return true if the size is valid
    pub fn validate_size(&self) -> bool {
        self.serialised_size() <= MAX_IMMUTABLE_DATA_SIZE_IN_BYTES
    }
}

impl Serialize for ImmutableData {
    fn serialize<S: Serializer>(&self, serialiser: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serialiser)
    }
}

impl<'de> Deserialize<'de> for ImmutableData {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<ImmutableData, D::Error> {
        let value: Vec<u8> = Deserialize::deserialize(deserializer)?;
        Ok(ImmutableData::new(value))
    }
}

impl Debug for ImmutableData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "ImmutableData {:?}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "mock_base"))]
    use hex::ToHex;
    use maidsafe_utilities::{serialisation, SeededRng};
    use rand::Rng;
    use unwrap::unwrap;

    #[cfg(not(feature = "mock_base"))]
    #[test]
    fn deterministic_test() {
        let value = "immutable data value".to_owned().into_bytes();
        let immutable_data = ImmutableData::new(value);
        let immutable_data_name = immutable_data.name().0.as_ref().to_hex();
        let expected_name = "fac2869677ee06277633c37ac7e8e5c655f3d652f707c7a79fab930d584a3016";

        assert_eq!(&expected_name, &immutable_data_name);
    }

    #[test]
    fn serialisation() {
        let mut rng = SeededRng::thread_rng();
        let len = rng.gen_range(1, 10_000);
        let value = rng.gen_iter().take(len).collect();
        let immutable_data = ImmutableData::new(value);
        let serialised = unwrap!(serialisation::serialise(&immutable_data));
        let parsed = unwrap!(serialisation::deserialise(&serialised));
        assert_eq!(immutable_data, parsed);
    }
}
