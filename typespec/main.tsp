// Agent Pontifex wire contract -- TypeSpec authority.
//
// This file is an INDEPENDENT, human-authored authority. It is not generated
// from schemas/*.json and must never be overwritten by a generator. The JSON
// Schema tree under schemas/ is its co-equal peer. tools/check-peer-parity.rs
// compares the two and fails closed on any unexplained divergence.
//
// Authority chain (see DEN-3959):
//   TypeSpec           -> IR_T -> SQL_T -> Protobuf/proto3 -> gRPC -> wire clients
//   JSON Schema/OpenAPI-> IR_J -> interfaces/types/validators -> SQL_J -> HTTP/write clients

import "@typespec/json-schema";

using JsonSchema;

namespace AgentPontifex;

/** Lowercase ASCII identifier, 1..128 chars, [a-z0-9._-]. */
@minLength(1) @maxLength(128) @pattern("^[a-z0-9._-]+$")
scalar Identifier extends string;

/** Identifier that must carry a namespace segment (contains a dot). */
@pattern("\\.")
scalar NamespacedIdentifier extends Identifier;

/** RFC 3339 timestamp. The wire type is always a string, never a number. */
@format("date-time")
scalar Timestamp extends string;

model ProtocolVersionRange {
  @minValue(1) @maxValue(65535) min_major: uint16;
  @minValue(1) @maxValue(65535) max_major: uint16;
}

enum ServiceKind { bridge, coordinator }

model ServiceDescriptor {
  schema_version: 1;
  protocol: "agent-pontifex.bridge" | "agent-pontifex.coordinator";
  protocol_versions: ProtocolVersionRange;
  service: ServiceKind;
  implementation: Identifier;
  @maxItems(256) @uniqueItems capabilities?: NamespacedIdentifier[];
  // Capped at 64 entries by the independent JSON Schema and runtime conformance gate.
  extensions?: Record<unknown>;
}

model ErrorResponse {
  ok?: boolean;
  error: string;
  message?: string;
  details?: unknown;
}

namespace Bridge {
  enum AgentKind { chatgpt, claude, codex, grok, gemini, kimi, qwen, human, service, other }
  enum Role { user, assistant, system, tool }
  enum MemberRole { owner, member, observer }
  enum PresenceKind { joined, left }

  model Agent {
    agent_key: Identifier;
    display_name?: string;
    kind?: AgentKind;
    host?: string | null;
    meta?: unknown;
    registered_at?: Timestamp;
  }

  model FileLease {
    @minLength(1) id: string;
    @minLength(1) repository: string;
    path: string;
    recursive: boolean;
    agent_key: Identifier;
    purpose?: string;
    meta?: unknown;
    @minValue(1) @maxValue(9007199254740991) fencing_token: uint64;
    acquired_at: Timestamp;
    expires_at: Timestamp;
  }

  model Message {
    @minLength(1) id: string;
    @minLength(1) channel: string;
    @minValue(0) @maxValue(9007199254740991) seq: uint64;
    @minLength(1) from: string;
    role?: Role;
    content: string;
    meta?: unknown;
    created_at: Timestamp;
  }

  model Channel {
    @minLength(1) slug: string;
    topic: string;
    topic_summary?: string | null;
    created_by: string;
    created_at: Timestamp;
    @minValue(0) @maxValue(4294967295) member_count: uint32;
    @minValue(0) @maxValue(9007199254740991) message_count: uint64;
    embedding_model: string;
    meta?: unknown;
  }

  model AcquireFileLeaseRequest {
    @minLength(1) repository: string;
    @minItems(1) paths: string[];
    agent_key: Identifier;
    @minValue(1) @maxValue(9007199254740991) ttl_ms?: uint64;
    wait?: boolean = false;
  }

  model PostMessageRequest {
    @minLength(1) from: string;
    content: string;
    role?: Role;
    meta?: unknown;
  }
}

namespace Coordinator {
  enum JobStatus { queued, running, succeeded, failed, cancelled }
  enum CompletionOutcome { succeeded, failed }

