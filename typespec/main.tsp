// Agent Pontifex wire contract -- TypeSpec authority.
//
// This file is an INDEPENDENT, human-authored authority. It is not generated
// from schemas/*.json and must never be overwritten by a generator. The JSON
// Schema tree under schemas/ is its co-equal peer. tools/check_peer_parity.py
// compares the two and fails closed on any unexplained divergence.
//
// Authority chain (see DEN-3959):
//   TypeSpec           -> IR_T -> SQL_T -> Protobuf/proto3 -> gRPC -> wire clients
//   JSON Schema/OpenAPI-> IR_J -> interfaces/types/validators -> SQL_J -> HTTP/write clients

namespace AgentPontifex;

/** Lowercase ASCII identifier, 1..128 chars, [a-z0-9._-]. */
@minLength(1) @maxLength(128) @pattern("^[a-z0-9._-]+$")
scalar Identifier extends string;

/** Identifier that must carry a namespace segment (contains a dot). */
@pattern("^[a-z0-9._-]*\\.[a-z0-9._-]*$")
scalar NamespacedIdentifier extends Identifier;

/** RFC 3339 timestamp. The wire type is always a string, never a number. */
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
  @maxItems(256) capabilities?: NamespacedIdentifier[];
  @maxProperties(64) extensions?: Record<unknown>;
}

model ErrorResponse {
  ok?: boolean;
  error: string;
  message?: string;
  details?: unknown;
}

namespace Bridge {
  enum AgentKind { claude, codex, gemini, kimi, qwen, human, other }
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
    id: string;
    repository: string;
    path: string;
    recursive: boolean;
    agent_key: Identifier;
    purpose?: string;
    meta?: unknown;
    fencing_token: uint64;
    acquired_at: Timestamp;
    expires_at: Timestamp;
  }

  model Message {
    id: string;
    channel: string;
    seq: uint64;
    from: string;
    role?: Role;
    content: string;
    meta?: unknown;
    created_at: Timestamp;
  }

  model Channel {
    slug: string;
    topic: string;
    topic_summary?: string | null;
    created_by: string;
    created_at: Timestamp;
    member_count: uint32;
    message_count: uint64;
    embedding_model: string;
    meta?: unknown;
  }

  model AcquireFileLeaseRequest {
    repository: string;
    @minItems(1) paths: string[];
    agent_key: Identifier;
    ttl_ms?: uint64;
    wait?: boolean = false;
  }

  model PostMessageRequest {
    from: string;
    content: string;
    role?: Role;
    meta?: unknown;
  }
}

namespace Coordinator {
  enum JobStatus { queued, running, succeeded, failed, cancelled }
  enum CompletionOutcome { succeeded, failed }

  model Job {
    id: string;
    org: string;
    repo: string;
    task_type: string;
    payload: unknown;
    priority: int64;
    status: JobStatus;
    created_at: Timestamp;
    updated_at: Timestamp;
    available_at: Timestamp;
    claimed_by: string | null;
    lease_expires_at: Timestamp | null;
    attempts: int64;
    max_attempts: int64;
    result: unknown | null;
    last_error: string | null;
    budget_usd: float64 | null;
  }

  model CreateJobRequest {
    org: string;
    repo: string;
    task_type: string;
    payload?: unknown;
    priority?: int64 = 0;
    max_attempts?: int64 = 3;
    available_at?: Timestamp;
    budget_usd?: float64;
  }

  model ClaimJobRequest {
    worker_id: string;
    orgs?: string[];
    repositories?: string[];
    task_types?: string[];
    lease_seconds?: int64 = 120;
  }

  model CompleteJobRequest {
    worker_id: string;
    outcome: CompletionOutcome;
    result?: unknown;
    error?: string;
    retryable?: boolean = false;
    retry_delay_seconds?: int64 = 30;
  }
}
