use ibc_core_host_types::identifiers::{ChannelId, PortId};
use ibc_primitives::prelude::*;
use ibc_primitives::{Msg, Signer};
use ibc_proto::ibc::core::channel::v1::MsgChannelCloseInit as RawMsgChannelCloseInit;
use ibc_proto::Protobuf;

use crate::error::ChannelError;

pub const CHAN_CLOSE_INIT_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelCloseInit";

///
/// Message definition for the first step in the channel close handshake (`ChanCloseInit` datagram).
/// Per our convention, this message is sent to chain A.
///
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgChannelCloseInit {
    pub port_id_on_a: PortId,
    pub chan_id_on_a: ChannelId,
    pub signer: Signer,
}

impl Msg for MsgChannelCloseInit {
    type Raw = RawMsgChannelCloseInit;

    fn type_url(&self) -> String {
        CHAN_CLOSE_INIT_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgChannelCloseInit> for MsgChannelCloseInit {}

impl TryFrom<RawMsgChannelCloseInit> for MsgChannelCloseInit {
    type Error = ChannelError;

    fn try_from(raw_msg: RawMsgChannelCloseInit) -> Result<Self, Self::Error> {
        Ok(MsgChannelCloseInit {
            port_id_on_a: raw_msg.port_id.parse()?,
            chan_id_on_a: raw_msg.channel_id.parse()?,
            signer: raw_msg.signer.into(),
        })
    }
}

impl From<MsgChannelCloseInit> for RawMsgChannelCloseInit {
    fn from(domain_msg: MsgChannelCloseInit) -> Self {
        RawMsgChannelCloseInit {
            port_id: domain_msg.port_id_on_a.to_string(),
            channel_id: domain_msg.chan_id_on_a.to_string(),
            signer: domain_msg.signer.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ibc_primitives::prelude::*;
    use ibc_proto::ibc::core::channel::v1::MsgChannelCloseInit as RawMsgChannelCloseInit;
    use ibc_testkit::utils::core::channel::dummy_raw_msg_chan_close_init;

    use crate::msgs::chan_close_init::MsgChannelCloseInit;

    #[test]
    fn parse_channel_close_init_msg() {
        struct Test {
            name: String,
            raw: RawMsgChannelCloseInit,
            want_pass: bool,
        }

        let default_raw_msg = dummy_raw_msg_chan_close_init();

        let tests: Vec<Test> = vec![
            Test {
                name: "Good parameters".to_string(),
                raw: default_raw_msg.clone(),
                want_pass: true,
            },
            Test {
                name: "Correct port".to_string(),
                raw: RawMsgChannelCloseInit {
                    port_id: "p34".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Bad port, name too short".to_string(),
                raw: RawMsgChannelCloseInit {
                    port_id: "p".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad port, name too long".to_string(),
                raw: RawMsgChannelCloseInit {
                    port_id: "abcdefsdfasdfasdfasdfasdfasdfadsfasdgafsgadfasdfasdfasdfsdfasdfaghijklmnopqrstuabcdefsdfasdfasdfasdfasdfasdfadsfasdgafsgadfasdfasdfasdfsdfasdfaghijklmnopqrstu".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Correct channel identifier".to_string(),
                raw: RawMsgChannelCloseInit {
                    channel_id: "channel-34".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Bad channel, name too short".to_string(),
                raw: RawMsgChannelCloseInit {
                    channel_id: "chshort".to_string(),
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Bad channel, name too long".to_string(),
                raw: RawMsgChannelCloseInit {
                    channel_id: "channel-128391283791827398127398791283912837918273981273987912839".to_string(),
                    ..default_raw_msg
                },
                want_pass: false,
            },
        ]
            .into_iter()
            .collect();

        for test in tests {
            let msg = MsgChannelCloseInit::try_from(test.raw.clone());

            assert_eq!(
                test.want_pass,
                msg.is_ok(),
                "MsgChanCloseInit::try_from failed for test {}, \nmsg {:?} with error {:?}",
                test.name,
                test.raw,
                msg.err(),
            );
        }
    }

    #[test]
    fn to_and_from() {
        let raw = dummy_raw_msg_chan_close_init();
        let msg = MsgChannelCloseInit::try_from(raw.clone()).unwrap();
        let raw_back = RawMsgChannelCloseInit::from(msg.clone());
        let msg_back = MsgChannelCloseInit::try_from(raw_back.clone()).unwrap();
        assert_eq!(raw, raw_back);
        assert_eq!(msg, msg_back);
    }
}