  model Job {
    @minLength(1) id: string;
    @minLength(1) org: string;
    @minLength(1) repo: string;
    @minLength(1) task_type: string;
    payload: unknown;
    priority: int64;
    status: JobStatus;
    created_at: Timestamp;
    updated_at: Timestamp;
    available_at: Timestamp;
    claimed_by: string | null;
    lease_expires_at: Timestamp | null;
    @minValue(0) attempts: int64;
    @minValue(1) max_attempts: int64;
    result: unknown | null;
    last_error: string | null;
    budget_usd: float64 | null;
  }

  model CreateJobRequest {
    @minLength(1) org: string;
    @minLength(1) repo: string;
    @minLength(1) task_type: string;
    payload?: unknown;
    priority?: int64 = 0;
    @minValue(1) max_attempts?: int64 = 3;
    available_at?: Timestamp;
    @minValue(0) budget_usd?: float64;
  }

  model ClaimJobRequest {
    @minLength(1) worker_id: string;
    orgs?: string[];
    repositories?: string[];
    task_types?: string[];
    @minValue(1) lease_seconds?: int64 = 120;
  }

  model CompleteJobRequest {
    @minLength(1) worker_id: string;
    outcome: CompletionOutcome;
    result?: unknown;
    error?: string;
    retryable?: boolean = false;
    @minValue(0) retry_delay_seconds?: int64 = 30;
  }
}

namespace Realtime {
  enum MessageKind {
    command,
    event,
    observation,
    proposal,
    decision,
    handoff,
    heartbeat,
    ack,
    error,
    tool_call,
    tool_result,
    review_request,
    review_result,
  }

  enum DeliveryMode { at_most_once, at_least_once }

  enum AckStatus {
    accepted,
    rejected,
    duplicate,
    expired,
    unauthorized,
    stale_lease,
  }

  model AgentRef {
    agent_key: Identifier;
    kind: Bridge.AgentKind;
    instance_id: Identifier;
    display_name?: string;
  }

  model TraceContext {
    @pattern("^[0-9a-f]{32}$") trace_id: string;
    @pattern("^[0-9a-f]{16}$") span_id: string;
    trace_flags?: string;
    trace_state?: string;
  }

  model LeaseRef {
    @minLength(1) lease_id: string;
    @minLength(1) repository: string;
    path: string;
    @minValue(1) @maxValue(9007199254740991) fencing_token: uint64;
    expires_at: Timestamp;
  }

  model RealtimeEnvelope {
    schema_version: "agent-pontifex/realtime-envelope/v1";
    @minLength(1) message_id: string;
    @minLength(1) conversation_id: string;
    @minLength(1) correlation_id?: string;
    @minLength(1) causation_id?: string;
    @minValue(0) @maxValue(9007199254740991) sequence: uint64;
    sender: AgentRef;
    @maxItems(64) recipients: AgentRef[];
    kind: MessageKind;
    delivery: DeliveryMode;
    @minLength(1) idempotency_key: string;
    trace?: TraceContext;
    lease?: LeaseRef;
    payload: Record<unknown>;
    created_at: Timestamp;
    expires_at?: Timestamp;
  }

  model Acknowledgement {
    schema_version: "agent-pontifex/acknowledgement/v1";
    @minLength(1) acknowledgement_id: string;
    @minLength(1) acknowledged_message_id: string;
    @minLength(1) conversation_id: string;
    sender: AgentRef;
    status: AckStatus;
    reason_code?: string;
    @minValue(0) @maxValue(9007199254740991) observed_sequence: uint64;
    created_at: Timestamp;
  }

  model WorkHandoff {
    schema_version: "agent-pontifex/work-handoff/v1";
    @minLength(1) handoff_id: string;
    @minLength(1) conversation_id: string;
    from_agent: AgentRef;
    to_agent: AgentRef;
    @minLength(1) objective: string;
    @maxItems(256) completed_work: string[];
    @maxItems(256) remaining_work: string[];
    @maxItems(256) evidence_uris: string[];
    lease?: LeaseRef;
    created_at: Timestamp;
  }
}
