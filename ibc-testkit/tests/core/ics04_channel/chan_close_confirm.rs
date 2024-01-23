use ibc::core::channel::types::channel::{ChannelEnd, Counterparty, Order, State as ChannelState};
use ibc::core::channel::types::msgs::{ChannelMsg, MsgChannelCloseConfirm};
use ibc::core::channel::types::Version;
use ibc::core::connection::types::version::get_compatible_versions;
use ibc::core::connection::types::{
    ConnectionEnd, Counterparty as ConnectionCounterparty, State as ConnectionState,
};
use ibc::core::entrypoint::{execute, validate};
use ibc::core::handler::types::events::{IbcEvent, MessageEvent};
use ibc::core::handler::types::msgs::MsgEnvelope;
use ibc::core::host::types::identifiers::ConnectionId;
use ibc::core::host::ValidationContext;
use ibc::core::primitives::*;
use ibc_testkit::fixtures::core::channel::dummy_raw_msg_chan_close_confirm;
use ibc_testkit::fixtures::core::connection::dummy_raw_counterparty_conn;
use ibc_testkit::testapp::ibc::clients::mock::client_state::client_type as mock_client_type;
use ibc_testkit::testapp::ibc::core::router::MockRouter;
use ibc_testkit::testapp::ibc::core::types::{MockClientConfig, MockContext};

#[test]
fn test_chan_close_confirm_validate() {
    let client_id = mock_client_type().build_client_id(24);
    let conn_id = ConnectionId::new(2);
    let default_context = MockContext::default();
    let client_consensus_state_height = default_context.host_height().unwrap();

    let conn_end = ConnectionEnd::new(
        ConnectionState::Open,
        client_id.clone(),
        ConnectionCounterparty::try_from(dummy_raw_counterparty_conn(Some(0))).unwrap(),
        get_compatible_versions(),
        ZERO_DURATION,
    )
    .unwrap();

    let msg_chan_close_confirm = MsgChannelCloseConfirm::try_from(
        dummy_raw_msg_chan_close_confirm(client_consensus_state_height.revision_height()),
    )
    .unwrap();

    let msg_envelope = MsgEnvelope::from(ChannelMsg::from(msg_chan_close_confirm.clone()));

    let chan_end = ChannelEnd::new(
        ChannelState::Open,
        Order::default(),
        Counterparty::new(
            msg_chan_close_confirm.port_id_on_b.clone(),
            Some(msg_chan_close_confirm.chan_id_on_b.clone()),
        ),
        vec![conn_id.clone()],
        Version::default(),
    )
    .unwrap();

    let context = default_context
        .with_client_config(
            MockClientConfig::builder()
                .client_id(client_id.clone())
                .latest_height(client_consensus_state_height)
                .build(),
        )
        .with_connection(conn_id, conn_end)
        .with_channel(
            msg_chan_close_confirm.port_id_on_b.clone(),
            msg_chan_close_confirm.chan_id_on_b.clone(),
            chan_end,
        );

    let router = MockRouter::new_with_transfer();

    let res = validate(&context, &router, msg_envelope);

    assert!(
        res.is_ok(),
        "Validation expected to succeed (happy path). Error: {res:?}"
    );
}

#[test]
fn test_chan_close_confirm_execute() {
    let client_id = mock_client_type().build_client_id(24);
    let conn_id = ConnectionId::new(2);
    let default_context = MockContext::default();
    let client_consensus_state_height = default_context.host_height().unwrap();

    let conn_end = ConnectionEnd::new(
        ConnectionState::Open,
        client_id.clone(),
        ConnectionCounterparty::try_from(dummy_raw_counterparty_conn(Some(0))).unwrap(),
        get_compatible_versions(),
        ZERO_DURATION,
    )
    .unwrap();

    let msg_chan_close_confirm = MsgChannelCloseConfirm::try_from(
        dummy_raw_msg_chan_close_confirm(client_consensus_state_height.revision_height()),
    )
    .unwrap();

    let msg_envelope = MsgEnvelope::from(ChannelMsg::from(msg_chan_close_confirm.clone()));

    let chan_end = ChannelEnd::new(
        ChannelState::Open,
        Order::default(),
        Counterparty::new(
            msg_chan_close_confirm.port_id_on_b.clone(),
            Some(msg_chan_close_confirm.chan_id_on_b.clone()),
        ),
        vec![conn_id.clone()],
        Version::default(),
    )
    .unwrap();

    let mut context = default_context
        .with_client_config(
            MockClientConfig::builder()
                .client_id(client_id.clone())
                .latest_height(client_consensus_state_height)
                .build(),
        )
        .with_connection(conn_id, conn_end)
        .with_channel(
            msg_chan_close_confirm.port_id_on_b.clone(),
            msg_chan_close_confirm.chan_id_on_b.clone(),
            chan_end,
        );

    let mut router = MockRouter::new_with_transfer();

    let res = execute(&mut context, &mut router, msg_envelope);

    assert!(res.is_ok(), "Execution success: happy path");

    let ibc_events = context.get_events();

    assert_eq!(ibc_events.len(), 2);

    assert!(matches!(
        ibc_events[0],
        IbcEvent::Message(MessageEvent::Channel)
    ));

    assert!(matches!(ibc_events[1], IbcEvent::CloseConfirmChannel(_)));
}
