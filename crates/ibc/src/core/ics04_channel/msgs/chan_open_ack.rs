use crate::core::ics04_channel::error::ChannelError;
use crate::core::ics04_channel::Version;
use crate::core::ics23_commitment::commitment::CommitmentProofBytes;
use crate::core::ics24_host::identifier::{ChannelId, PortId};
use crate::signer::Signer;
use crate::tx_msg::Msg;
use crate::{prelude::*, Height};

use ibc_proto::ibc::core::channel::v1::MsgChannelOpenAck as RawMsgChannelOpenAck;
use ibc_proto::protobuf::Protobuf;

pub const TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenAck";

///
/// Per our convention, this message is sent to chain A.
/// Message definition for the third step in the channel open handshake (`ChanOpenAck` datagram).
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgChannelOpenAck {
    pub port_id_on_a: PortId,
    pub chan_id_on_a: ChannelId,
    pub chan_id_on_b: ChannelId,
    pub version_on_b: Version,
    pub proof_chan_end_on_b: CommitmentProofBytes,
    pub proof_height_on_b: Height,
    pub signer: Signer,
}

impl MsgChannelOpenAck {
    pub fn new(
        port_id_on_a: PortId,
        chan_id_on_a: ChannelId,
        chan_id_on_b: ChannelId,
        version_on_b: Version,
        proof_chan_end_on_b: CommitmentProofBytes,
        proof_height_on_b: Height,
        signer: Signer,
    ) -> Self {
        Self {
            port_id_on_a,
            chan_id_on_a,
            chan_id_on_b,
            version_on_b,
            proof_chan_end_on_b,
            proof_height_on_b,
            signer,
        }
    }
}

impl Msg for MsgChannelOpenAck {
    type ValidationError = ChannelError;
    type Raw = RawMsgChannelOpenAck;

