//! Implementation of a global context mock. Used in testing handlers of all IBC modules.

use alloc::fmt::Debug;
use core::time::Duration;

use basecoin_store::context::ProvableStore;
use basecoin_store::types::height::Height as StoreHeight;
use ibc::core::channel::types::channel::{ChannelEnd, IdentifiedChannelEnd};
use ibc::core::channel::types::commitment::{AcknowledgementCommitment, PacketCommitment};
use ibc::core::channel::types::error::{ChannelError, PacketError};
use ibc::core::channel::types::packet::{PacketState, Receipt};
use ibc::core::client::context::consensus_state::ConsensusState;
use ibc::core::client::types::error::ClientError;
use ibc::core::client::types::Height;
use ibc::core::commitment_types::commitment::CommitmentPrefix;
use ibc::core::connection::types::error::ConnectionError;
use ibc::core::connection::types::version::Version as ConnectionVersion;
use ibc::core::connection::types::{ConnectionEnd, IdentifiedConnectionEnd};
use ibc::core::handler::types::error::ContextError;
use ibc::core::handler::types::events::IbcEvent;
use ibc::core::host::types::identifiers::{ClientId, ConnectionId, Sequence};
use ibc::core::host::types::path::{
    AckPath, ChannelEndPath, ClientConnectionPath, ClientConsensusStatePath, ClientStatePath,
    CommitmentPath, ConnectionPath, NextChannelSequencePath, NextClientSequencePath,
    NextConnectionSequencePath, Path, ReceiptPath, SeqAckPath, SeqRecvPath, SeqSendPath,
};
use ibc::core::host::{ExecutionContext, ValidationContext};
use ibc::core::primitives::prelude::*;
use ibc::core::primitives::{Signer, Timestamp};
use ibc::primitives::proto::Any;
use ibc::primitives::ToVec;
use ibc_query::core::context::{ProvableContext, QueryContext};

use super::types::MockGenericContext;
use crate::hosts::{TestBlock, TestHeader, TestHost};
use crate::testapp::ibc::clients::{AnyClientState, AnyConsensusState};
use crate::testapp::ibc::utils::blocks_since;

