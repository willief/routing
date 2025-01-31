// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

/// Maximum allowed length for a [message's `body`](struct.MpidMessage.html#method.new) (101,760
/// bytes).
pub const MAX_BODY_SIZE: usize = 102_400 - 512 - super::MAX_HEADER_METADATA_SIZE;

use super::{Error, MpidHeader};
use crate::xor_name::XorName;
use hex_fmt::HexFmt;
use maidsafe_utilities::serialisation::serialise;
use safe_crypto::{PublicSignKey, SecretSignKey, Signature};
use std::fmt::{self, Debug, Formatter};

#[derive(PartialEq, Eq, Hash, Clone, Deserialize, Serialize)]
struct Detail {
    recipient: XorName,
    body: Vec<u8>,
}

/// A full message including header and body which can be sent to or retrieved from the network.
#[derive(PartialEq, Eq, Hash, Clone, Deserialize, Serialize)]
pub struct MpidMessage {
    header: MpidHeader,
    detail: Detail,
    signature: Signature,
}

impl MpidMessage {
    /// Constructor.
    ///
    /// `sender` and `metadata` are used to construct an `MpidHeader` member, accessed via the
    /// [`header()`](#method.header) getter.  For details on these arguments, see
    /// [MpidHeader::new()](struct.MpidHeader.html#method.new).
    ///
    /// `recipient` represents the name of the intended receiver of the message.
    ///
    /// `body` is arbitrary, user-supplied data representing the main portion of the message.  It
    /// must not exceed [`MAX_BODY_SIZE`](constant.MAX_BODY_SIZE.html).  It can be empty if desired.
    ///
    /// An error will be returned if `body` exceeds `MAX_BODY_SIZE`, if
    /// [MpidHeader::new()](struct.MpidHeader.html#method.new) fails or if
    /// serialisation during the signing process fails.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        sender: XorName,
        metadata: Vec<u8>,
        recipient: XorName,
        body: Vec<u8>,
        secret_key: &SecretSignKey,
    ) -> Result<MpidMessage, Error> {
        if body.len() > MAX_BODY_SIZE {
            return Err(Error::BodyTooLarge);
        }

        let header = MpidHeader::new(sender, metadata, secret_key)?;

        let detail = Detail {
            recipient: recipient,
            body: body,
        };

        let recipient_and_body = serialise(&detail)?;
        Ok(MpidMessage {
            header: header,
            detail: detail,
            signature: secret_key.sign_detached(&recipient_and_body),
        })
    }

    /// Getter for `MpidHeader` member, created when calling `new()`.
    pub fn header(&self) -> &MpidHeader {
        &self.header
    }

    /// The name of the intended receiver of the message.
    pub fn recipient(&self) -> &XorName {
        &self.detail.recipient
    }

    /// Arbitrary, user-supplied data representing the main portion of the message.
    pub fn body(&self) -> &Vec<u8> {
        &self.detail.body
    }

    /// The name of the message, equivalent to the
    /// [`MpidHeader::name()`](../struct.MpidHeader.html#method.name).  As per that getter, this is
    /// relatively expensive, so its use should be minimised.
    pub fn name(&self) -> Result<XorName, Error> {
        self.header.name()
    }

    /// Validates the message and header signatures against the provided `PublicSignKey`.
    pub fn verify(&self, public_key: &PublicSignKey) -> bool {
        match serialise(&self.detail) {
            Ok(recipient_and_body) => {
                public_key.verify_detached(&self.signature, &recipient_and_body)
                    && self.header.verify(public_key)
            }
            Err(_) => false,
        }
    }
}

impl Debug for MpidMessage {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(
            formatter,
            "MpidMessage {{ header: {:?}, recipient: {:?}, body: {:.14}, signature: {:.14} }}",
            self.header,
            self.detail.recipient,
            HexFmt(&self.detail.body),
            HexFmt(&self.signature.into_bytes()[..])
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging;
    use crate::xor_name::XorName;
    use rand;
    use safe_crypto::gen_sign_keypair;
    use unwrap::unwrap;

    #[test]
    fn full() {
        let (public_key, secret_key) = gen_sign_keypair();
        let sender: XorName = rand::random();
        let metadata = messaging::generate_random_bytes(messaging::MAX_HEADER_METADATA_SIZE);
        let recipient: XorName = rand::random();

        // Check with body which is empty, then at size limit, then just above limit.
        {
            let message = unwrap!(MpidMessage::new(
                sender,
                metadata.clone(),
                recipient,
                vec![],
                &secret_key,
            ));
            assert!(message.body().is_empty());
        }
        let mut body = messaging::generate_random_bytes(MAX_BODY_SIZE);
        let message = unwrap!(MpidMessage::new(
            sender,
            metadata.clone(),
            recipient,
            body.clone(),
            &secret_key,
        ));
        assert_eq!(*message.body(), body);
        body.push(0);
        assert!(MpidMessage::new(
            sender,
            metadata.clone(),
            recipient,
            body.clone(),
            &secret_key,
        )
        .is_err());
        let _ = body.pop();

        // Check verify function with a valid and invalid key
        assert!(message.verify(&public_key));
        let (rand_public_key, _) = gen_sign_keypair();
        assert!(!message.verify(&rand_public_key));
    }
}