    fn route(&self) -> String {
        crate::keys::ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgChannelOpenAck> for MsgChannelOpenAck {}

impl TryFrom<RawMsgChannelOpenAck> for MsgChannelOpenAck {
    type Error = ChannelError;

    fn try_from(raw_msg: RawMsgChannelOpenAck) -> Result<Self, Self::Error> {
        Ok(MsgChannelOpenAck {
            port_id_on_a: raw_msg.port_id.parse().map_err(ChannelError::Identifier)?,
            chan_id_on_a: raw_msg
                .channel_id
                .parse()
                .map_err(ChannelError::Identifier)?,
            chan_id_on_b: raw_msg
                .counterparty_channel_id
                .parse()
                .map_err(ChannelError::Identifier)?,
            version_on_b: raw_msg.counterparty_version.into(),
            proof_chan_end_on_b: raw_msg
                .proof_try
                .try_into()
                .map_err(ChannelError::InvalidProof)?,
            proof_height_on_b: raw_msg
                .proof_height
                .and_then(|raw_height| raw_height.try_into().ok())
                .ok_or(ChannelError::MissingHeight)?,
            signer: raw_msg.signer.parse().map_err(ChannelError::Signer)?,
        })
    }
}

impl From<MsgChannelOpenAck> for RawMsgChannelOpenAck {
    fn from(domain_msg: MsgChannelOpenAck) -> Self {
        RawMsgChannelOpenAck {
            port_id: domain_msg.port_id_on_a.to_string(),
            channel_id: domain_msg.chan_id_on_a.to_string(),
            counterparty_channel_id: domain_msg.chan_id_on_b.to_string(),
            counterparty_version: domain_msg.version_on_b.to_string(),
            proof_try: domain_msg.proof_chan_end_on_b.into(),
            proof_height: Some(domain_msg.proof_height_on_b.into()),
            signer: domain_msg.signer.to_string(),
        }
    }
}

#[cfg(test)]
pub mod test_util {
    use crate::prelude::*;
    use ibc_proto::ibc::core::channel::v1::MsgChannelOpenAck as RawMsgChannelOpenAck;

    use crate::core::ics24_host::identifier::{ChannelId, PortId};
    use crate::test_utils::{get_dummy_bech32_account, get_dummy_proof};
    use ibc_proto::ibc::core::client::v1::Height;

    /// Returns a dummy `RawMsgChannelOpenAck`, for testing only!
    pub fn get_dummy_raw_msg_chan_open_ack(proof_height: u64) -> RawMsgChannelOpenAck {
        RawMsgChannelOpenAck {
            port_id: PortId::default().to_string(),
            channel_id: ChannelId::default().to_string(),
            counterparty_channel_id: ChannelId::default().to_string(),
            counterparty_version: "".to_string(),
            proof_try: get_dummy_proof(),
            proof_height: Some(Height {
                revision_number: 0,
                revision_height: proof_height,
            }),
            signer: get_dummy_bech32_account(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use ibc_proto::ibc::core::channel::v1::MsgChannelOpenAck as RawMsgChannelOpenAck;
    use test_log::test;

    use crate::core::ics04_channel::msgs::chan_open_ack::test_util::get_dummy_raw_msg_chan_open_ack;
    use crate::core::ics04_channel::msgs::chan_open_ack::MsgChannelOpenAck;

    use ibc_proto::ibc::core::client::v1::Height;

    #[test]
    fn parse_channel_open_ack_msg() {
        struct Test {
            name: String,
            raw: RawMsgChannelOpenAck,
            want_pass: bool,
        }

        let proof_height = 20;
        let default_raw_msg = get_dummy_raw_msg_chan_open_ack(proof_height);

        let tests: Vec<Test> = vec![
            Test {
                name: "Good parameters".to_string(),
                raw: default_raw_msg.clone(),
                want_pass: true,
            },
            Test {
                name: "Correct port identifier".to_string(),
                raw: RawMsgChannelOpenAck {
                    port_id: "p34".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Bad port, name too short".to_string(),
                raw: RawMsgChannelOpenAck {
                    port_id: "p".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad port, name too long".to_string(),
                raw: RawMsgChannelOpenAck {
                    port_id: "abcdezdfDfsdfgfddsfsfdsdfdfvxcvzxcvsgdfsdfwefwvsdfdsfdasgagadgsadgsdffghijklmnopqrstuabcdezdfDfsdfgfddsfsfdsdfdfvxcvzxcvsgdfsdfwefwvsdfdsfdasgagadgsadgsdffghijklmnopqrstu".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Correct channel identifier".to_string(),
                raw: RawMsgChannelOpenAck {
                    channel_id: "channel-34".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Bad channel, name too short".to_string(),
                raw: RawMsgChannelOpenAck {
                    channel_id: "chshort".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad channel, name too long".to_string(),
                raw: RawMsgChannelOpenAck {
                    channel_id: "channel-128391283791827398127398791283912837918273981273987912839".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "[Counterparty] Correct channel identifier".to_string(),
                raw: RawMsgChannelOpenAck {
                    counterparty_channel_id: "channel-34".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "[Counterparty] Bad channel, name too short".to_string(),
                raw: RawMsgChannelOpenAck {
                    counterparty_channel_id: "chshort".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "[Counterparty] Bad channel, name too long".to_string(),
                raw: RawMsgChannelOpenAck {
                    counterparty_channel_id: "channel-128391283791827398127398791283912837918273981273987912839".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Empty counterparty version (allowed)".to_string(),
                raw: RawMsgChannelOpenAck {
                    counterparty_version: " ".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Arbitrary counterparty version (allowed)".to_string(),
                raw: RawMsgChannelOpenAck {
                    counterparty_version: "v1.1.23-alpha".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Bad proof height, height = 0".to_string(),
                raw: RawMsgChannelOpenAck {
                    proof_height: Some(Height {
                        revision_number: 0,
                        revision_height: 0,
                    }),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Missing proof height".to_string(),
                raw: RawMsgChannelOpenAck {
                    proof_height: None,
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Missing proof try (object proof)".to_string(),
                raw: RawMsgChannelOpenAck {
                    proof_try: Vec::new(),
                    ..default_raw_msg
                },
                want_pass: false,
            },
        ]
            .into_iter()
            .collect();

        for test in tests {
            let res_msg = MsgChannelOpenAck::try_from(test.raw.clone());

            assert_eq!(
                test.want_pass,
                res_msg.is_ok(),
                "MsgChanOpenAck::try_from raw failed for test {}, \nraw msg {:?} with error {:?}",
                test.name,
                test.raw,
                res_msg.err(),
            );
        }
    }

    #[test]
    fn to_and_from() {
        let raw = get_dummy_raw_msg_chan_open_ack(100);
        let msg = MsgChannelOpenAck::try_from(raw.clone()).unwrap();
        let raw_back = RawMsgChannelOpenAck::from(msg.clone());
        let msg_back = MsgChannelOpenAck::try_from(raw_back.clone()).unwrap();
        assert_eq!(raw, raw_back);
        assert_eq!(msg, msg_back);
    }
}