impl<S, H> ValidationContext for MockGenericContext<S, H>
where
    S: ProvableStore + Debug,
    H: TestHost,
{
    type V = Self;
    type E = Self;
    type AnyConsensusState = AnyConsensusState;
    type AnyClientState = AnyClientState;

    fn client_state(&self, client_id: &ClientId) -> Result<Self::AnyClientState, ContextError> {
        Ok(self
            .ibc_store
            .client_state_store
            .get(StoreHeight::Pending, &ClientStatePath(client_id.clone()))
            .ok_or(ClientError::ClientStateNotFound {
                client_id: client_id.clone(),
            })?)
    }

    fn decode_client_state(&self, client_state: Any) -> Result<Self::AnyClientState, ContextError> {
        Ok(AnyClientState::try_from(client_state)?)
    }

    fn consensus_state(
        &self,
        client_cons_state_path: &ClientConsensusStatePath,
    ) -> Result<Self::AnyConsensusState, ContextError> {
        let height = Height::new(
            client_cons_state_path.revision_number,
            client_cons_state_path.revision_height,
        )
        .map_err(|_| ClientError::InvalidHeight)?;
        let consensus_state = self
            .ibc_store
            .consensus_state_store
            .get(StoreHeight::Pending, client_cons_state_path)
            .ok_or(ClientError::ConsensusStateNotFound {
                client_id: client_cons_state_path.client_id.clone(),
                height,
            })?;

        Ok(consensus_state)
    }

    fn host_height(&self) -> Result<Height, ContextError> {
        // TODO(rano): height sync with block and merkle tree
        Ok(self.history.last().expect("atleast one block").height())
    }

    fn host_timestamp(&self) -> Result<Timestamp, ContextError> {
        let host_height = self.host_height()?;
        let host_cons_state = self.host_consensus_state(&host_height)?;
        Ok(host_cons_state.timestamp())
    }

    fn host_consensus_state(
        &self,
        height: &Height,
    ) -> Result<Self::AnyConsensusState, ContextError> {
        // TODO(rano): height sync with block and merkle tree
        let height_delta = blocks_since(self.host_height().expect("no error"), *height)
            .expect("no error") as usize;

        let index = self
            .history
            .len()
            .checked_sub(1 + height_delta)
            .ok_or(ClientError::MissingLocalConsensusState { height: *height })?;

        let consensus_state = self.history[index]
            .clone()
            .into_header()
            .into_consensus_state()
            .into();
        Ok(consensus_state)
    }

    fn client_counter(&self) -> Result<u64, ContextError> {
        Ok(self
            .ibc_store
            .client_counter
            .get(StoreHeight::Pending, &NextClientSequencePath)
            .ok_or(ClientError::Other {
                description: "client counter not found".into(),
            })?)
    }

    fn connection_end(&self, conn_id: &ConnectionId) -> Result<ConnectionEnd, ContextError> {
        Ok(self
            .ibc_store
            .connection_end_store
            .get(StoreHeight::Pending, &ConnectionPath::new(conn_id))
            .ok_or(ConnectionError::ConnectionNotFound {
                connection_id: conn_id.clone(),
            })?)
    }

    fn validate_self_client(&self, _counterparty_client_state: Any) -> Result<(), ContextError> {
        Ok(())
    }

    fn commitment_prefix(&self) -> CommitmentPrefix {
        // this is prefix of ibc store
        // using default, as in our mock context, we don't store any other data
        CommitmentPrefix::default()
    }

    fn connection_counter(&self) -> Result<u64, ContextError> {
        Ok(self
            .ibc_store
            .conn_counter
            .get(StoreHeight::Pending, &NextConnectionSequencePath)
            .ok_or(ConnectionError::Other {
                description: "connection counter not found".into(),
            })?)
    }

    fn get_compatible_versions(&self) -> Vec<ConnectionVersion> {
        vec![ConnectionVersion::default()]
    }

    fn channel_end(&self, channel_end_path: &ChannelEndPath) -> Result<ChannelEnd, ContextError> {
        let channel_end = self
            .ibc_store
            .channel_end_store
            .get(
                StoreHeight::Pending,
                &ChannelEndPath::new(&channel_end_path.0, &channel_end_path.1),
            )
            .ok_or(ChannelError::MissingChannel)?;
        Ok(channel_end)
    }

    fn get_next_sequence_send(
        &self,
        seq_send_path: &SeqSendPath,
    ) -> Result<Sequence, ContextError> {
        let seq_send = self
            .ibc_store
            .send_sequence_store
            .get(
                StoreHeight::Pending,
                &SeqSendPath::new(&seq_send_path.0, &seq_send_path.1),
            )
            .ok_or(PacketError::ImplementationSpecific)?;
        Ok(seq_send)
    }

    fn get_next_sequence_recv(
        &self,
        seq_recv_path: &SeqRecvPath,
    ) -> Result<Sequence, ContextError> {
        let seq_recv = self
            .ibc_store
            .recv_sequence_store
            .get(
                StoreHeight::Pending,
                &SeqRecvPath::new(&seq_recv_path.0, &seq_recv_path.1),
            )
            .ok_or(PacketError::ImplementationSpecific)?;
        Ok(seq_recv)
    }

    fn get_next_sequence_ack(&self, seq_ack_path: &SeqAckPath) -> Result<Sequence, ContextError> {
        let seq_ack = self
            .ibc_store
            .ack_sequence_store
            .get(
                StoreHeight::Pending,
                &SeqAckPath::new(&seq_ack_path.0, &seq_ack_path.1),
            )
            .ok_or(PacketError::ImplementationSpecific)?;
        Ok(seq_ack)
    }

    fn get_packet_commitment(
        &self,
        commitment_path: &CommitmentPath,
    ) -> Result<PacketCommitment, ContextError> {
        let commitment = self
            .ibc_store
            .packet_commitment_store
            .get(
                StoreHeight::Pending,
                &CommitmentPath::new(
                    &commitment_path.port_id,
                    &commitment_path.channel_id,
                    commitment_path.sequence,
                ),
            )
            .ok_or(PacketError::ImplementationSpecific)?;
        Ok(commitment)
    }

    fn get_packet_receipt(&self, receipt_path: &ReceiptPath) -> Result<Receipt, ContextError> {
        let receipt = self
            .ibc_store
            .packet_receipt_store
            .is_path_set(
                StoreHeight::Pending,
                &ReceiptPath::new(
                    &receipt_path.port_id,
                    &receipt_path.channel_id,
                    receipt_path.sequence,
                ),
            )
            .then_some(Receipt::Ok)
            .ok_or(PacketError::PacketReceiptNotFound {
                sequence: receipt_path.sequence,
            })?;
        Ok(receipt)
    }

    fn get_packet_acknowledgement(
        &self,
        ack_path: &AckPath,
    ) -> Result<AcknowledgementCommitment, ContextError> {
        let ack = self
            .ibc_store
            .packet_ack_store
            .get(
                StoreHeight::Pending,
                &AckPath::new(&ack_path.port_id, &ack_path.channel_id, ack_path.sequence),
            )
            .ok_or(PacketError::PacketAcknowledgementNotFound {
                sequence: ack_path.sequence,
            })?;
        Ok(ack)
    }

    /// Returns a counter on the number of channel ids have been created thus far.
    /// The value of this counter should increase only via method
    /// `ChannelKeeper::increase_channel_counter`.
    fn channel_counter(&self) -> Result<u64, ContextError> {
        Ok(self
            .ibc_store
            .channel_counter
            .get(StoreHeight::Pending, &NextChannelSequencePath)
            .ok_or(ChannelError::Other {
                description: "channel counter not found".into(),
            })?)
    }

    /// Returns the maximum expected time per block
    fn max_expected_time_per_block(&self) -> Duration {
        self.block_time
    }

    fn validate_message_signer(&self, _signer: &Signer) -> Result<(), ContextError> {
        Ok(())
    }

    fn get_client_validation_context(&self) -> &Self::V {
        self
    }
}

