//! Messages used by the Kafka protocol.
//!
//! These messages are generated programmatically. See the [Kafka's protocol documentation](https://kafka.apache.org/protocol.html) for more information about a given message type.
// WARNING: the items of this module are generated and should not be edited directly.

#[cfg(feature = "messages_enums")]
#[cfg(any(feature = "client", feature = "broker"))]
use crate::error::Result;
#[cfg(feature = "messages_enums")]
#[cfg(any(feature = "client", feature = "broker"))]
use crate::protocol::Decodable;
#[cfg(feature = "messages_enums")]
#[cfg(any(feature = "client", feature = "broker"))]
use crate::protocol::Encodable;
#[cfg(all(feature = "client", feature = "broker"))]
use crate::protocol::Request;
use crate::protocol::VersionRange;
use crate::protocol::{HeaderVersion, NewType, StrBytes};
use std::convert::TryFrom;

pub mod consumer_protocol_assignment;
pub use consumer_protocol_assignment::ConsumerProtocolAssignment;

pub mod consumer_protocol_subscription;
pub use consumer_protocol_subscription::ConsumerProtocolSubscription;

pub mod default_principal_data;
pub use default_principal_data::DefaultPrincipalData;

pub mod end_txn_marker;
pub use end_txn_marker::EndTxnMarker;

pub mod k_raft_version_record;
pub use k_raft_version_record::KRaftVersionRecord;

pub mod leader_change_message;
pub use leader_change_message::LeaderChangeMessage;

pub mod request_header;
pub use request_header::RequestHeader;

pub mod response_header;
pub use response_header::ResponseHeader;

pub mod snapshot_footer_record;
pub use snapshot_footer_record::SnapshotFooterRecord;

pub mod snapshot_header_record;
pub use snapshot_header_record::SnapshotHeaderRecord;

pub mod voters_record;
pub use voters_record::VotersRecord;

pub mod produce_request;
pub use produce_request::ProduceRequest;

pub mod fetch_request;
pub use fetch_request::FetchRequest;

pub mod list_offsets_request;
pub use list_offsets_request::ListOffsetsRequest;

pub mod metadata_request;
pub use metadata_request::MetadataRequest;

pub mod offset_commit_request;
pub use offset_commit_request::OffsetCommitRequest;

pub mod offset_fetch_request;
pub use offset_fetch_request::OffsetFetchRequest;

pub mod find_coordinator_request;
pub use find_coordinator_request::FindCoordinatorRequest;

pub mod join_group_request;
pub use join_group_request::JoinGroupRequest;

pub mod heartbeat_request;
pub use heartbeat_request::HeartbeatRequest;

pub mod leave_group_request;
pub use leave_group_request::LeaveGroupRequest;

pub mod sync_group_request;
pub use sync_group_request::SyncGroupRequest;

pub mod describe_groups_request;
pub use describe_groups_request::DescribeGroupsRequest;

pub mod list_groups_request;
pub use list_groups_request::ListGroupsRequest;

pub mod sasl_handshake_request;
pub use sasl_handshake_request::SaslHandshakeRequest;

pub mod api_versions_request;
pub use api_versions_request::ApiVersionsRequest;

pub mod create_topics_request;
pub use create_topics_request::CreateTopicsRequest;

pub mod delete_topics_request;
pub use delete_topics_request::DeleteTopicsRequest;

pub mod delete_records_request;
pub use delete_records_request::DeleteRecordsRequest;

pub mod init_producer_id_request;
pub use init_producer_id_request::InitProducerIdRequest;

pub mod offset_for_leader_epoch_request;
pub use offset_for_leader_epoch_request::OffsetForLeaderEpochRequest;

pub mod add_partitions_to_txn_request;
pub use add_partitions_to_txn_request::AddPartitionsToTxnRequest;

pub mod add_offsets_to_txn_request;
pub use add_offsets_to_txn_request::AddOffsetsToTxnRequest;

pub mod end_txn_request;
pub use end_txn_request::EndTxnRequest;

pub mod write_txn_markers_request;
pub use write_txn_markers_request::WriteTxnMarkersRequest;

pub mod txn_offset_commit_request;
pub use txn_offset_commit_request::TxnOffsetCommitRequest;

pub mod describe_acls_request;
pub use describe_acls_request::DescribeAclsRequest;

pub mod create_acls_request;
pub use create_acls_request::CreateAclsRequest;

pub mod delete_acls_request;
pub use delete_acls_request::DeleteAclsRequest;

pub mod describe_configs_request;
pub use describe_configs_request::DescribeConfigsRequest;

pub mod alter_configs_request;
pub use alter_configs_request::AlterConfigsRequest;

pub mod alter_replica_log_dirs_request;
pub use alter_replica_log_dirs_request::AlterReplicaLogDirsRequest;

pub mod describe_log_dirs_request;
pub use describe_log_dirs_request::DescribeLogDirsRequest;

pub mod sasl_authenticate_request;
pub use sasl_authenticate_request::SaslAuthenticateRequest;

pub mod create_partitions_request;
pub use create_partitions_request::CreatePartitionsRequest;

pub mod create_delegation_token_request;
pub use create_delegation_token_request::CreateDelegationTokenRequest;

pub mod renew_delegation_token_request;
pub use renew_delegation_token_request::RenewDelegationTokenRequest;

pub mod expire_delegation_token_request;
pub use expire_delegation_token_request::ExpireDelegationTokenRequest;

pub mod describe_delegation_token_request;
pub use describe_delegation_token_request::DescribeDelegationTokenRequest;

pub mod delete_groups_request;
pub use delete_groups_request::DeleteGroupsRequest;

pub mod elect_leaders_request;
pub use elect_leaders_request::ElectLeadersRequest;

pub mod incremental_alter_configs_request;
pub use incremental_alter_configs_request::IncrementalAlterConfigsRequest;

pub mod alter_partition_reassignments_request;
pub use alter_partition_reassignments_request::AlterPartitionReassignmentsRequest;

pub mod list_partition_reassignments_request;
pub use list_partition_reassignments_request::ListPartitionReassignmentsRequest;

pub mod offset_delete_request;
pub use offset_delete_request::OffsetDeleteRequest;

pub mod describe_client_quotas_request;
pub use describe_client_quotas_request::DescribeClientQuotasRequest;

pub mod alter_client_quotas_request;
pub use alter_client_quotas_request::AlterClientQuotasRequest;

pub mod describe_user_scram_credentials_request;
pub use describe_user_scram_credentials_request::DescribeUserScramCredentialsRequest;

pub mod alter_user_scram_credentials_request;
pub use alter_user_scram_credentials_request::AlterUserScramCredentialsRequest;

pub mod vote_request;
pub use vote_request::VoteRequest;

pub mod begin_quorum_epoch_request;
pub use begin_quorum_epoch_request::BeginQuorumEpochRequest;

pub mod end_quorum_epoch_request;
pub use end_quorum_epoch_request::EndQuorumEpochRequest;

pub mod describe_quorum_request;
pub use describe_quorum_request::DescribeQuorumRequest;

pub mod alter_partition_request;
pub use alter_partition_request::AlterPartitionRequest;

pub mod update_features_request;
pub use update_features_request::UpdateFeaturesRequest;

pub mod envelope_request;
pub use envelope_request::EnvelopeRequest;

pub mod fetch_snapshot_request;
pub use fetch_snapshot_request::FetchSnapshotRequest;

pub mod describe_cluster_request;
pub use describe_cluster_request::DescribeClusterRequest;

pub mod describe_producers_request;
pub use describe_producers_request::DescribeProducersRequest;

pub mod broker_registration_request;
pub use broker_registration_request::BrokerRegistrationRequest;

pub mod broker_heartbeat_request;
pub use broker_heartbeat_request::BrokerHeartbeatRequest;

pub mod unregister_broker_request;
pub use unregister_broker_request::UnregisterBrokerRequest;

pub mod describe_transactions_request;
pub use describe_transactions_request::DescribeTransactionsRequest;

pub mod list_transactions_request;
pub use list_transactions_request::ListTransactionsRequest;

pub mod allocate_producer_ids_request;
pub use allocate_producer_ids_request::AllocateProducerIdsRequest;

pub mod consumer_group_heartbeat_request;
pub use consumer_group_heartbeat_request::ConsumerGroupHeartbeatRequest;

pub mod consumer_group_describe_request;
pub use consumer_group_describe_request::ConsumerGroupDescribeRequest;

pub mod controller_registration_request;
pub use controller_registration_request::ControllerRegistrationRequest;

pub mod get_telemetry_subscriptions_request;
pub use get_telemetry_subscriptions_request::GetTelemetrySubscriptionsRequest;

pub mod push_telemetry_request;
pub use push_telemetry_request::PushTelemetryRequest;

pub mod assign_replicas_to_dirs_request;
pub use assign_replicas_to_dirs_request::AssignReplicasToDirsRequest;

pub mod list_config_resources_request;
pub use list_config_resources_request::ListConfigResourcesRequest;

pub mod describe_topic_partitions_request;
pub use describe_topic_partitions_request::DescribeTopicPartitionsRequest;

pub mod share_group_heartbeat_request;
pub use share_group_heartbeat_request::ShareGroupHeartbeatRequest;

pub mod share_group_describe_request;
pub use share_group_describe_request::ShareGroupDescribeRequest;

pub mod share_fetch_request;
pub use share_fetch_request::ShareFetchRequest;

pub mod share_acknowledge_request;
pub use share_acknowledge_request::ShareAcknowledgeRequest;

pub mod add_raft_voter_request;
pub use add_raft_voter_request::AddRaftVoterRequest;

pub mod remove_raft_voter_request;
pub use remove_raft_voter_request::RemoveRaftVoterRequest;

pub mod update_raft_voter_request;
pub use update_raft_voter_request::UpdateRaftVoterRequest;

pub mod initialize_share_group_state_request;
pub use initialize_share_group_state_request::InitializeShareGroupStateRequest;

pub mod read_share_group_state_request;
pub use read_share_group_state_request::ReadShareGroupStateRequest;

pub mod write_share_group_state_request;
pub use write_share_group_state_request::WriteShareGroupStateRequest;

pub mod delete_share_group_state_request;
pub use delete_share_group_state_request::DeleteShareGroupStateRequest;

pub mod read_share_group_state_summary_request;
pub use read_share_group_state_summary_request::ReadShareGroupStateSummaryRequest;

pub mod describe_share_group_offsets_request;
pub use describe_share_group_offsets_request::DescribeShareGroupOffsetsRequest;

pub mod alter_share_group_offsets_request;
pub use alter_share_group_offsets_request::AlterShareGroupOffsetsRequest;

pub mod delete_share_group_offsets_request;
pub use delete_share_group_offsets_request::DeleteShareGroupOffsetsRequest;

pub mod produce_response;
pub use produce_response::ProduceResponse;

pub mod fetch_response;
pub use fetch_response::FetchResponse;

pub mod list_offsets_response;
pub use list_offsets_response::ListOffsetsResponse;

pub mod metadata_response;
pub use metadata_response::MetadataResponse;

pub mod offset_commit_response;
pub use offset_commit_response::OffsetCommitResponse;

pub mod offset_fetch_response;
pub use offset_fetch_response::OffsetFetchResponse;

pub mod find_coordinator_response;
pub use find_coordinator_response::FindCoordinatorResponse;

pub mod join_group_response;
pub use join_group_response::JoinGroupResponse;

pub mod heartbeat_response;
pub use heartbeat_response::HeartbeatResponse;

pub mod leave_group_response;
pub use leave_group_response::LeaveGroupResponse;

pub mod sync_group_response;
pub use sync_group_response::SyncGroupResponse;

pub mod describe_groups_response;
pub use describe_groups_response::DescribeGroupsResponse;

pub mod list_groups_response;
pub use list_groups_response::ListGroupsResponse;

pub mod sasl_handshake_response;
pub use sasl_handshake_response::SaslHandshakeResponse;

pub mod api_versions_response;
pub use api_versions_response::ApiVersionsResponse;

pub mod create_topics_response;
pub use create_topics_response::CreateTopicsResponse;

pub mod delete_topics_response;
pub use delete_topics_response::DeleteTopicsResponse;

pub mod delete_records_response;
pub use delete_records_response::DeleteRecordsResponse;

pub mod init_producer_id_response;
pub use init_producer_id_response::InitProducerIdResponse;

pub mod offset_for_leader_epoch_response;
pub use offset_for_leader_epoch_response::OffsetForLeaderEpochResponse;

pub mod add_partitions_to_txn_response;
pub use add_partitions_to_txn_response::AddPartitionsToTxnResponse;

pub mod add_offsets_to_txn_response;
pub use add_offsets_to_txn_response::AddOffsetsToTxnResponse;

pub mod end_txn_response;
pub use end_txn_response::EndTxnResponse;

