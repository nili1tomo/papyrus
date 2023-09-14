use futures::AsyncWriteExt;
use libp2p::core::upgrade::{write_varint, InboundUpgrade, OutboundUpgrade};
use libp2p::core::UpgradeInfo;
use pretty_assertions::assert_eq;

use super::{InboundProtocol, OutboundProtocol, PROTOCOL_NAME};
use crate::messages::block::{BlockHeader, GetBlocks, GetBlocksResponse};
use crate::messages::common::{BlockId, Fin};
use crate::messages::proto::p2p::proto::get_blocks_response::Response;
use crate::messages::{read_message, write_message};
use crate::test_utils::get_connected_streams;

fn hardcoded_responses() -> Vec<GetBlocksResponse> {
    vec![
        GetBlocksResponse {
            response: Some(Response::Header(BlockHeader {
                parent_block: Some(BlockId { hash: None, height: 1 }),
                ..Default::default()
            })),
        },
        GetBlocksResponse {
            response: Some(Response::Header(BlockHeader {
                parent_block: Some(BlockId { hash: None, height: 2 }),
                ..Default::default()
            })),
        },
        GetBlocksResponse {
            response: Some(Response::Header(BlockHeader {
                parent_block: Some(BlockId { hash: None, height: 3 }),
                ..Default::default()
            })),
        },
        GetBlocksResponse { response: Some(Response::Fin(Fin {})) },
    ]
}

#[test]
fn both_protocols_have_same_info() {
    let outbound_protocol = OutboundProtocol::<GetBlocks> { query: Default::default() };
    let inbound_protocol = InboundProtocol::<GetBlocks>::new();
    assert_eq!(
        outbound_protocol.protocol_info().collect::<Vec<_>>(),
        inbound_protocol.protocol_info().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn positive_flow() {
    let (inbound_stream, outbound_stream) = get_connected_streams().await;

    let query = GetBlocks::default();
    let outbound_protocol = OutboundProtocol { query: query.clone() };
    let inbound_protocol = InboundProtocol::<GetBlocks>::new();

    tokio::join!(
        async move {
            let (received_query, mut stream) =
                inbound_protocol.upgrade_inbound(inbound_stream, PROTOCOL_NAME).await.unwrap();
            assert_eq!(query, received_query);
            for response in hardcoded_responses() {
                write_message(response, &mut stream).await.unwrap();
            }
        },
        async move {
            let mut stream =
                outbound_protocol.upgrade_outbound(outbound_stream, PROTOCOL_NAME).await.unwrap();
            for expected_response in hardcoded_responses() {
                let response = read_message::<GetBlocksResponse, _>(&mut stream).await.unwrap();
                assert_eq!(response, expected_response);
            }
        }
    );
}

#[tokio::test]
async fn outbound_sends_invalid_request() {
    let (inbound_stream, mut outbound_stream) = get_connected_streams().await;
    let inbound_protocol = InboundProtocol::<GetBlocks>::new();

    tokio::join!(
        async move {
            assert!(inbound_protocol.upgrade_inbound(inbound_stream, PROTOCOL_NAME).await.is_err());
        },
        async move {
            // The first element is the length of the message, if we don't write that many bytes
            // after then the message will be invalid.
            write_varint(&mut outbound_stream, 10).await.unwrap();
            outbound_stream.close().await.unwrap();
        },
    );
}