/// Trait to provide proofs in gRPC service blanket implementations.
impl<S, H> ProvableContext for MockGenericContext<S, H>
where
    S: ProvableStore + Debug,
    H: TestHost,
{
    /// Returns the proof for the given [`Height`] and [`Path`]
    fn get_proof(&self, height: Height, path: &Path) -> Option<Vec<u8>> {
        self.ibc_store
            .store
            .get_proof(height.revision_height().into(), &path.to_string().into())
            .map(|p| p.to_vec())
    }
}

/// Trait to complete the gRPC service blanket implementations.
impl<S, H> QueryContext for MockGenericContext<S, H>
where
    S: ProvableStore + Debug,
    H: TestHost,
{
    /// Returns the list of all client states.
    fn client_states(&self) -> Result<Vec<(ClientId, Self::AnyClientState)>, ContextError> {
        let path = "clients".to_owned().into();

        self.ibc_store
            .client_state_store
            .get_keys(&path)
            .into_iter()
            .filter_map(|path| {
                if let Ok(Path::ClientState(client_path)) = path.try_into() {
                    Some(client_path)
                } else {
                    None
                }
            })
            .map(|client_state_path| {
                let client_state = self
                    .ibc_store
                    .client_state_store
                    .get(StoreHeight::Pending, &client_state_path)
                    .ok_or_else(|| ClientError::ClientStateNotFound {
                        client_id: client_state_path.0.clone(),
                    })?;
                Ok((client_state_path.0, client_state))
            })
            .collect()
    }

    /// Returns the list of all consensus states of the given client.
    fn consensus_states(
        &self,
        client_id: &ClientId,
    ) -> Result<Vec<(Height, Self::AnyConsensusState)>, ContextError> {
        let path = format!("clients/{}/consensusStates", client_id)
            .try_into()
            .map_err(|_| ClientError::Other {
                description: "Invalid consensus state path".into(),
            })?;

        self.ibc_store
            .consensus_state_store
            .get_keys(&path)
            .into_iter()
            .flat_map(|path| {
                if let Ok(Path::ClientConsensusState(consensus_path)) = path.try_into() {
                    Some(consensus_path)
                } else {
                    None
                }
            })
            .map(|consensus_path| {
                let height = Height::new(
                    consensus_path.revision_number,
                    consensus_path.revision_height,
                )?;
                let client_state = self
                    .ibc_store
                    .consensus_state_store
                    .get(StoreHeight::Pending, &consensus_path)
                    .ok_or({
                        ClientError::ConsensusStateNotFound {
                            client_id: consensus_path.client_id,
                            height,
                        }
                    })?;
                Ok((height, client_state))
            })
            .collect()
    }

    /// Returns the list of heights at which the consensus state of the given client was updated.
    fn consensus_state_heights(&self, client_id: &ClientId) -> Result<Vec<Height>, ContextError> {
        let path = format!("clients/{}/consensusStates", client_id)
            .try_into()
            .map_err(|_| ClientError::Other {
                description: "Invalid consensus state path".into(),
            })?;

        self.ibc_store
            .consensus_state_store
            .get_keys(&path)
            .into_iter()
            .flat_map(|path| {
                if let Ok(Path::ClientConsensusState(consensus_path)) = path.try_into() {
                    Some(consensus_path)
                } else {
                    None
                }
            })
            .map(|consensus_path| {
                Ok(Height::new(
                    consensus_path.revision_number,
                    consensus_path.revision_height,
                )?)
            })
            .collect::<Result<Vec<_>, _>>()
    }

    /// Connections queries all the IBC connections of a chain.
    fn connection_ends(&self) -> Result<Vec<IdentifiedConnectionEnd>, ContextError> {
        let path = "connections".to_owned().into();

        self.ibc_store
            .connection_end_store
            .get_keys(&path)
            .into_iter()
            .flat_map(|path| {
                if let Ok(Path::Connection(connection_path)) = path.try_into() {
                    Some(connection_path)
                } else {
                    None
                }
            })
            .map(|connection_path| {
                let connection_end = self
                    .ibc_store
                    .connection_end_store
                    .get(StoreHeight::Pending, &connection_path)
                    .ok_or_else(|| ConnectionError::ConnectionNotFound {
                        connection_id: connection_path.0.clone(),
                    })?;
                Ok(IdentifiedConnectionEnd {
                    connection_id: connection_path.0,
                    connection_end,
                })
            })
            .collect()
    }

    /// ClientConnections queries all the connection paths associated with a client.
    fn client_connection_ends(
        &self,
        client_id: &ClientId,
    ) -> Result<Vec<ConnectionId>, ContextError> {
        let client_connection_path = ClientConnectionPath::new(client_id.clone());

        Ok(self
            .ibc_store
            .connection_ids_store
            .get(StoreHeight::Pending, &client_connection_path)
            .unwrap_or_default())
    }

    /// Channels queries all the IBC channels of a chain.
    fn channel_ends(&self) -> Result<Vec<IdentifiedChannelEnd>, ContextError> {
        let path = "channelEnds".to_owned().into();

        self.ibc_store
            .channel_end_store
            .get_keys(&path)
            .into_iter()
            .flat_map(|path| {
                if let Ok(Path::ChannelEnd(channel_path)) = path.try_into() {
                    Some(channel_path)
                } else {
                    None
                }
            })
            .map(|channel_path| {
                let channel_end = self
                    .ibc_store
                    .channel_end_store
                    .get(StoreHeight::Pending, &channel_path)
                    .ok_or_else(|| ChannelError::ChannelNotFound {
                        port_id: channel_path.0.clone(),
                        channel_id: channel_path.1.clone(),
                    })?;
                Ok(IdentifiedChannelEnd {
                    port_id: channel_path.0,
                    channel_id: channel_path.1,
                    channel_end,
                })
            })
            .collect()
    }

    /// PacketCommitments returns all the packet commitments associated with a channel.
    fn packet_commitments(
        &self,
        channel_end_path: &ChannelEndPath,
    ) -> Result<Vec<PacketState>, ContextError> {
        let path = format!(
            "commitments/ports/{}/channels/{}/sequences",
            channel_end_path.0, channel_end_path.1
        )
        .try_into()
        .map_err(|_| PacketError::Other {
            description: "Invalid commitment path".into(),
        })?;

        self.ibc_store
            .packet_commitment_store
            .get_keys(&path)
            .into_iter()
            .flat_map(|path| {
                if let Ok(Path::Commitment(commitment_path)) = path.try_into() {
                    Some(commitment_path)
                } else {
                    None
                }
            })
            .filter(|commitment_path| {
                self.ibc_store
                    .packet_commitment_store
                    .get(StoreHeight::Pending, commitment_path)
                    .is_some()
            })
            .map(|commitment_path| {
                self.get_packet_commitment(&commitment_path)
                    .map(|packet| PacketState {
                        seq: commitment_path.sequence,
                        port_id: commitment_path.port_id,
                        chan_id: commitment_path.channel_id,
                        data: packet.as_ref().into(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    /// PacketAcknowledgements returns all the packet acknowledgements associated with a channel.
    /// Returns all the packet acknowledgements if sequences is empty.
    fn packet_acknowledgements(
        &self,
        channel_end_path: &ChannelEndPath,
        sequences: impl ExactSizeIterator<Item = Sequence>,
    ) -> Result<Vec<PacketState>, ContextError> {
        let collected_paths: Vec<_> = if sequences.len() == 0 {
            // if sequences is empty, return all the acks
            let ack_path_prefix = format!(
                "acks/ports/{}/channels/{}/sequences",
                channel_end_path.0, channel_end_path.1
            )
            .try_into()
            .map_err(|_| PacketError::Other {
                description: "Invalid ack path".into(),
            })?;

            self.ibc_store
                .packet_ack_store
                .get_keys(&ack_path_prefix)
                .into_iter()
                .flat_map(|path| {
                    if let Ok(Path::Ack(ack_path)) = path.try_into() {
                        Some(ack_path)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            sequences
                .into_iter()
                .map(|seq| AckPath::new(&channel_end_path.0, &channel_end_path.1, seq))
                .collect()
        };

        collected_paths
            .into_iter()
            .filter(|ack_path| {
                self.ibc_store
                    .packet_ack_store
                    .get(StoreHeight::Pending, ack_path)
                    .is_some()
            })
            .map(|ack_path| {
                self.get_packet_acknowledgement(&ack_path)
                    .map(|packet| PacketState {
                        seq: ack_path.sequence,
                        port_id: ack_path.port_id,
                        chan_id: ack_path.channel_id,
                        data: packet.as_ref().into(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    /// UnreceivedPackets returns all the unreceived IBC packets associated with
    /// a channel and sequences.
    fn unreceived_packets(
        &self,
        channel_end_path: &ChannelEndPath,
        sequences: impl ExactSizeIterator<Item = Sequence>,
    ) -> Result<Vec<Sequence>, ContextError> {
        // QUESTION. Currently only works for unordered channels; ordered channels
        // don't use receipts. However, ibc-go does it this way. Investigate if
        // this query only ever makes sense on unordered channels.

        Ok(sequences
            .into_iter()
            .map(|seq| ReceiptPath::new(&channel_end_path.0, &channel_end_path.1, seq))
            .filter(|receipt_path| {
                self.ibc_store
                    .packet_receipt_store
                    .get(StoreHeight::Pending, receipt_path)
                    .is_none()
            })
            .map(|receipts_path| receipts_path.sequence)
            .collect())
    }

    /// UnreceivedAcks returns all the unreceived IBC acknowledgements associated with a channel and sequences.
    /// Returns all the unreceived acks if sequences is empty.
    fn unreceived_acks(
        &self,
        channel_end_path: &ChannelEndPath,
        sequences: impl ExactSizeIterator<Item = Sequence>,
    ) -> Result<Vec<Sequence>, ContextError> {
        let collected_paths: Vec<_> = if sequences.len() == 0 {
            // if sequences is empty, return all the acks
            let commitment_path_prefix = format!(
                "commitments/ports/{}/channels/{}/sequences",
                channel_end_path.0, channel_end_path.1
            )
            .try_into()
            .map_err(|_| PacketError::Other {
                description: "Invalid commitment path".into(),
            })?;

            self.ibc_store
                .packet_commitment_store
                .get_keys(&commitment_path_prefix)
                .into_iter()
                .flat_map(|path| {
                    if let Ok(Path::Commitment(commitment_path)) = path.try_into() {
                        Some(commitment_path)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            sequences
                .into_iter()
                .map(|seq| CommitmentPath::new(&channel_end_path.0, &channel_end_path.1, seq))
                .collect()
        };

        Ok(collected_paths
            .into_iter()
            .filter(|commitment_path: &CommitmentPath| -> bool {
                self.ibc_store
                    .packet_commitment_store
                    .get(StoreHeight::Pending, commitment_path)
                    .is_some()
            })
            .map(|commitment_path| commitment_path.sequence)
            .collect())
    }
}

impl<S, H> ExecutionContext for MockGenericContext<S, H>
where
    S: ProvableStore + Debug,
    H: TestHost,
{
    /// Called upon client creation.
    /// Increases the counter which keeps track of how many clients have been created.
    /// Should never fail.
    fn increase_client_counter(&mut self) -> Result<(), ContextError> {
        let current_sequence = self
            .ibc_store
            .client_counter
            .get(StoreHeight::Pending, &NextClientSequencePath)
            .ok_or(ClientError::Other {
                description: "client counter not found".into(),
            })?;

        self.ibc_store
            .client_counter
            .set(NextClientSequencePath, current_sequence + 1)
            .map_err(|e| ClientError::Other {
                description: format!("client counter update failed: {e:?}"),
            })?;

        Ok(())
    }

    /// Stores the given connection_end at path
    fn store_connection(
        &mut self,
        connection_path: &ConnectionPath,
        connection_end: ConnectionEnd,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .connection_end_store
            .set(connection_path.clone(), connection_end)
            .map_err(|_| ConnectionError::Other {
                description: "Connection end store error".to_string(),
            })?;
        Ok(())
    }

    /// Stores the given connection_id at a path associated with the client_id.
    fn store_connection_to_client(
        &mut self,
        client_connection_path: &ClientConnectionPath,
        conn_id: ConnectionId,
    ) -> Result<(), ContextError> {
        let mut conn_ids: Vec<ConnectionId> = self
            .ibc_store
            .connection_ids_store
            .get(StoreHeight::Pending, client_connection_path)
            .unwrap_or_default();
        conn_ids.push(conn_id);
        self.ibc_store
            .connection_ids_store
            .set(client_connection_path.clone(), conn_ids)
            .map_err(|_| ConnectionError::Other {
                description: "Connection ids store error".to_string(),
            })?;
        Ok(())
    }

    /// Called upon connection identifier creation (Init or Try process).
    /// Increases the counter which keeps track of how many connections have been created.
    /// Should never fail.
    fn increase_connection_counter(&mut self) -> Result<(), ContextError> {
        let current_sequence = self
            .ibc_store
            .conn_counter
            .get(StoreHeight::Pending, &NextConnectionSequencePath)
            .ok_or(ConnectionError::Other {
                description: "connection counter not found".into(),
            })?;

        self.ibc_store
            .conn_counter
            .set(NextConnectionSequencePath, current_sequence + 1)
            .map_err(|e| ConnectionError::Other {
                description: format!("connection counter update failed: {e:?}"),
            })?;

        Ok(())
    }

    fn store_packet_commitment(
        &mut self,
        commitment_path: &CommitmentPath,
        commitment: PacketCommitment,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .packet_commitment_store
            .set(commitment_path.clone(), commitment)
            .map_err(|_| PacketError::ImplementationSpecific)?;
        Ok(())
    }

    fn delete_packet_commitment(&mut self, key: &CommitmentPath) -> Result<(), ContextError> {
        self.ibc_store.packet_commitment_store.delete(key.clone());
        Ok(())
    }

    fn store_packet_receipt(
        &mut self,
        receipt_path: &ReceiptPath,
        _receipt: Receipt,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .packet_receipt_store
            .set_path(receipt_path.clone())
            .map_err(|_| PacketError::ImplementationSpecific)?;
        Ok(())
    }

    fn store_packet_acknowledgement(
        &mut self,
        ack_path: &AckPath,
        ack_commitment: AcknowledgementCommitment,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .packet_ack_store
            .set(ack_path.clone(), ack_commitment)
            .map_err(|_| PacketError::ImplementationSpecific)?;
        Ok(())
    }

    fn delete_packet_acknowledgement(&mut self, ack_path: &AckPath) -> Result<(), ContextError> {
        self.ibc_store.packet_ack_store.delete(ack_path.clone());
        Ok(())
    }

    /// Stores the given channel_end at a path associated with the port_id and channel_id.
    fn store_channel(
        &mut self,
        channel_end_path: &ChannelEndPath,
        channel_end: ChannelEnd,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .channel_end_store
            .set(channel_end_path.clone(), channel_end)
            .map_err(|_| ChannelError::Other {
                description: "Channel end store error".to_string(),
            })?;
        Ok(())
    }

    fn store_next_sequence_send(
        &mut self,
        seq_send_path: &SeqSendPath,
        seq: Sequence,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .send_sequence_store
            .set(seq_send_path.clone(), seq)
            .map_err(|_| PacketError::ImplementationSpecific)?;
        Ok(())
    }

    fn store_next_sequence_recv(
        &mut self,
        seq_recv_path: &SeqRecvPath,
        seq: Sequence,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .recv_sequence_store
            .set(seq_recv_path.clone(), seq)
            .map_err(|_| PacketError::ImplementationSpecific)?;
        Ok(())
    }

    fn store_next_sequence_ack(
        &mut self,
        seq_ack_path: &SeqAckPath,
        seq: Sequence,
    ) -> Result<(), ContextError> {
        self.ibc_store
            .ack_sequence_store
            .set(seq_ack_path.clone(), seq)
            .map_err(|_| PacketError::ImplementationSpecific)?;
        Ok(())
    }

    fn increase_channel_counter(&mut self) -> Result<(), ContextError> {
        let current_sequence = self
            .ibc_store
            .channel_counter
            .get(StoreHeight::Pending, &NextChannelSequencePath)
            .ok_or(ChannelError::Other {
                description: "channel counter not found".into(),
            })?;

        self.ibc_store
            .channel_counter
            .set(NextChannelSequencePath, current_sequence + 1)
            .map_err(|e| ChannelError::Other {
                description: format!("channel counter update failed: {e:?}"),
            })?;

        Ok(())
    }

    fn emit_ibc_event(&mut self, event: IbcEvent) -> Result<(), ContextError> {
        self.ibc_store.events.lock().push(event);
        Ok(())
    }

    fn log_message(&mut self, message: String) -> Result<(), ContextError> {
        self.ibc_store.logs.lock().push(message);
        Ok(())
    }

    fn get_client_execution_context(&mut self) -> &mut Self::E {
        self
    }
}