pub mod write_txn_markers_response;
pub use write_txn_markers_response::WriteTxnMarkersResponse;

pub mod txn_offset_commit_response;
pub use txn_offset_commit_response::TxnOffsetCommitResponse;

pub mod describe_acls_response;
pub use describe_acls_response::DescribeAclsResponse;

pub mod create_acls_response;
pub use create_acls_response::CreateAclsResponse;

pub mod delete_acls_response;
pub use delete_acls_response::DeleteAclsResponse;

pub mod describe_configs_response;
pub use describe_configs_response::DescribeConfigsResponse;

pub mod alter_configs_response;
pub use alter_configs_response::AlterConfigsResponse;

pub mod alter_replica_log_dirs_response;
pub use alter_replica_log_dirs_response::AlterReplicaLogDirsResponse;

pub mod describe_log_dirs_response;
pub use describe_log_dirs_response::DescribeLogDirsResponse;

pub mod sasl_authenticate_response;
pub use sasl_authenticate_response::SaslAuthenticateResponse;

pub mod create_partitions_response;
pub use create_partitions_response::CreatePartitionsResponse;

pub mod create_delegation_token_response;
pub use create_delegation_token_response::CreateDelegationTokenResponse;

pub mod renew_delegation_token_response;
pub use renew_delegation_token_response::RenewDelegationTokenResponse;

pub mod expire_delegation_token_response;
pub use expire_delegation_token_response::ExpireDelegationTokenResponse;

pub mod describe_delegation_token_response;
pub use describe_delegation_token_response::DescribeDelegationTokenResponse;

pub mod delete_groups_response;
pub use delete_groups_response::DeleteGroupsResponse;

pub mod elect_leaders_response;
pub use elect_leaders_response::ElectLeadersResponse;

pub mod incremental_alter_configs_response;
pub use incremental_alter_configs_response::IncrementalAlterConfigsResponse;

pub mod alter_partition_reassignments_response;
pub use alter_partition_reassignments_response::AlterPartitionReassignmentsResponse;

pub mod list_partition_reassignments_response;
pub use list_partition_reassignments_response::ListPartitionReassignmentsResponse;

pub mod offset_delete_response;
pub use offset_delete_response::OffsetDeleteResponse;

pub mod describe_client_quotas_response;
pub use describe_client_quotas_response::DescribeClientQuotasResponse;

pub mod alter_client_quotas_response;
pub use alter_client_quotas_response::AlterClientQuotasResponse;

pub mod describe_user_scram_credentials_response;
pub use describe_user_scram_credentials_response::DescribeUserScramCredentialsResponse;

pub mod alter_user_scram_credentials_response;
pub use alter_user_scram_credentials_response::AlterUserScramCredentialsResponse;

pub mod vote_response;
pub use vote_response::VoteResponse;

pub mod begin_quorum_epoch_response;
pub use begin_quorum_epoch_response::BeginQuorumEpochResponse;

pub mod end_quorum_epoch_response;
pub use end_quorum_epoch_response::EndQuorumEpochResponse;

pub mod describe_quorum_response;
pub use describe_quorum_response::DescribeQuorumResponse;

pub mod alter_partition_response;
pub use alter_partition_response::AlterPartitionResponse;

pub mod update_features_response;
pub use update_features_response::UpdateFeaturesResponse;

pub mod envelope_response;
pub use envelope_response::EnvelopeResponse;

pub mod fetch_snapshot_response;
pub use fetch_snapshot_response::FetchSnapshotResponse;

pub mod describe_cluster_response;
pub use describe_cluster_response::DescribeClusterResponse;

pub mod describe_producers_response;
pub use describe_producers_response::DescribeProducersResponse;

pub mod broker_registration_response;
pub use broker_registration_response::BrokerRegistrationResponse;

pub mod broker_heartbeat_response;
pub use broker_heartbeat_response::BrokerHeartbeatResponse;

pub mod unregister_broker_response;
pub use unregister_broker_response::UnregisterBrokerResponse;

pub mod describe_transactions_response;
pub use describe_transactions_response::DescribeTransactionsResponse;

pub mod list_transactions_response;
pub use list_transactions_response::ListTransactionsResponse;

pub mod allocate_producer_ids_response;
pub use allocate_producer_ids_response::AllocateProducerIdsResponse;

pub mod consumer_group_heartbeat_response;
pub use consumer_group_heartbeat_response::ConsumerGroupHeartbeatResponse;

pub mod consumer_group_describe_response;
pub use consumer_group_describe_response::ConsumerGroupDescribeResponse;

pub mod controller_registration_response;
pub use controller_registration_response::ControllerRegistrationResponse;

pub mod get_telemetry_subscriptions_response;
pub use get_telemetry_subscriptions_response::GetTelemetrySubscriptionsResponse;

pub mod push_telemetry_response;
pub use push_telemetry_response::PushTelemetryResponse;

pub mod assign_replicas_to_dirs_response;
pub use assign_replicas_to_dirs_response::AssignReplicasToDirsResponse;

pub mod list_config_resources_response;
pub use list_config_resources_response::ListConfigResourcesResponse;

pub mod describe_topic_partitions_response;
pub use describe_topic_partitions_response::DescribeTopicPartitionsResponse;

pub mod share_group_heartbeat_response;
pub use share_group_heartbeat_response::ShareGroupHeartbeatResponse;

pub mod share_group_describe_response;
pub use share_group_describe_response::ShareGroupDescribeResponse;

pub mod share_fetch_response;
pub use share_fetch_response::ShareFetchResponse;

pub mod share_acknowledge_response;
pub use share_acknowledge_response::ShareAcknowledgeResponse;

pub mod add_raft_voter_response;
pub use add_raft_voter_response::AddRaftVoterResponse;

pub mod remove_raft_voter_response;
pub use remove_raft_voter_response::RemoveRaftVoterResponse;

pub mod update_raft_voter_response;
pub use update_raft_voter_response::UpdateRaftVoterResponse;

pub mod initialize_share_group_state_response;
pub use initialize_share_group_state_response::InitializeShareGroupStateResponse;

pub mod read_share_group_state_response;
pub use read_share_group_state_response::ReadShareGroupStateResponse;

pub mod write_share_group_state_response;
pub use write_share_group_state_response::WriteShareGroupStateResponse;

pub mod delete_share_group_state_response;
pub use delete_share_group_state_response::DeleteShareGroupStateResponse;

pub mod read_share_group_state_summary_response;
pub use read_share_group_state_summary_response::ReadShareGroupStateSummaryResponse;

pub mod describe_share_group_offsets_response;
pub use describe_share_group_offsets_response::DescribeShareGroupOffsetsResponse;

pub mod alter_share_group_offsets_response;
pub use alter_share_group_offsets_response::AlterShareGroupOffsetsResponse;

pub mod delete_share_group_offsets_response;
pub use delete_share_group_offsets_response::DeleteShareGroupOffsetsResponse;

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ProduceRequest {
    const KEY: i16 = 0;
    type Response = ProduceResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for FetchRequest {
    const KEY: i16 = 1;
    type Response = FetchResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ListOffsetsRequest {
    const KEY: i16 = 2;
    type Response = ListOffsetsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for MetadataRequest {
    const KEY: i16 = 3;
    type Response = MetadataResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for OffsetCommitRequest {
    const KEY: i16 = 8;
    type Response = OffsetCommitResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for OffsetFetchRequest {
    const KEY: i16 = 9;
    type Response = OffsetFetchResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for FindCoordinatorRequest {
    const KEY: i16 = 10;
    type Response = FindCoordinatorResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for JoinGroupRequest {
    const KEY: i16 = 11;
    type Response = JoinGroupResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for HeartbeatRequest {
    const KEY: i16 = 12;
    type Response = HeartbeatResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for LeaveGroupRequest {
    const KEY: i16 = 13;
    type Response = LeaveGroupResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for SyncGroupRequest {
    const KEY: i16 = 14;
    type Response = SyncGroupResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeGroupsRequest {
    const KEY: i16 = 15;
    type Response = DescribeGroupsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ListGroupsRequest {
    const KEY: i16 = 16;
    type Response = ListGroupsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for SaslHandshakeRequest {
    const KEY: i16 = 17;
    type Response = SaslHandshakeResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ApiVersionsRequest {
    const KEY: i16 = 18;
    type Response = ApiVersionsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for CreateTopicsRequest {
    const KEY: i16 = 19;
    type Response = CreateTopicsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DeleteTopicsRequest {
    const KEY: i16 = 20;
    type Response = DeleteTopicsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DeleteRecordsRequest {
    const KEY: i16 = 21;
    type Response = DeleteRecordsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for InitProducerIdRequest {
    const KEY: i16 = 22;
    type Response = InitProducerIdResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for OffsetForLeaderEpochRequest {
    const KEY: i16 = 23;
    type Response = OffsetForLeaderEpochResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AddPartitionsToTxnRequest {
    const KEY: i16 = 24;
    type Response = AddPartitionsToTxnResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AddOffsetsToTxnRequest {
    const KEY: i16 = 25;
    type Response = AddOffsetsToTxnResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for EndTxnRequest {
    const KEY: i16 = 26;
    type Response = EndTxnResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for WriteTxnMarkersRequest {
    const KEY: i16 = 27;
    type Response = WriteTxnMarkersResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for TxnOffsetCommitRequest {
    const KEY: i16 = 28;
    type Response = TxnOffsetCommitResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeAclsRequest {
    const KEY: i16 = 29;
    type Response = DescribeAclsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for CreateAclsRequest {
    const KEY: i16 = 30;
    type Response = CreateAclsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DeleteAclsRequest {
    const KEY: i16 = 31;
    type Response = DeleteAclsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeConfigsRequest {
    const KEY: i16 = 32;
    type Response = DescribeConfigsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AlterConfigsRequest {
    const KEY: i16 = 33;
    type Response = AlterConfigsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AlterReplicaLogDirsRequest {
    const KEY: i16 = 34;
    type Response = AlterReplicaLogDirsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeLogDirsRequest {
    const KEY: i16 = 35;
    type Response = DescribeLogDirsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for SaslAuthenticateRequest {
    const KEY: i16 = 36;
    type Response = SaslAuthenticateResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for CreatePartitionsRequest {
    const KEY: i16 = 37;
    type Response = CreatePartitionsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for CreateDelegationTokenRequest {
    const KEY: i16 = 38;
    type Response = CreateDelegationTokenResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for RenewDelegationTokenRequest {
    const KEY: i16 = 39;
    type Response = RenewDelegationTokenResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ExpireDelegationTokenRequest {
    const KEY: i16 = 40;
    type Response = ExpireDelegationTokenResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeDelegationTokenRequest {
    const KEY: i16 = 41;
    type Response = DescribeDelegationTokenResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DeleteGroupsRequest {
    const KEY: i16 = 42;
    type Response = DeleteGroupsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ElectLeadersRequest {
    const KEY: i16 = 43;
    type Response = ElectLeadersResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for IncrementalAlterConfigsRequest {
    const KEY: i16 = 44;
    type Response = IncrementalAlterConfigsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AlterPartitionReassignmentsRequest {
    const KEY: i16 = 45;
    type Response = AlterPartitionReassignmentsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ListPartitionReassignmentsRequest {
    const KEY: i16 = 46;
    type Response = ListPartitionReassignmentsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for OffsetDeleteRequest {
    const KEY: i16 = 47;
    type Response = OffsetDeleteResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeClientQuotasRequest {
    const KEY: i16 = 48;
    type Response = DescribeClientQuotasResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AlterClientQuotasRequest {
    const KEY: i16 = 49;
    type Response = AlterClientQuotasResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeUserScramCredentialsRequest {
    const KEY: i16 = 50;
    type Response = DescribeUserScramCredentialsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AlterUserScramCredentialsRequest {
    const KEY: i16 = 51;
    type Response = AlterUserScramCredentialsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for VoteRequest {
    const KEY: i16 = 52;
    type Response = VoteResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for BeginQuorumEpochRequest {
    const KEY: i16 = 53;
    type Response = BeginQuorumEpochResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for EndQuorumEpochRequest {
    const KEY: i16 = 54;
    type Response = EndQuorumEpochResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeQuorumRequest {
    const KEY: i16 = 55;
    type Response = DescribeQuorumResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AlterPartitionRequest {
    const KEY: i16 = 56;
    type Response = AlterPartitionResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for UpdateFeaturesRequest {
    const KEY: i16 = 57;
    type Response = UpdateFeaturesResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for EnvelopeRequest {
    const KEY: i16 = 58;
    type Response = EnvelopeResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for FetchSnapshotRequest {
    const KEY: i16 = 59;
    type Response = FetchSnapshotResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeClusterRequest {
    const KEY: i16 = 60;
    type Response = DescribeClusterResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeProducersRequest {
    const KEY: i16 = 61;
    type Response = DescribeProducersResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for BrokerRegistrationRequest {
    const KEY: i16 = 62;
    type Response = BrokerRegistrationResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for BrokerHeartbeatRequest {
    const KEY: i16 = 63;
    type Response = BrokerHeartbeatResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for UnregisterBrokerRequest {
    const KEY: i16 = 64;
    type Response = UnregisterBrokerResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeTransactionsRequest {
    const KEY: i16 = 65;
    type Response = DescribeTransactionsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ListTransactionsRequest {
    const KEY: i16 = 66;
    type Response = ListTransactionsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AllocateProducerIdsRequest {
    const KEY: i16 = 67;
    type Response = AllocateProducerIdsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ConsumerGroupHeartbeatRequest {
    const KEY: i16 = 68;
    type Response = ConsumerGroupHeartbeatResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ConsumerGroupDescribeRequest {
    const KEY: i16 = 69;
    type Response = ConsumerGroupDescribeResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ControllerRegistrationRequest {
    const KEY: i16 = 70;
    type Response = ControllerRegistrationResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for GetTelemetrySubscriptionsRequest {
    const KEY: i16 = 71;
    type Response = GetTelemetrySubscriptionsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for PushTelemetryRequest {
    const KEY: i16 = 72;
    type Response = PushTelemetryResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AssignReplicasToDirsRequest {
    const KEY: i16 = 73;
    type Response = AssignReplicasToDirsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ListConfigResourcesRequest {
    const KEY: i16 = 74;
    type Response = ListConfigResourcesResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeTopicPartitionsRequest {
    const KEY: i16 = 75;
    type Response = DescribeTopicPartitionsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ShareGroupHeartbeatRequest {
    const KEY: i16 = 76;
    type Response = ShareGroupHeartbeatResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ShareGroupDescribeRequest {
    const KEY: i16 = 77;
    type Response = ShareGroupDescribeResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ShareFetchRequest {
    const KEY: i16 = 78;
    type Response = ShareFetchResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ShareAcknowledgeRequest {
    const KEY: i16 = 79;
    type Response = ShareAcknowledgeResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AddRaftVoterRequest {
    const KEY: i16 = 80;
    type Response = AddRaftVoterResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for RemoveRaftVoterRequest {
    const KEY: i16 = 81;
    type Response = RemoveRaftVoterResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for UpdateRaftVoterRequest {
    const KEY: i16 = 82;
    type Response = UpdateRaftVoterResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for InitializeShareGroupStateRequest {
    const KEY: i16 = 83;
    type Response = InitializeShareGroupStateResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ReadShareGroupStateRequest {
    const KEY: i16 = 84;
    type Response = ReadShareGroupStateResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for WriteShareGroupStateRequest {
    const KEY: i16 = 85;
    type Response = WriteShareGroupStateResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DeleteShareGroupStateRequest {
    const KEY: i16 = 86;
    type Response = DeleteShareGroupStateResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for ReadShareGroupStateSummaryRequest {
    const KEY: i16 = 87;
    type Response = ReadShareGroupStateSummaryResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DescribeShareGroupOffsetsRequest {
    const KEY: i16 = 90;
    type Response = DescribeShareGroupOffsetsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for AlterShareGroupOffsetsRequest {
    const KEY: i16 = 91;
    type Response = AlterShareGroupOffsetsResponse;
}

#[cfg(all(feature = "client", feature = "broker"))]
impl Request for DeleteShareGroupOffsetsRequest {
    const KEY: i16 = 92;
    type Response = DeleteShareGroupOffsetsResponse;
}

macro_rules! for_each_api {
    ($mac:ident) => {
        $mac! {
            (Produce, ProduceRequest, ProduceResponse, 3, 13),
            (Fetch, FetchRequest, FetchResponse, 4, 18),
            (ListOffsets, ListOffsetsRequest, ListOffsetsResponse, 1, 10),
            (Metadata, MetadataRequest, MetadataResponse, 0, 13),
            (OffsetCommit, OffsetCommitRequest, OffsetCommitResponse, 2, 10),
            (OffsetFetch, OffsetFetchRequest, OffsetFetchResponse, 1, 10),
            (FindCoordinator, FindCoordinatorRequest, FindCoordinatorResponse, 0, 6),
            (JoinGroup, JoinGroupRequest, JoinGroupResponse, 0, 9),
            (Heartbeat, HeartbeatRequest, HeartbeatResponse, 0, 4),
            (LeaveGroup, LeaveGroupRequest, LeaveGroupResponse, 0, 5),
            (SyncGroup, SyncGroupRequest, SyncGroupResponse, 0, 5),
            (DescribeGroups, DescribeGroupsRequest, DescribeGroupsResponse, 0, 6),
            (ListGroups, ListGroupsRequest, ListGroupsResponse, 0, 5),
            (SaslHandshake, SaslHandshakeRequest, SaslHandshakeResponse, 0, 1),
            (ApiVersions, ApiVersionsRequest, ApiVersionsResponse, 0, 4),
            (CreateTopics, CreateTopicsRequest, CreateTopicsResponse, 2, 7),
            (DeleteTopics, DeleteTopicsRequest, DeleteTopicsResponse, 1, 6),
            (DeleteRecords, DeleteRecordsRequest, DeleteRecordsResponse, 0, 2),
            (InitProducerId, InitProducerIdRequest, InitProducerIdResponse, 0, 6),
            (OffsetForLeaderEpoch, OffsetForLeaderEpochRequest, OffsetForLeaderEpochResponse, 2, 4),
            (AddPartitionsToTxn, AddPartitionsToTxnRequest, AddPartitionsToTxnResponse, 0, 5),
            (AddOffsetsToTxn, AddOffsetsToTxnRequest, AddOffsetsToTxnResponse, 0, 4),
            (EndTxn, EndTxnRequest, EndTxnResponse, 0, 5),
            (WriteTxnMarkers, WriteTxnMarkersRequest, WriteTxnMarkersResponse, 1, 1),
            (TxnOffsetCommit, TxnOffsetCommitRequest, TxnOffsetCommitResponse, 0, 5),
            (DescribeAcls, DescribeAclsRequest, DescribeAclsResponse, 1, 3),
            (CreateAcls, CreateAclsRequest, CreateAclsResponse, 1, 3),
            (DeleteAcls, DeleteAclsRequest, DeleteAclsResponse, 1, 3),
            (DescribeConfigs, DescribeConfigsRequest, DescribeConfigsResponse, 1, 4),
            (AlterConfigs, AlterConfigsRequest, AlterConfigsResponse, 0, 2),
            (AlterReplicaLogDirs, AlterReplicaLogDirsRequest, AlterReplicaLogDirsResponse, 1, 2),
            (DescribeLogDirs, DescribeLogDirsRequest, DescribeLogDirsResponse, 1, 4),
            (SaslAuthenticate, SaslAuthenticateRequest, SaslAuthenticateResponse, 0, 2),
            (CreatePartitions, CreatePartitionsRequest, CreatePartitionsResponse, 0, 3),
            (CreateDelegationToken, CreateDelegationTokenRequest, CreateDelegationTokenResponse, 1, 3),
            (RenewDelegationToken, RenewDelegationTokenRequest, RenewDelegationTokenResponse, 1, 2),
            (ExpireDelegationToken, ExpireDelegationTokenRequest, ExpireDelegationTokenResponse, 1, 2),
            (DescribeDelegationToken, DescribeDelegationTokenRequest, DescribeDelegationTokenResponse, 1, 3),
            (DeleteGroups, DeleteGroupsRequest, DeleteGroupsResponse, 0, 2),
            (ElectLeaders, ElectLeadersRequest, ElectLeadersResponse, 0, 2),
            (IncrementalAlterConfigs, IncrementalAlterConfigsRequest, IncrementalAlterConfigsResponse, 0, 1),
            (AlterPartitionReassignments, AlterPartitionReassignmentsRequest, AlterPartitionReassignmentsResponse, 0, 1),
            (ListPartitionReassignments, ListPartitionReassignmentsRequest, ListPartitionReassignmentsResponse, 0, 0),
            (OffsetDelete, OffsetDeleteRequest, OffsetDeleteResponse, 0, 0),
            (DescribeClientQuotas, DescribeClientQuotasRequest, DescribeClientQuotasResponse, 0, 1),
            (AlterClientQuotas, AlterClientQuotasRequest, AlterClientQuotasResponse, 0, 1),
            (DescribeUserScramCredentials, DescribeUserScramCredentialsRequest, DescribeUserScramCredentialsResponse, 0, 0),
            (AlterUserScramCredentials, AlterUserScramCredentialsRequest, AlterUserScramCredentialsResponse, 0, 0),
            (Vote, VoteRequest, VoteResponse, 0, 2),
            (BeginQuorumEpoch, BeginQuorumEpochRequest, BeginQuorumEpochResponse, 0, 1),
            (EndQuorumEpoch, EndQuorumEpochRequest, EndQuorumEpochResponse, 0, 1),
            (DescribeQuorum, DescribeQuorumRequest, DescribeQuorumResponse, 0, 2),
            (AlterPartition, AlterPartitionRequest, AlterPartitionResponse, 2, 3),
            (UpdateFeatures, UpdateFeaturesRequest, UpdateFeaturesResponse, 0, 2),
            (Envelope, EnvelopeRequest, EnvelopeResponse, 0, 0),
            (FetchSnapshot, FetchSnapshotRequest, FetchSnapshotResponse, 0, 1),
            (DescribeCluster, DescribeClusterRequest, DescribeClusterResponse, 0, 2),
            (DescribeProducers, DescribeProducersRequest, DescribeProducersResponse, 0, 0),
            (BrokerRegistration, BrokerRegistrationRequest, BrokerRegistrationResponse, 0, 4),
            (BrokerHeartbeat, BrokerHeartbeatRequest, BrokerHeartbeatResponse, 0, 1),
            (UnregisterBroker, UnregisterBrokerRequest, UnregisterBrokerResponse, 0, 0),
            (DescribeTransactions, DescribeTransactionsRequest, DescribeTransactionsResponse, 0, 0),
            (ListTransactions, ListTransactionsRequest, ListTransactionsResponse, 0, 2),
            (AllocateProducerIds, AllocateProducerIdsRequest, AllocateProducerIdsResponse, 0, 0),
            (ConsumerGroupHeartbeat, ConsumerGroupHeartbeatRequest, ConsumerGroupHeartbeatResponse, 0, 1),
            (ConsumerGroupDescribe, ConsumerGroupDescribeRequest, ConsumerGroupDescribeResponse, 0, 1),
            (ControllerRegistration, ControllerRegistrationRequest, ControllerRegistrationResponse, 0, 0),
            (GetTelemetrySubscriptions, GetTelemetrySubscriptionsRequest, GetTelemetrySubscriptionsResponse, 0, 0),
            (PushTelemetry, PushTelemetryRequest, PushTelemetryResponse, 0, 0),
            (AssignReplicasToDirs, AssignReplicasToDirsRequest, AssignReplicasToDirsResponse, 0, 0),
            (ListConfigResources, ListConfigResourcesRequest, ListConfigResourcesResponse, 0, 1),
            (DescribeTopicPartitions, DescribeTopicPartitionsRequest, DescribeTopicPartitionsResponse, 0, 0),
            (ShareGroupHeartbeat, ShareGroupHeartbeatRequest, ShareGroupHeartbeatResponse, 1, 1),
            (ShareGroupDescribe, ShareGroupDescribeRequest, ShareGroupDescribeResponse, 1, 1),
            (ShareFetch, ShareFetchRequest, ShareFetchResponse, 1, 1),
            (ShareAcknowledge, ShareAcknowledgeRequest, ShareAcknowledgeResponse, 1, 1),
            (AddRaftVoter, AddRaftVoterRequest, AddRaftVoterResponse, 0, 0),
            (RemoveRaftVoter, RemoveRaftVoterRequest, RemoveRaftVoterResponse, 0, 0),
            (UpdateRaftVoter, UpdateRaftVoterRequest, UpdateRaftVoterResponse, 0, 0),
            (InitializeShareGroupState, InitializeShareGroupStateRequest, InitializeShareGroupStateResponse, 0, 0),
            (ReadShareGroupState, ReadShareGroupStateRequest, ReadShareGroupStateResponse, 0, 0),
            (WriteShareGroupState, WriteShareGroupStateRequest, WriteShareGroupStateResponse, 0, 0),
            (DeleteShareGroupState, DeleteShareGroupStateRequest, DeleteShareGroupStateResponse, 0, 0),
            (ReadShareGroupStateSummary, ReadShareGroupStateSummaryRequest, ReadShareGroupStateSummaryResponse, 0, 0),
            (DescribeShareGroupOffsets, DescribeShareGroupOffsetsRequest, DescribeShareGroupOffsetsResponse, 0, 0),
            (AlterShareGroupOffsets, AlterShareGroupOffsetsRequest, AlterShareGroupOffsetsResponse, 0, 0),
            (DeleteShareGroupOffsets, DeleteShareGroupOffsetsRequest, DeleteShareGroupOffsetsResponse, 0, 0),
        }
    };
}

/// Valid API keys in the Kafka protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiKey {
    /// API key for request ProduceRequest
    Produce = 0,
    /// API key for request FetchRequest
    Fetch = 1,
    /// API key for request ListOffsetsRequest
    ListOffsets = 2,
    /// API key for request MetadataRequest
    Metadata = 3,
    /// API key for request OffsetCommitRequest
    OffsetCommit = 8,
    /// API key for request OffsetFetchRequest
    OffsetFetch = 9,
    /// API key for request FindCoordinatorRequest
    FindCoordinator = 10,
    /// API key for request JoinGroupRequest
    JoinGroup = 11,
    /// API key for request HeartbeatRequest
    Heartbeat = 12,
    /// API key for request LeaveGroupRequest
    LeaveGroup = 13,
    /// API key for request SyncGroupRequest
    SyncGroup = 14,
    /// API key for request DescribeGroupsRequest
    DescribeGroups = 15,
    /// API key for request ListGroupsRequest
    ListGroups = 16,
    /// API key for request SaslHandshakeRequest
    SaslHandshake = 17,
    /// API key for request ApiVersionsRequest
    ApiVersions = 18,
    /// API key for request CreateTopicsRequest
    CreateTopics = 19,
    /// API key for request DeleteTopicsRequest
    DeleteTopics = 20,
    /// API key for request DeleteRecordsRequest
    DeleteRecords = 21,
    /// API key for request InitProducerIdRequest
    InitProducerId = 22,
    /// API key for request OffsetForLeaderEpochRequest
    OffsetForLeaderEpoch = 23,
    /// API key for request AddPartitionsToTxnRequest
    AddPartitionsToTxn = 24,
    /// API key for request AddOffsetsToTxnRequest
    AddOffsetsToTxn = 25,
    /// API key for request EndTxnRequest
    EndTxn = 26,
    /// API key for request WriteTxnMarkersRequest
    WriteTxnMarkers = 27,
    /// API key for request TxnOffsetCommitRequest
    TxnOffsetCommit = 28,
    /// API key for request DescribeAclsRequest
    DescribeAcls = 29,
    /// API key for request CreateAclsRequest
    CreateAcls = 30,
    /// API key for request DeleteAclsRequest
    DeleteAcls = 31,
    /// API key for request DescribeConfigsRequest
    DescribeConfigs = 32,
    /// API key for request AlterConfigsRequest
    AlterConfigs = 33,
    /// API key for request AlterReplicaLogDirsRequest
    AlterReplicaLogDirs = 34,
    /// API key for request DescribeLogDirsRequest
    DescribeLogDirs = 35,
    /// API key for request SaslAuthenticateRequest
    SaslAuthenticate = 36,
    /// API key for request CreatePartitionsRequest
    CreatePartitions = 37,
    /// API key for request CreateDelegationTokenRequest
    CreateDelegationToken = 38,
    /// API key for request RenewDelegationTokenRequest
    RenewDelegationToken = 39,
    /// API key for request ExpireDelegationTokenRequest
    ExpireDelegationToken = 40,
    /// API key for request DescribeDelegationTokenRequest
    DescribeDelegationToken = 41,
    /// API key for request DeleteGroupsRequest
    DeleteGroups = 42,
    /// API key for request ElectLeadersRequest
    ElectLeaders = 43,
    /// API key for request IncrementalAlterConfigsRequest
    IncrementalAlterConfigs = 44,
    /// API key for request AlterPartitionReassignmentsRequest
    AlterPartitionReassignments = 45,
    /// API key for request ListPartitionReassignmentsRequest
    ListPartitionReassignments = 46,
    /// API key for request OffsetDeleteRequest
    OffsetDelete = 47,
    /// API key for request DescribeClientQuotasRequest
    DescribeClientQuotas = 48,
    /// API key for request AlterClientQuotasRequest
    AlterClientQuotas = 49,
    /// API key for request DescribeUserScramCredentialsRequest
    DescribeUserScramCredentials = 50,
    /// API key for request AlterUserScramCredentialsRequest
    AlterUserScramCredentials = 51,
    /// API key for request VoteRequest
    Vote = 52,
    /// API key for request BeginQuorumEpochRequest
    BeginQuorumEpoch = 53,
    /// API key for request EndQuorumEpochRequest
    EndQuorumEpoch = 54,
    /// API key for request DescribeQuorumRequest
    DescribeQuorum = 55,
    /// API key for request AlterPartitionRequest
    AlterPartition = 56,
    /// API key for request UpdateFeaturesRequest
    UpdateFeatures = 57,
    /// API key for request EnvelopeRequest
    Envelope = 58,
    /// API key for request FetchSnapshotRequest
    FetchSnapshot = 59,
    /// API key for request DescribeClusterRequest
    DescribeCluster = 60,
    /// API key for request DescribeProducersRequest
    DescribeProducers = 61,
    /// API key for request BrokerRegistrationRequest
    BrokerRegistration = 62,
    /// API key for request BrokerHeartbeatRequest
    BrokerHeartbeat = 63,
    /// API key for request UnregisterBrokerRequest
    UnregisterBroker = 64,
    /// API key for request DescribeTransactionsRequest
    DescribeTransactions = 65,
    /// API key for request ListTransactionsRequest
    ListTransactions = 66,
    /// API key for request AllocateProducerIdsRequest
    AllocateProducerIds = 67,
    /// API key for request ConsumerGroupHeartbeatRequest
    ConsumerGroupHeartbeat = 68,
    /// API key for request ConsumerGroupDescribeRequest
    ConsumerGroupDescribe = 69,
    /// API key for request ControllerRegistrationRequest
    ControllerRegistration = 70,
    /// API key for request GetTelemetrySubscriptionsRequest
    GetTelemetrySubscriptions = 71,
    /// API key for request PushTelemetryRequest
    PushTelemetry = 72,
    /// API key for request AssignReplicasToDirsRequest
    AssignReplicasToDirs = 73,
    /// API key for request ListConfigResourcesRequest
    ListConfigResources = 74,
    /// API key for request DescribeTopicPartitionsRequest
    DescribeTopicPartitions = 75,
    /// API key for request ShareGroupHeartbeatRequest
    ShareGroupHeartbeat = 76,
    /// API key for request ShareGroupDescribeRequest
    ShareGroupDescribe = 77,
    /// API key for request ShareFetchRequest
    ShareFetch = 78,
    /// API key for request ShareAcknowledgeRequest
    ShareAcknowledge = 79,
    /// API key for request AddRaftVoterRequest
    AddRaftVoter = 80,
    /// API key for request RemoveRaftVoterRequest
    RemoveRaftVoter = 81,
    /// API key for request UpdateRaftVoterRequest
    UpdateRaftVoter = 82,
    /// API key for request InitializeShareGroupStateRequest
    InitializeShareGroupState = 83,
    /// API key for request ReadShareGroupStateRequest
    ReadShareGroupState = 84,
    /// API key for request WriteShareGroupStateRequest
    WriteShareGroupState = 85,
    /// API key for request DeleteShareGroupStateRequest
    DeleteShareGroupState = 86,
    /// API key for request ReadShareGroupStateSummaryRequest
    ReadShareGroupStateSummary = 87,
    /// API key for request DescribeShareGroupOffsetsRequest
    DescribeShareGroupOffsets = 90,
    /// API key for request AlterShareGroupOffsetsRequest
    AlterShareGroupOffsets = 91,
    /// API key for request DeleteShareGroupOffsetsRequest
    DeleteShareGroupOffsets = 92,
}

impl ApiKey {
    /// Get the version of request header that needs to be prepended to this message
    pub fn request_header_version(&self, version: i16) -> i16 {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match self { $( ApiKey::$var => $req::header_version(version), )* }
            };
        }
        for_each_api!(__arm)
    }

    /// Get the version of response header that needs to be prepended to this message
    pub fn response_header_version(&self, version: i16) -> i16 {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match self { $( ApiKey::$var => $resp::header_version(version), )* }
            };
        }
        for_each_api!(__arm)
    }

    /// Returns the valid versions that can be used with this ApiKey
    pub fn valid_versions(&self) -> VersionRange {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match self { $( ApiKey::$var => VersionRange { min: $min, max: $max }, )* }
            };
        }
        for_each_api!(__arm)
    }

    /// Iterate through every ApiKey variant in the order of the internal code.
    pub fn iter() -> impl Iterator<Item = ApiKey> {
        (0..=92).filter_map(|i| ApiKey::try_from(i).ok())
    }
}

impl TryFrom<i16> for ApiKey {
    type Error = ();

    fn try_from(v: i16) -> std::result::Result<Self, Self::Error> {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match v {
                    $( x if x == ApiKey::$var as i16 => Ok(ApiKey::$var), )*
                    _ => Err(()),
                }
            };
        }
        for_each_api!(__arm)
    }
}

/// Wrapping enum for all requests in the Kafka protocol.
#[cfg(feature = "messages_enums")]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum RequestKind {
    /// ProduceRequest,
    Produce(ProduceRequest),
    /// FetchRequest,
    Fetch(FetchRequest),
    /// ListOffsetsRequest,
    ListOffsets(ListOffsetsRequest),
    /// MetadataRequest,
    Metadata(MetadataRequest),
    /// OffsetCommitRequest,
    OffsetCommit(OffsetCommitRequest),
    /// OffsetFetchRequest,
    OffsetFetch(OffsetFetchRequest),
    /// FindCoordinatorRequest,
    FindCoordinator(FindCoordinatorRequest),
    /// JoinGroupRequest,
    JoinGroup(JoinGroupRequest),
    /// HeartbeatRequest,
    Heartbeat(HeartbeatRequest),
    /// LeaveGroupRequest,
    LeaveGroup(LeaveGroupRequest),
    /// SyncGroupRequest,
    SyncGroup(SyncGroupRequest),
    /// DescribeGroupsRequest,
    DescribeGroups(DescribeGroupsRequest),
    /// ListGroupsRequest,
    ListGroups(ListGroupsRequest),
    /// SaslHandshakeRequest,
    SaslHandshake(SaslHandshakeRequest),
    /// ApiVersionsRequest,
    ApiVersions(ApiVersionsRequest),
    /// CreateTopicsRequest,
    CreateTopics(CreateTopicsRequest),
    /// DeleteTopicsRequest,
    DeleteTopics(DeleteTopicsRequest),
    /// DeleteRecordsRequest,
    DeleteRecords(DeleteRecordsRequest),
    /// InitProducerIdRequest,
    InitProducerId(InitProducerIdRequest),
    /// OffsetForLeaderEpochRequest,
    OffsetForLeaderEpoch(OffsetForLeaderEpochRequest),
    /// AddPartitionsToTxnRequest,
    AddPartitionsToTxn(AddPartitionsToTxnRequest),
    /// AddOffsetsToTxnRequest,
    AddOffsetsToTxn(AddOffsetsToTxnRequest),
    /// EndTxnRequest,
    EndTxn(EndTxnRequest),
    /// WriteTxnMarkersRequest,
    WriteTxnMarkers(WriteTxnMarkersRequest),
    /// TxnOffsetCommitRequest,
    TxnOffsetCommit(TxnOffsetCommitRequest),
    /// DescribeAclsRequest,
    DescribeAcls(DescribeAclsRequest),
    /// CreateAclsRequest,
    CreateAcls(CreateAclsRequest),
    /// DeleteAclsRequest,
    DeleteAcls(DeleteAclsRequest),
    /// DescribeConfigsRequest,
    DescribeConfigs(DescribeConfigsRequest),
    /// AlterConfigsRequest,
    AlterConfigs(AlterConfigsRequest),
    /// AlterReplicaLogDirsRequest,
    AlterReplicaLogDirs(AlterReplicaLogDirsRequest),
    /// DescribeLogDirsRequest,
    DescribeLogDirs(DescribeLogDirsRequest),
    /// SaslAuthenticateRequest,
    SaslAuthenticate(SaslAuthenticateRequest),
    /// CreatePartitionsRequest,
    CreatePartitions(CreatePartitionsRequest),
    /// CreateDelegationTokenRequest,
    CreateDelegationToken(CreateDelegationTokenRequest),
    /// RenewDelegationTokenRequest,
    RenewDelegationToken(RenewDelegationTokenRequest),
    /// ExpireDelegationTokenRequest,
    ExpireDelegationToken(ExpireDelegationTokenRequest),
    /// DescribeDelegationTokenRequest,
    DescribeDelegationToken(DescribeDelegationTokenRequest),
    /// DeleteGroupsRequest,
    DeleteGroups(DeleteGroupsRequest),
    /// ElectLeadersRequest,
    ElectLeaders(ElectLeadersRequest),
    /// IncrementalAlterConfigsRequest,
    IncrementalAlterConfigs(IncrementalAlterConfigsRequest),
    /// AlterPartitionReassignmentsRequest,
    AlterPartitionReassignments(AlterPartitionReassignmentsRequest),
    /// ListPartitionReassignmentsRequest,
    ListPartitionReassignments(ListPartitionReassignmentsRequest),
    /// OffsetDeleteRequest,
    OffsetDelete(OffsetDeleteRequest),
    /// DescribeClientQuotasRequest,
    DescribeClientQuotas(DescribeClientQuotasRequest),
    /// AlterClientQuotasRequest,
    AlterClientQuotas(AlterClientQuotasRequest),
    /// DescribeUserScramCredentialsRequest,
    DescribeUserScramCredentials(DescribeUserScramCredentialsRequest),
    /// AlterUserScramCredentialsRequest,
    AlterUserScramCredentials(AlterUserScramCredentialsRequest),
    /// VoteRequest,
    Vote(VoteRequest),
    /// BeginQuorumEpochRequest,
    BeginQuorumEpoch(BeginQuorumEpochRequest),
    /// EndQuorumEpochRequest,
    EndQuorumEpoch(EndQuorumEpochRequest),
    /// DescribeQuorumRequest,
    DescribeQuorum(DescribeQuorumRequest),
    /// AlterPartitionRequest,
    AlterPartition(AlterPartitionRequest),
    /// UpdateFeaturesRequest,
    UpdateFeatures(UpdateFeaturesRequest),
    /// EnvelopeRequest,
    Envelope(EnvelopeRequest),
    /// FetchSnapshotRequest,
    FetchSnapshot(FetchSnapshotRequest),
    /// DescribeClusterRequest,
    DescribeCluster(DescribeClusterRequest),
    /// DescribeProducersRequest,
    DescribeProducers(DescribeProducersRequest),
    /// BrokerRegistrationRequest,
    BrokerRegistration(BrokerRegistrationRequest),
    /// BrokerHeartbeatRequest,
    BrokerHeartbeat(BrokerHeartbeatRequest),
    /// UnregisterBrokerRequest,
    UnregisterBroker(UnregisterBrokerRequest),
    /// DescribeTransactionsRequest,
    DescribeTransactions(DescribeTransactionsRequest),
    /// ListTransactionsRequest,
    ListTransactions(ListTransactionsRequest),
    /// AllocateProducerIdsRequest,
    AllocateProducerIds(AllocateProducerIdsRequest),
    /// ConsumerGroupHeartbeatRequest,
    ConsumerGroupHeartbeat(ConsumerGroupHeartbeatRequest),
    /// ConsumerGroupDescribeRequest,
    ConsumerGroupDescribe(ConsumerGroupDescribeRequest),
    /// ControllerRegistrationRequest,
    ControllerRegistration(ControllerRegistrationRequest),
    /// GetTelemetrySubscriptionsRequest,
    GetTelemetrySubscriptions(GetTelemetrySubscriptionsRequest),
    /// PushTelemetryRequest,
    PushTelemetry(PushTelemetryRequest),
    /// AssignReplicasToDirsRequest,
    AssignReplicasToDirs(AssignReplicasToDirsRequest),
    /// ListConfigResourcesRequest,
    ListConfigResources(ListConfigResourcesRequest),
    /// DescribeTopicPartitionsRequest,
    DescribeTopicPartitions(DescribeTopicPartitionsRequest),
    /// ShareGroupHeartbeatRequest,
    ShareGroupHeartbeat(ShareGroupHeartbeatRequest),
    /// ShareGroupDescribeRequest,
    ShareGroupDescribe(ShareGroupDescribeRequest),
    /// ShareFetchRequest,
    ShareFetch(ShareFetchRequest),
    /// ShareAcknowledgeRequest,
    ShareAcknowledge(ShareAcknowledgeRequest),
    /// AddRaftVoterRequest,
    AddRaftVoter(AddRaftVoterRequest),
    /// RemoveRaftVoterRequest,
    RemoveRaftVoter(RemoveRaftVoterRequest),
    /// UpdateRaftVoterRequest,
    UpdateRaftVoter(UpdateRaftVoterRequest),
    /// InitializeShareGroupStateRequest,
    InitializeShareGroupState(InitializeShareGroupStateRequest),
    /// ReadShareGroupStateRequest,
    ReadShareGroupState(ReadShareGroupStateRequest),
    /// WriteShareGroupStateRequest,
    WriteShareGroupState(WriteShareGroupStateRequest),
    /// DeleteShareGroupStateRequest,
    DeleteShareGroupState(DeleteShareGroupStateRequest),
    /// ReadShareGroupStateSummaryRequest,
    ReadShareGroupStateSummary(ReadShareGroupStateSummaryRequest),
    /// DescribeShareGroupOffsetsRequest,
    DescribeShareGroupOffsets(DescribeShareGroupOffsetsRequest),
    /// AlterShareGroupOffsetsRequest,
    AlterShareGroupOffsets(AlterShareGroupOffsetsRequest),
    /// DeleteShareGroupOffsetsRequest,
    DeleteShareGroupOffsets(DeleteShareGroupOffsetsRequest),
}

#[cfg(feature = "messages_enums")]
impl RequestKind {
    /// Encode the message into the target buffer
    #[cfg(feature = "client")]
    pub fn encode(&self, bytes: &mut bytes::BytesMut, version: i16) -> crate::error::Result<()> {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match self { $( RequestKind::$var(x) => encode(x, bytes, version), )* }
            };
        }
        for_each_api!(__arm)
    }

    /// Decode the message from the provided buffer and version
    #[cfg(feature = "broker")]
    pub fn decode(
        api_key: ApiKey,
        bytes: &mut bytes::Bytes,
        version: i16,
    ) -> crate::error::Result<RequestKind> {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match api_key { $( ApiKey::$var => Ok(RequestKind::$var(decode(bytes, version)?)), )* }
            };
        }
        for_each_api!(__arm)
    }
}
#[cfg(feature = "messages_enums")]
impl From<ProduceRequest> for RequestKind {
    fn from(value: ProduceRequest) -> RequestKind {
        RequestKind::Produce(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<FetchRequest> for RequestKind {
    fn from(value: FetchRequest) -> RequestKind {
        RequestKind::Fetch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListOffsetsRequest> for RequestKind {
    fn from(value: ListOffsetsRequest) -> RequestKind {
        RequestKind::ListOffsets(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<MetadataRequest> for RequestKind {
    fn from(value: MetadataRequest) -> RequestKind {
        RequestKind::Metadata(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetCommitRequest> for RequestKind {
    fn from(value: OffsetCommitRequest) -> RequestKind {
        RequestKind::OffsetCommit(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetFetchRequest> for RequestKind {
    fn from(value: OffsetFetchRequest) -> RequestKind {
        RequestKind::OffsetFetch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<FindCoordinatorRequest> for RequestKind {
    fn from(value: FindCoordinatorRequest) -> RequestKind {
        RequestKind::FindCoordinator(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<JoinGroupRequest> for RequestKind {
    fn from(value: JoinGroupRequest) -> RequestKind {
        RequestKind::JoinGroup(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<HeartbeatRequest> for RequestKind {
    fn from(value: HeartbeatRequest) -> RequestKind {
        RequestKind::Heartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<LeaveGroupRequest> for RequestKind {
    fn from(value: LeaveGroupRequest) -> RequestKind {
        RequestKind::LeaveGroup(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<SyncGroupRequest> for RequestKind {
    fn from(value: SyncGroupRequest) -> RequestKind {
        RequestKind::SyncGroup(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeGroupsRequest> for RequestKind {
    fn from(value: DescribeGroupsRequest) -> RequestKind {
        RequestKind::DescribeGroups(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListGroupsRequest> for RequestKind {
    fn from(value: ListGroupsRequest) -> RequestKind {
        RequestKind::ListGroups(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<SaslHandshakeRequest> for RequestKind {
    fn from(value: SaslHandshakeRequest) -> RequestKind {
        RequestKind::SaslHandshake(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ApiVersionsRequest> for RequestKind {
    fn from(value: ApiVersionsRequest) -> RequestKind {
        RequestKind::ApiVersions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreateTopicsRequest> for RequestKind {
    fn from(value: CreateTopicsRequest) -> RequestKind {
        RequestKind::CreateTopics(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteTopicsRequest> for RequestKind {
    fn from(value: DeleteTopicsRequest) -> RequestKind {
        RequestKind::DeleteTopics(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteRecordsRequest> for RequestKind {
    fn from(value: DeleteRecordsRequest) -> RequestKind {
        RequestKind::DeleteRecords(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<InitProducerIdRequest> for RequestKind {
    fn from(value: InitProducerIdRequest) -> RequestKind {
        RequestKind::InitProducerId(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetForLeaderEpochRequest> for RequestKind {
    fn from(value: OffsetForLeaderEpochRequest) -> RequestKind {
        RequestKind::OffsetForLeaderEpoch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AddPartitionsToTxnRequest> for RequestKind {
    fn from(value: AddPartitionsToTxnRequest) -> RequestKind {
        RequestKind::AddPartitionsToTxn(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AddOffsetsToTxnRequest> for RequestKind {
    fn from(value: AddOffsetsToTxnRequest) -> RequestKind {
        RequestKind::AddOffsetsToTxn(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<EndTxnRequest> for RequestKind {
    fn from(value: EndTxnRequest) -> RequestKind {
        RequestKind::EndTxn(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<WriteTxnMarkersRequest> for RequestKind {
    fn from(value: WriteTxnMarkersRequest) -> RequestKind {
        RequestKind::WriteTxnMarkers(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<TxnOffsetCommitRequest> for RequestKind {
    fn from(value: TxnOffsetCommitRequest) -> RequestKind {
        RequestKind::TxnOffsetCommit(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeAclsRequest> for RequestKind {
    fn from(value: DescribeAclsRequest) -> RequestKind {
        RequestKind::DescribeAcls(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreateAclsRequest> for RequestKind {
    fn from(value: CreateAclsRequest) -> RequestKind {
        RequestKind::CreateAcls(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteAclsRequest> for RequestKind {
    fn from(value: DeleteAclsRequest) -> RequestKind {
        RequestKind::DeleteAcls(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeConfigsRequest> for RequestKind {
    fn from(value: DescribeConfigsRequest) -> RequestKind {
        RequestKind::DescribeConfigs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterConfigsRequest> for RequestKind {
    fn from(value: AlterConfigsRequest) -> RequestKind {
        RequestKind::AlterConfigs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterReplicaLogDirsRequest> for RequestKind {
    fn from(value: AlterReplicaLogDirsRequest) -> RequestKind {
        RequestKind::AlterReplicaLogDirs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeLogDirsRequest> for RequestKind {
    fn from(value: DescribeLogDirsRequest) -> RequestKind {
        RequestKind::DescribeLogDirs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<SaslAuthenticateRequest> for RequestKind {
    fn from(value: SaslAuthenticateRequest) -> RequestKind {
        RequestKind::SaslAuthenticate(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreatePartitionsRequest> for RequestKind {
    fn from(value: CreatePartitionsRequest) -> RequestKind {
        RequestKind::CreatePartitions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreateDelegationTokenRequest> for RequestKind {
    fn from(value: CreateDelegationTokenRequest) -> RequestKind {
        RequestKind::CreateDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<RenewDelegationTokenRequest> for RequestKind {
    fn from(value: RenewDelegationTokenRequest) -> RequestKind {
        RequestKind::RenewDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ExpireDelegationTokenRequest> for RequestKind {
    fn from(value: ExpireDelegationTokenRequest) -> RequestKind {
        RequestKind::ExpireDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeDelegationTokenRequest> for RequestKind {
    fn from(value: DescribeDelegationTokenRequest) -> RequestKind {
        RequestKind::DescribeDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteGroupsRequest> for RequestKind {
    fn from(value: DeleteGroupsRequest) -> RequestKind {
        RequestKind::DeleteGroups(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ElectLeadersRequest> for RequestKind {
    fn from(value: ElectLeadersRequest) -> RequestKind {
        RequestKind::ElectLeaders(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<IncrementalAlterConfigsRequest> for RequestKind {
    fn from(value: IncrementalAlterConfigsRequest) -> RequestKind {
        RequestKind::IncrementalAlterConfigs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterPartitionReassignmentsRequest> for RequestKind {
    fn from(value: AlterPartitionReassignmentsRequest) -> RequestKind {
        RequestKind::AlterPartitionReassignments(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListPartitionReassignmentsRequest> for RequestKind {
    fn from(value: ListPartitionReassignmentsRequest) -> RequestKind {
        RequestKind::ListPartitionReassignments(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetDeleteRequest> for RequestKind {
    fn from(value: OffsetDeleteRequest) -> RequestKind {
        RequestKind::OffsetDelete(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeClientQuotasRequest> for RequestKind {
    fn from(value: DescribeClientQuotasRequest) -> RequestKind {
        RequestKind::DescribeClientQuotas(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterClientQuotasRequest> for RequestKind {
    fn from(value: AlterClientQuotasRequest) -> RequestKind {
        RequestKind::AlterClientQuotas(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeUserScramCredentialsRequest> for RequestKind {
    fn from(value: DescribeUserScramCredentialsRequest) -> RequestKind {
        RequestKind::DescribeUserScramCredentials(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterUserScramCredentialsRequest> for RequestKind {
    fn from(value: AlterUserScramCredentialsRequest) -> RequestKind {
        RequestKind::AlterUserScramCredentials(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<VoteRequest> for RequestKind {
    fn from(value: VoteRequest) -> RequestKind {
        RequestKind::Vote(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<BeginQuorumEpochRequest> for RequestKind {
    fn from(value: BeginQuorumEpochRequest) -> RequestKind {
        RequestKind::BeginQuorumEpoch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<EndQuorumEpochRequest> for RequestKind {
    fn from(value: EndQuorumEpochRequest) -> RequestKind {
        RequestKind::EndQuorumEpoch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeQuorumRequest> for RequestKind {
    fn from(value: DescribeQuorumRequest) -> RequestKind {
        RequestKind::DescribeQuorum(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterPartitionRequest> for RequestKind {
    fn from(value: AlterPartitionRequest) -> RequestKind {
        RequestKind::AlterPartition(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<UpdateFeaturesRequest> for RequestKind {
    fn from(value: UpdateFeaturesRequest) -> RequestKind {
        RequestKind::UpdateFeatures(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<EnvelopeRequest> for RequestKind {
    fn from(value: EnvelopeRequest) -> RequestKind {
        RequestKind::Envelope(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<FetchSnapshotRequest> for RequestKind {
    fn from(value: FetchSnapshotRequest) -> RequestKind {
        RequestKind::FetchSnapshot(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeClusterRequest> for RequestKind {
    fn from(value: DescribeClusterRequest) -> RequestKind {
        RequestKind::DescribeCluster(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeProducersRequest> for RequestKind {
    fn from(value: DescribeProducersRequest) -> RequestKind {
        RequestKind::DescribeProducers(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<BrokerRegistrationRequest> for RequestKind {
    fn from(value: BrokerRegistrationRequest) -> RequestKind {
        RequestKind::BrokerRegistration(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<BrokerHeartbeatRequest> for RequestKind {
    fn from(value: BrokerHeartbeatRequest) -> RequestKind {
        RequestKind::BrokerHeartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<UnregisterBrokerRequest> for RequestKind {
    fn from(value: UnregisterBrokerRequest) -> RequestKind {
        RequestKind::UnregisterBroker(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeTransactionsRequest> for RequestKind {
    fn from(value: DescribeTransactionsRequest) -> RequestKind {
        RequestKind::DescribeTransactions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListTransactionsRequest> for RequestKind {
    fn from(value: ListTransactionsRequest) -> RequestKind {
        RequestKind::ListTransactions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AllocateProducerIdsRequest> for RequestKind {
    fn from(value: AllocateProducerIdsRequest) -> RequestKind {
        RequestKind::AllocateProducerIds(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ConsumerGroupHeartbeatRequest> for RequestKind {
    fn from(value: ConsumerGroupHeartbeatRequest) -> RequestKind {
        RequestKind::ConsumerGroupHeartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ConsumerGroupDescribeRequest> for RequestKind {
    fn from(value: ConsumerGroupDescribeRequest) -> RequestKind {
        RequestKind::ConsumerGroupDescribe(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ControllerRegistrationRequest> for RequestKind {
    fn from(value: ControllerRegistrationRequest) -> RequestKind {
        RequestKind::ControllerRegistration(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<GetTelemetrySubscriptionsRequest> for RequestKind {
    fn from(value: GetTelemetrySubscriptionsRequest) -> RequestKind {
        RequestKind::GetTelemetrySubscriptions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<PushTelemetryRequest> for RequestKind {
    fn from(value: PushTelemetryRequest) -> RequestKind {
        RequestKind::PushTelemetry(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AssignReplicasToDirsRequest> for RequestKind {
    fn from(value: AssignReplicasToDirsRequest) -> RequestKind {
        RequestKind::AssignReplicasToDirs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListConfigResourcesRequest> for RequestKind {
    fn from(value: ListConfigResourcesRequest) -> RequestKind {
        RequestKind::ListConfigResources(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeTopicPartitionsRequest> for RequestKind {
    fn from(value: DescribeTopicPartitionsRequest) -> RequestKind {
        RequestKind::DescribeTopicPartitions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareGroupHeartbeatRequest> for RequestKind {
    fn from(value: ShareGroupHeartbeatRequest) -> RequestKind {
        RequestKind::ShareGroupHeartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareGroupDescribeRequest> for RequestKind {
    fn from(value: ShareGroupDescribeRequest) -> RequestKind {
        RequestKind::ShareGroupDescribe(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareFetchRequest> for RequestKind {
    fn from(value: ShareFetchRequest) -> RequestKind {
        RequestKind::ShareFetch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareAcknowledgeRequest> for RequestKind {
    fn from(value: ShareAcknowledgeRequest) -> RequestKind {
        RequestKind::ShareAcknowledge(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AddRaftVoterRequest> for RequestKind {
    fn from(value: AddRaftVoterRequest) -> RequestKind {
        RequestKind::AddRaftVoter(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<RemoveRaftVoterRequest> for RequestKind {
    fn from(value: RemoveRaftVoterRequest) -> RequestKind {
        RequestKind::RemoveRaftVoter(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<UpdateRaftVoterRequest> for RequestKind {
    fn from(value: UpdateRaftVoterRequest) -> RequestKind {
        RequestKind::UpdateRaftVoter(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<InitializeShareGroupStateRequest> for RequestKind {
    fn from(value: InitializeShareGroupStateRequest) -> RequestKind {
        RequestKind::InitializeShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ReadShareGroupStateRequest> for RequestKind {
    fn from(value: ReadShareGroupStateRequest) -> RequestKind {
        RequestKind::ReadShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<WriteShareGroupStateRequest> for RequestKind {
    fn from(value: WriteShareGroupStateRequest) -> RequestKind {
        RequestKind::WriteShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteShareGroupStateRequest> for RequestKind {
    fn from(value: DeleteShareGroupStateRequest) -> RequestKind {
        RequestKind::DeleteShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ReadShareGroupStateSummaryRequest> for RequestKind {
    fn from(value: ReadShareGroupStateSummaryRequest) -> RequestKind {
        RequestKind::ReadShareGroupStateSummary(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeShareGroupOffsetsRequest> for RequestKind {
    fn from(value: DescribeShareGroupOffsetsRequest) -> RequestKind {
        RequestKind::DescribeShareGroupOffsets(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterShareGroupOffsetsRequest> for RequestKind {
    fn from(value: AlterShareGroupOffsetsRequest) -> RequestKind {
        RequestKind::AlterShareGroupOffsets(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteShareGroupOffsetsRequest> for RequestKind {
    fn from(value: DeleteShareGroupOffsetsRequest) -> RequestKind {
        RequestKind::DeleteShareGroupOffsets(value)
    }
}

#[cfg(feature = "messages_enums")]
#[cfg(any(feature = "client", feature = "broker"))]
fn decode<T: Decodable>(bytes: &mut bytes::Bytes, version: i16) -> Result<T> {
    T::decode(bytes, version)
}

#[cfg(feature = "messages_enums")]
#[cfg(any(feature = "client", feature = "broker"))]
fn encode<T: Encodable>(encodable: &T, bytes: &mut bytes::BytesMut, version: i16) -> Result<()> {
    encodable.encode(bytes, version)
}

#[cfg(feature = "messages_enums")]
#[cfg(feature = "broker")]
fn encode_into<T: Encodable, B: crate::protocol::buf::ByteBufMut>(
    encodable: &T,
    buf: &mut B,
    version: i16,
) -> Result<()> {
    encodable.encode(buf, version)
}

/// Wrapping enum for all responses in the Kafka protocol.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
#[cfg(feature = "messages_enums")]
pub enum ResponseKind {
    /// ProduceResponse,
    Produce(ProduceResponse),
    /// FetchResponse,
    Fetch(FetchResponse),
    /// ListOffsetsResponse,
    ListOffsets(ListOffsetsResponse),
    /// MetadataResponse,
    Metadata(MetadataResponse),
    /// OffsetCommitResponse,
    OffsetCommit(OffsetCommitResponse),
    /// OffsetFetchResponse,
    OffsetFetch(OffsetFetchResponse),
    /// FindCoordinatorResponse,
    FindCoordinator(FindCoordinatorResponse),
    /// JoinGroupResponse,
    JoinGroup(JoinGroupResponse),
    /// HeartbeatResponse,
    Heartbeat(HeartbeatResponse),
    /// LeaveGroupResponse,
    LeaveGroup(LeaveGroupResponse),
    /// SyncGroupResponse,
    SyncGroup(SyncGroupResponse),
    /// DescribeGroupsResponse,
    DescribeGroups(DescribeGroupsResponse),
    /// ListGroupsResponse,
    ListGroups(ListGroupsResponse),
    /// SaslHandshakeResponse,
    SaslHandshake(SaslHandshakeResponse),
    /// ApiVersionsResponse,
    ApiVersions(ApiVersionsResponse),
    /// CreateTopicsResponse,
    CreateTopics(CreateTopicsResponse),
    /// DeleteTopicsResponse,
    DeleteTopics(DeleteTopicsResponse),
    /// DeleteRecordsResponse,
    DeleteRecords(DeleteRecordsResponse),
    /// InitProducerIdResponse,
    InitProducerId(InitProducerIdResponse),
    /// OffsetForLeaderEpochResponse,
    OffsetForLeaderEpoch(OffsetForLeaderEpochResponse),
    /// AddPartitionsToTxnResponse,
    AddPartitionsToTxn(AddPartitionsToTxnResponse),
    /// AddOffsetsToTxnResponse,
    AddOffsetsToTxn(AddOffsetsToTxnResponse),
    /// EndTxnResponse,
    EndTxn(EndTxnResponse),
    /// WriteTxnMarkersResponse,
    WriteTxnMarkers(WriteTxnMarkersResponse),
    /// TxnOffsetCommitResponse,
    TxnOffsetCommit(TxnOffsetCommitResponse),
    /// DescribeAclsResponse,
    DescribeAcls(DescribeAclsResponse),
    /// CreateAclsResponse,
    CreateAcls(CreateAclsResponse),
    /// DeleteAclsResponse,
    DeleteAcls(DeleteAclsResponse),
    /// DescribeConfigsResponse,
    DescribeConfigs(DescribeConfigsResponse),
    /// AlterConfigsResponse,
    AlterConfigs(AlterConfigsResponse),
    /// AlterReplicaLogDirsResponse,
    AlterReplicaLogDirs(AlterReplicaLogDirsResponse),
    /// DescribeLogDirsResponse,
    DescribeLogDirs(DescribeLogDirsResponse),
    /// SaslAuthenticateResponse,
    SaslAuthenticate(SaslAuthenticateResponse),
    /// CreatePartitionsResponse,
    CreatePartitions(CreatePartitionsResponse),
    /// CreateDelegationTokenResponse,
    CreateDelegationToken(CreateDelegationTokenResponse),
    /// RenewDelegationTokenResponse,
    RenewDelegationToken(RenewDelegationTokenResponse),
    /// ExpireDelegationTokenResponse,
    ExpireDelegationToken(ExpireDelegationTokenResponse),
    /// DescribeDelegationTokenResponse,
    DescribeDelegationToken(DescribeDelegationTokenResponse),
    /// DeleteGroupsResponse,
    DeleteGroups(DeleteGroupsResponse),
    /// ElectLeadersResponse,
    ElectLeaders(ElectLeadersResponse),
    /// IncrementalAlterConfigsResponse,
    IncrementalAlterConfigs(IncrementalAlterConfigsResponse),
    /// AlterPartitionReassignmentsResponse,
    AlterPartitionReassignments(AlterPartitionReassignmentsResponse),
    /// ListPartitionReassignmentsResponse,
    ListPartitionReassignments(ListPartitionReassignmentsResponse),
    /// OffsetDeleteResponse,
    OffsetDelete(OffsetDeleteResponse),
    /// DescribeClientQuotasResponse,
    DescribeClientQuotas(DescribeClientQuotasResponse),
    /// AlterClientQuotasResponse,
    AlterClientQuotas(AlterClientQuotasResponse),
    /// DescribeUserScramCredentialsResponse,
    DescribeUserScramCredentials(DescribeUserScramCredentialsResponse),
    /// AlterUserScramCredentialsResponse,
    AlterUserScramCredentials(AlterUserScramCredentialsResponse),
    /// VoteResponse,
    Vote(VoteResponse),
    /// BeginQuorumEpochResponse,
    BeginQuorumEpoch(BeginQuorumEpochResponse),
    /// EndQuorumEpochResponse,
    EndQuorumEpoch(EndQuorumEpochResponse),
    /// DescribeQuorumResponse,
    DescribeQuorum(DescribeQuorumResponse),
    /// AlterPartitionResponse,
    AlterPartition(AlterPartitionResponse),
    /// UpdateFeaturesResponse,
    UpdateFeatures(UpdateFeaturesResponse),
    /// EnvelopeResponse,
    Envelope(EnvelopeResponse),
    /// FetchSnapshotResponse,
    FetchSnapshot(FetchSnapshotResponse),
    /// DescribeClusterResponse,
    DescribeCluster(DescribeClusterResponse),
    /// DescribeProducersResponse,
    DescribeProducers(DescribeProducersResponse),
    /// BrokerRegistrationResponse,
    BrokerRegistration(BrokerRegistrationResponse),
    /// BrokerHeartbeatResponse,
    BrokerHeartbeat(BrokerHeartbeatResponse),
    /// UnregisterBrokerResponse,
    UnregisterBroker(UnregisterBrokerResponse),
    /// DescribeTransactionsResponse,
    DescribeTransactions(DescribeTransactionsResponse),
    /// ListTransactionsResponse,
    ListTransactions(ListTransactionsResponse),
    /// AllocateProducerIdsResponse,
    AllocateProducerIds(AllocateProducerIdsResponse),
    /// ConsumerGroupHeartbeatResponse,
    ConsumerGroupHeartbeat(ConsumerGroupHeartbeatResponse),
    /// ConsumerGroupDescribeResponse,
    ConsumerGroupDescribe(ConsumerGroupDescribeResponse),
    /// ControllerRegistrationResponse,
    ControllerRegistration(ControllerRegistrationResponse),
    /// GetTelemetrySubscriptionsResponse,
    GetTelemetrySubscriptions(GetTelemetrySubscriptionsResponse),
    /// PushTelemetryResponse,
    PushTelemetry(PushTelemetryResponse),
    /// AssignReplicasToDirsResponse,
    AssignReplicasToDirs(AssignReplicasToDirsResponse),
    /// ListConfigResourcesResponse,
    ListConfigResources(ListConfigResourcesResponse),
    /// DescribeTopicPartitionsResponse,
    DescribeTopicPartitions(DescribeTopicPartitionsResponse),
    /// ShareGroupHeartbeatResponse,
    ShareGroupHeartbeat(ShareGroupHeartbeatResponse),
    /// ShareGroupDescribeResponse,
    ShareGroupDescribe(ShareGroupDescribeResponse),
    /// ShareFetchResponse,
    ShareFetch(ShareFetchResponse),
    /// ShareAcknowledgeResponse,
    ShareAcknowledge(ShareAcknowledgeResponse),
    /// AddRaftVoterResponse,
    AddRaftVoter(AddRaftVoterResponse),
    /// RemoveRaftVoterResponse,
    RemoveRaftVoter(RemoveRaftVoterResponse),
    /// UpdateRaftVoterResponse,
    UpdateRaftVoter(UpdateRaftVoterResponse),
    /// InitializeShareGroupStateResponse,
    InitializeShareGroupState(InitializeShareGroupStateResponse),
    /// ReadShareGroupStateResponse,
    ReadShareGroupState(ReadShareGroupStateResponse),
    /// WriteShareGroupStateResponse,
    WriteShareGroupState(WriteShareGroupStateResponse),
    /// DeleteShareGroupStateResponse,
    DeleteShareGroupState(DeleteShareGroupStateResponse),
    /// ReadShareGroupStateSummaryResponse,
    ReadShareGroupStateSummary(ReadShareGroupStateSummaryResponse),
    /// DescribeShareGroupOffsetsResponse,
    DescribeShareGroupOffsets(DescribeShareGroupOffsetsResponse),
    /// AlterShareGroupOffsetsResponse,
    AlterShareGroupOffsets(AlterShareGroupOffsetsResponse),
    /// DeleteShareGroupOffsetsResponse,
    DeleteShareGroupOffsets(DeleteShareGroupOffsetsResponse),
}

#[cfg(feature = "messages_enums")]
impl ResponseKind {
    /// Encode the message into the target buffer
    #[cfg(feature = "broker")]
    pub fn encode(&self, bytes: &mut bytes::BytesMut, version: i16) -> crate::error::Result<()> {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match self { $( ResponseKind::$var(x) => encode(x, bytes, version), )* }
            };
        }
        for_each_api!(__arm)
    }

    /// Decode the message from the provided buffer and version
    #[cfg(feature = "client")]
    pub fn decode(
        api_key: ApiKey,
        bytes: &mut bytes::Bytes,
        version: i16,
    ) -> crate::error::Result<ResponseKind> {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match api_key { $( ApiKey::$var => Ok(ResponseKind::$var(decode(bytes, version)?)), )* }
            };
        }
        for_each_api!(__arm)
    }

    /// Encode the message into a `ByteBufMut` (e.g. `SegmentedBuf` for zero-copy).
    #[cfg(feature = "broker")]
    pub fn encode_into<B: crate::protocol::buf::ByteBufMut>(
        &self,
        buf: &mut B,
        version: i16,
    ) -> crate::error::Result<()> {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match self { $( ResponseKind::$var(x) => encode_into(x, buf, version), )* }
            };
        }
        for_each_api!(__arm)
    }

    /// Get the version of request header that needs to be prepended to this message
    pub fn header_version(&self, version: i16) -> i16 {
        macro_rules! __arm {
            ($(($var:ident, $req:ident, $resp:ident, $min:literal, $max:literal)),* $(,)?) => {
                match self { $( ResponseKind::$var(_) => $resp::header_version(version), )* }
            };
        }
        for_each_api!(__arm)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ProduceResponse> for ResponseKind {
    fn from(value: ProduceResponse) -> ResponseKind {
        ResponseKind::Produce(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<FetchResponse> for ResponseKind {
    fn from(value: FetchResponse) -> ResponseKind {
        ResponseKind::Fetch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListOffsetsResponse> for ResponseKind {
    fn from(value: ListOffsetsResponse) -> ResponseKind {
        ResponseKind::ListOffsets(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<MetadataResponse> for ResponseKind {
    fn from(value: MetadataResponse) -> ResponseKind {
        ResponseKind::Metadata(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetCommitResponse> for ResponseKind {
    fn from(value: OffsetCommitResponse) -> ResponseKind {
        ResponseKind::OffsetCommit(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetFetchResponse> for ResponseKind {
    fn from(value: OffsetFetchResponse) -> ResponseKind {
        ResponseKind::OffsetFetch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<FindCoordinatorResponse> for ResponseKind {
    fn from(value: FindCoordinatorResponse) -> ResponseKind {
        ResponseKind::FindCoordinator(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<JoinGroupResponse> for ResponseKind {
    fn from(value: JoinGroupResponse) -> ResponseKind {
        ResponseKind::JoinGroup(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<HeartbeatResponse> for ResponseKind {
    fn from(value: HeartbeatResponse) -> ResponseKind {
        ResponseKind::Heartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<LeaveGroupResponse> for ResponseKind {
    fn from(value: LeaveGroupResponse) -> ResponseKind {
        ResponseKind::LeaveGroup(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<SyncGroupResponse> for ResponseKind {
    fn from(value: SyncGroupResponse) -> ResponseKind {
        ResponseKind::SyncGroup(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeGroupsResponse> for ResponseKind {
    fn from(value: DescribeGroupsResponse) -> ResponseKind {
        ResponseKind::DescribeGroups(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListGroupsResponse> for ResponseKind {
    fn from(value: ListGroupsResponse) -> ResponseKind {
        ResponseKind::ListGroups(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<SaslHandshakeResponse> for ResponseKind {
    fn from(value: SaslHandshakeResponse) -> ResponseKind {
        ResponseKind::SaslHandshake(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ApiVersionsResponse> for ResponseKind {
    fn from(value: ApiVersionsResponse) -> ResponseKind {
        ResponseKind::ApiVersions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreateTopicsResponse> for ResponseKind {
    fn from(value: CreateTopicsResponse) -> ResponseKind {
        ResponseKind::CreateTopics(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteTopicsResponse> for ResponseKind {
    fn from(value: DeleteTopicsResponse) -> ResponseKind {
        ResponseKind::DeleteTopics(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteRecordsResponse> for ResponseKind {
    fn from(value: DeleteRecordsResponse) -> ResponseKind {
        ResponseKind::DeleteRecords(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<InitProducerIdResponse> for ResponseKind {
    fn from(value: InitProducerIdResponse) -> ResponseKind {
        ResponseKind::InitProducerId(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetForLeaderEpochResponse> for ResponseKind {
    fn from(value: OffsetForLeaderEpochResponse) -> ResponseKind {
        ResponseKind::OffsetForLeaderEpoch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AddPartitionsToTxnResponse> for ResponseKind {
    fn from(value: AddPartitionsToTxnResponse) -> ResponseKind {
        ResponseKind::AddPartitionsToTxn(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AddOffsetsToTxnResponse> for ResponseKind {
    fn from(value: AddOffsetsToTxnResponse) -> ResponseKind {
        ResponseKind::AddOffsetsToTxn(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<EndTxnResponse> for ResponseKind {
    fn from(value: EndTxnResponse) -> ResponseKind {
        ResponseKind::EndTxn(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<WriteTxnMarkersResponse> for ResponseKind {
    fn from(value: WriteTxnMarkersResponse) -> ResponseKind {
        ResponseKind::WriteTxnMarkers(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<TxnOffsetCommitResponse> for ResponseKind {
    fn from(value: TxnOffsetCommitResponse) -> ResponseKind {
        ResponseKind::TxnOffsetCommit(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeAclsResponse> for ResponseKind {
    fn from(value: DescribeAclsResponse) -> ResponseKind {
        ResponseKind::DescribeAcls(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreateAclsResponse> for ResponseKind {
    fn from(value: CreateAclsResponse) -> ResponseKind {
        ResponseKind::CreateAcls(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteAclsResponse> for ResponseKind {
    fn from(value: DeleteAclsResponse) -> ResponseKind {
        ResponseKind::DeleteAcls(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeConfigsResponse> for ResponseKind {
    fn from(value: DescribeConfigsResponse) -> ResponseKind {
        ResponseKind::DescribeConfigs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterConfigsResponse> for ResponseKind {
    fn from(value: AlterConfigsResponse) -> ResponseKind {
        ResponseKind::AlterConfigs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterReplicaLogDirsResponse> for ResponseKind {
    fn from(value: AlterReplicaLogDirsResponse) -> ResponseKind {
        ResponseKind::AlterReplicaLogDirs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeLogDirsResponse> for ResponseKind {
    fn from(value: DescribeLogDirsResponse) -> ResponseKind {
        ResponseKind::DescribeLogDirs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<SaslAuthenticateResponse> for ResponseKind {
    fn from(value: SaslAuthenticateResponse) -> ResponseKind {
        ResponseKind::SaslAuthenticate(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreatePartitionsResponse> for ResponseKind {
    fn from(value: CreatePartitionsResponse) -> ResponseKind {
        ResponseKind::CreatePartitions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<CreateDelegationTokenResponse> for ResponseKind {
    fn from(value: CreateDelegationTokenResponse) -> ResponseKind {
        ResponseKind::CreateDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<RenewDelegationTokenResponse> for ResponseKind {
    fn from(value: RenewDelegationTokenResponse) -> ResponseKind {
        ResponseKind::RenewDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ExpireDelegationTokenResponse> for ResponseKind {
    fn from(value: ExpireDelegationTokenResponse) -> ResponseKind {
        ResponseKind::ExpireDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeDelegationTokenResponse> for ResponseKind {
    fn from(value: DescribeDelegationTokenResponse) -> ResponseKind {
        ResponseKind::DescribeDelegationToken(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteGroupsResponse> for ResponseKind {
    fn from(value: DeleteGroupsResponse) -> ResponseKind {
        ResponseKind::DeleteGroups(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ElectLeadersResponse> for ResponseKind {
    fn from(value: ElectLeadersResponse) -> ResponseKind {
        ResponseKind::ElectLeaders(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<IncrementalAlterConfigsResponse> for ResponseKind {
    fn from(value: IncrementalAlterConfigsResponse) -> ResponseKind {
        ResponseKind::IncrementalAlterConfigs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterPartitionReassignmentsResponse> for ResponseKind {
    fn from(value: AlterPartitionReassignmentsResponse) -> ResponseKind {
        ResponseKind::AlterPartitionReassignments(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListPartitionReassignmentsResponse> for ResponseKind {
    fn from(value: ListPartitionReassignmentsResponse) -> ResponseKind {
        ResponseKind::ListPartitionReassignments(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<OffsetDeleteResponse> for ResponseKind {
    fn from(value: OffsetDeleteResponse) -> ResponseKind {
        ResponseKind::OffsetDelete(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeClientQuotasResponse> for ResponseKind {
    fn from(value: DescribeClientQuotasResponse) -> ResponseKind {
        ResponseKind::DescribeClientQuotas(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterClientQuotasResponse> for ResponseKind {
    fn from(value: AlterClientQuotasResponse) -> ResponseKind {
        ResponseKind::AlterClientQuotas(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeUserScramCredentialsResponse> for ResponseKind {
    fn from(value: DescribeUserScramCredentialsResponse) -> ResponseKind {
        ResponseKind::DescribeUserScramCredentials(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterUserScramCredentialsResponse> for ResponseKind {
    fn from(value: AlterUserScramCredentialsResponse) -> ResponseKind {
        ResponseKind::AlterUserScramCredentials(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<VoteResponse> for ResponseKind {
    fn from(value: VoteResponse) -> ResponseKind {
        ResponseKind::Vote(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<BeginQuorumEpochResponse> for ResponseKind {
    fn from(value: BeginQuorumEpochResponse) -> ResponseKind {
        ResponseKind::BeginQuorumEpoch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<EndQuorumEpochResponse> for ResponseKind {
    fn from(value: EndQuorumEpochResponse) -> ResponseKind {
        ResponseKind::EndQuorumEpoch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeQuorumResponse> for ResponseKind {
    fn from(value: DescribeQuorumResponse) -> ResponseKind {
        ResponseKind::DescribeQuorum(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterPartitionResponse> for ResponseKind {
    fn from(value: AlterPartitionResponse) -> ResponseKind {
        ResponseKind::AlterPartition(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<UpdateFeaturesResponse> for ResponseKind {
    fn from(value: UpdateFeaturesResponse) -> ResponseKind {
        ResponseKind::UpdateFeatures(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<EnvelopeResponse> for ResponseKind {
    fn from(value: EnvelopeResponse) -> ResponseKind {
        ResponseKind::Envelope(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<FetchSnapshotResponse> for ResponseKind {
    fn from(value: FetchSnapshotResponse) -> ResponseKind {
        ResponseKind::FetchSnapshot(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeClusterResponse> for ResponseKind {
    fn from(value: DescribeClusterResponse) -> ResponseKind {
        ResponseKind::DescribeCluster(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeProducersResponse> for ResponseKind {
    fn from(value: DescribeProducersResponse) -> ResponseKind {
        ResponseKind::DescribeProducers(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<BrokerRegistrationResponse> for ResponseKind {
    fn from(value: BrokerRegistrationResponse) -> ResponseKind {
        ResponseKind::BrokerRegistration(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<BrokerHeartbeatResponse> for ResponseKind {
    fn from(value: BrokerHeartbeatResponse) -> ResponseKind {
        ResponseKind::BrokerHeartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<UnregisterBrokerResponse> for ResponseKind {
    fn from(value: UnregisterBrokerResponse) -> ResponseKind {
        ResponseKind::UnregisterBroker(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeTransactionsResponse> for ResponseKind {
    fn from(value: DescribeTransactionsResponse) -> ResponseKind {
        ResponseKind::DescribeTransactions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListTransactionsResponse> for ResponseKind {
    fn from(value: ListTransactionsResponse) -> ResponseKind {
        ResponseKind::ListTransactions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AllocateProducerIdsResponse> for ResponseKind {
    fn from(value: AllocateProducerIdsResponse) -> ResponseKind {
        ResponseKind::AllocateProducerIds(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ConsumerGroupHeartbeatResponse> for ResponseKind {
    fn from(value: ConsumerGroupHeartbeatResponse) -> ResponseKind {
        ResponseKind::ConsumerGroupHeartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ConsumerGroupDescribeResponse> for ResponseKind {
    fn from(value: ConsumerGroupDescribeResponse) -> ResponseKind {
        ResponseKind::ConsumerGroupDescribe(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ControllerRegistrationResponse> for ResponseKind {
    fn from(value: ControllerRegistrationResponse) -> ResponseKind {
        ResponseKind::ControllerRegistration(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<GetTelemetrySubscriptionsResponse> for ResponseKind {
    fn from(value: GetTelemetrySubscriptionsResponse) -> ResponseKind {
        ResponseKind::GetTelemetrySubscriptions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<PushTelemetryResponse> for ResponseKind {
    fn from(value: PushTelemetryResponse) -> ResponseKind {
        ResponseKind::PushTelemetry(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AssignReplicasToDirsResponse> for ResponseKind {
    fn from(value: AssignReplicasToDirsResponse) -> ResponseKind {
        ResponseKind::AssignReplicasToDirs(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ListConfigResourcesResponse> for ResponseKind {
    fn from(value: ListConfigResourcesResponse) -> ResponseKind {
        ResponseKind::ListConfigResources(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeTopicPartitionsResponse> for ResponseKind {
    fn from(value: DescribeTopicPartitionsResponse) -> ResponseKind {
        ResponseKind::DescribeTopicPartitions(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareGroupHeartbeatResponse> for ResponseKind {
    fn from(value: ShareGroupHeartbeatResponse) -> ResponseKind {
        ResponseKind::ShareGroupHeartbeat(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareGroupDescribeResponse> for ResponseKind {
    fn from(value: ShareGroupDescribeResponse) -> ResponseKind {
        ResponseKind::ShareGroupDescribe(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareFetchResponse> for ResponseKind {
    fn from(value: ShareFetchResponse) -> ResponseKind {
        ResponseKind::ShareFetch(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ShareAcknowledgeResponse> for ResponseKind {
    fn from(value: ShareAcknowledgeResponse) -> ResponseKind {
        ResponseKind::ShareAcknowledge(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AddRaftVoterResponse> for ResponseKind {
    fn from(value: AddRaftVoterResponse) -> ResponseKind {
        ResponseKind::AddRaftVoter(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<RemoveRaftVoterResponse> for ResponseKind {
    fn from(value: RemoveRaftVoterResponse) -> ResponseKind {
        ResponseKind::RemoveRaftVoter(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<UpdateRaftVoterResponse> for ResponseKind {
    fn from(value: UpdateRaftVoterResponse) -> ResponseKind {
        ResponseKind::UpdateRaftVoter(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<InitializeShareGroupStateResponse> for ResponseKind {
    fn from(value: InitializeShareGroupStateResponse) -> ResponseKind {
        ResponseKind::InitializeShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ReadShareGroupStateResponse> for ResponseKind {
    fn from(value: ReadShareGroupStateResponse) -> ResponseKind {
        ResponseKind::ReadShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<WriteShareGroupStateResponse> for ResponseKind {
    fn from(value: WriteShareGroupStateResponse) -> ResponseKind {
        ResponseKind::WriteShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteShareGroupStateResponse> for ResponseKind {
    fn from(value: DeleteShareGroupStateResponse) -> ResponseKind {
        ResponseKind::DeleteShareGroupState(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<ReadShareGroupStateSummaryResponse> for ResponseKind {
    fn from(value: ReadShareGroupStateSummaryResponse) -> ResponseKind {
        ResponseKind::ReadShareGroupStateSummary(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DescribeShareGroupOffsetsResponse> for ResponseKind {
    fn from(value: DescribeShareGroupOffsetsResponse) -> ResponseKind {
        ResponseKind::DescribeShareGroupOffsets(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<AlterShareGroupOffsetsResponse> for ResponseKind {
    fn from(value: AlterShareGroupOffsetsResponse) -> ResponseKind {
        ResponseKind::AlterShareGroupOffsets(value)
    }
}

#[cfg(feature = "messages_enums")]
impl From<DeleteShareGroupOffsetsResponse> for ResponseKind {
    fn from(value: DeleteShareGroupOffsetsResponse) -> ResponseKind {
        ResponseKind::DeleteShareGroupOffsets(value)
    }
}

/// The replica id of the current leader or -1 if the leader is unknown.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Copy)]
pub struct BrokerId(pub i32);

impl From<i32> for BrokerId {
    fn from(other: i32) -> Self {
        Self(other)
    }
}
impl From<BrokerId> for i32 {
    fn from(other: BrokerId) -> Self {
        other.0
    }
}
impl std::borrow::Borrow<i32> for BrokerId {
    fn borrow(&self) -> &i32 {
        &self.0
    }
}
impl std::ops::Deref for BrokerId {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::cmp::PartialEq<i32> for BrokerId {
    fn eq(&self, other: &i32) -> bool {
        &self.0 == other
    }
}
impl std::cmp::PartialEq<BrokerId> for i32 {
    fn eq(&self, other: &BrokerId) -> bool {
        self == &other.0
    }
}
impl std::fmt::Debug for BrokerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl NewType<i32> for BrokerId {}

/// The group identifier.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct GroupId(pub StrBytes);

impl From<StrBytes> for GroupId {
    fn from(other: StrBytes) -> Self {
        Self(other)
    }
}
impl From<GroupId> for StrBytes {
    fn from(other: GroupId) -> Self {
        other.0
    }
}
impl std::borrow::Borrow<StrBytes> for GroupId {
    fn borrow(&self) -> &StrBytes {
        &self.0
    }
}
impl std::ops::Deref for GroupId {
    type Target = StrBytes;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::cmp::PartialEq<StrBytes> for GroupId {
    fn eq(&self, other: &StrBytes) -> bool {
        &self.0 == other
    }
}
impl std::cmp::PartialEq<GroupId> for StrBytes {
    fn eq(&self, other: &GroupId) -> bool {
        self == &other.0
    }
}
impl std::fmt::Debug for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl NewType<StrBytes> for GroupId {}

/// The first producer ID in this range, inclusive.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Copy)]
pub struct ProducerId(pub i64);

impl From<i64> for ProducerId {
    fn from(other: i64) -> Self {
        Self(other)
    }
}
impl From<ProducerId> for i64 {
    fn from(other: ProducerId) -> Self {
        other.0
    }
}
impl std::borrow::Borrow<i64> for ProducerId {
    fn borrow(&self) -> &i64 {
        &self.0
    }
}
impl std::ops::Deref for ProducerId {
    type Target = i64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::cmp::PartialEq<i64> for ProducerId {
    fn eq(&self, other: &i64) -> bool {
        &self.0 == other
    }
}
impl std::cmp::PartialEq<ProducerId> for i64 {
    fn eq(&self, other: &ProducerId) -> bool {
        self == &other.0
    }
}
impl std::fmt::Debug for ProducerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl NewType<i64> for ProducerId {}

/// The topic name.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct TopicName(pub StrBytes);

impl From<StrBytes> for TopicName {
    fn from(other: StrBytes) -> Self {
        Self(other)
    }
}
impl From<TopicName> for StrBytes {
    fn from(other: TopicName) -> Self {
        other.0
    }
}
impl std::borrow::Borrow<StrBytes> for TopicName {
    fn borrow(&self) -> &StrBytes {
        &self.0
    }
}
impl std::ops::Deref for TopicName {
    type Target = StrBytes;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::cmp::PartialEq<StrBytes> for TopicName {
    fn eq(&self, other: &StrBytes) -> bool {
        &self.0 == other
    }
}
impl std::cmp::PartialEq<TopicName> for StrBytes {
    fn eq(&self, other: &TopicName) -> bool {
        self == &other.0
    }
}
impl std::fmt::Debug for TopicName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl NewType<StrBytes> for TopicName {}

/// The transactional id.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct TransactionalId(pub StrBytes);

impl From<StrBytes> for TransactionalId {
    fn from(other: StrBytes) -> Self {
        Self(other)
    }
}
impl From<TransactionalId> for StrBytes {
    fn from(other: TransactionalId) -> Self {
        other.0
    }
}
impl std::borrow::Borrow<StrBytes> for TransactionalId {
    fn borrow(&self) -> &StrBytes {
        &self.0
    }
}
impl std::ops::Deref for TransactionalId {
    type Target = StrBytes;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::cmp::PartialEq<StrBytes> for TransactionalId {
    fn eq(&self, other: &StrBytes) -> bool {
        &self.0 == other
    }
}
impl std::cmp::PartialEq<TransactionalId> for StrBytes {
    fn eq(&self, other: &TransactionalId) -> bool {
        self == &other.0
    }
}
impl std::fmt::Debug for TransactionalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl NewType<StrBytes> for TransactionalId {}
