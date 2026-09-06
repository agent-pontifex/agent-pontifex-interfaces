// Agent Pontifex interfaces -- TypeScript projection.
//
// Types only. No transport, no persistence, no product behaviour. This file is a
// PROJECTION of the two authorities (schemas/*.json and typespec/main.tsp) and is
// covered by tools/check-peer-parity.rs; do not add fields here that neither
// authority declares.

export const PROTOCOL_SCHEMA_VERSION = 1 as const;
export const BRIDGE_PROTOCOL_ID = "agent-pontifex.bridge" as const;
export const COORDINATOR_PROTOCOL_ID = "agent-pontifex.coordinator" as const;
export const DISCOVERY_PATH = "/.well-known/agent-pontifex" as const;

/** RFC 3339 timestamp. Always a string on the wire, never a number. */
export type Timestamp = string;

export interface ProtocolVersionRange {
  min_major: number;
  max_major: number;
}

export type ServiceKind = "bridge" | "coordinator";
export type ProtocolId = typeof BRIDGE_PROTOCOL_ID | typeof COORDINATOR_PROTOCOL_ID;

export interface ServiceDescriptor {
  schema_version: 1;
  protocol: ProtocolId;
  protocol_versions: ProtocolVersionRange;
  service: ServiceKind;
  implementation: string;
  capabilities?: string[];
  extensions?: Record<string, unknown>;
}

export interface ErrorResponse {
  ok?: boolean;
  error: string;
  message?: string;
  details?: unknown;
}

export type AgentKind =
  | "chatgpt"
  | "claude"
  | "codex"
  | "grok"
  | "gemini"
  | "kimi"
  | "qwen"
  | "human"
  | "service"
  | "other";
export type Role = "user" | "assistant" | "system" | "tool";
export type MemberRole = "owner" | "member" | "observer";
export type PresenceKind = "joined" | "left";

export interface Agent {
  agent_key: string;
  display_name?: string;
  kind?: AgentKind;
  host?: string | null;
  meta?: unknown;
  registered_at?: Timestamp;
}

export interface FileLease {
  id: string;
  repository: string;
  path: string;
  recursive: boolean;
  agent_key: string;
  purpose?: string;
  meta?: unknown;
  /** Monotonic per lease line. Compare before any write; never ignore. */
  fencing_token: number;
  acquired_at: Timestamp;
  expires_at: Timestamp;
}

export interface Message {
  id: string;
  channel: string;
  seq: number;
  from: string;
  role?: Role;
  content: string;
  meta?: unknown;
  created_at: Timestamp;
}

export interface Channel {
  slug: string;
  topic: string;
  topic_summary?: string | null;
  created_by: string;
  created_at: Timestamp;
  member_count: number;
  message_count: number;
  embedding_model: string;
  meta?: unknown;
}

export type Event =
  | ({ type: "message" } & Message)
  | {
      type: "presence";
      channel: string;
      agent_key: string;
      event: PresenceKind;
      member_count: number;
      at: Timestamp;
    };

export interface AcquireFileLeaseRequest {
  repository: string;
  paths: string[];
  agent_key: string;
  ttl_ms?: number;
  wait?: boolean;
}

export interface PostMessageRequest {
  from: string;
  content: string;
  role?: Role;
  meta?: unknown;
}

export type MessageKind =
  | "command"
  | "event"
  | "observation"
  | "proposal"
  | "decision"
  | "handoff"
  | "heartbeat"
  | "ack"
  | "error"
  | "tool_call"
  | "tool_result"
  | "review_request"
  | "review_result";

export type DeliveryMode = "at_most_once" | "at_least_once";
export type AckStatus =
  | "accepted"
  | "rejected"
  | "duplicate"
  | "expired"
  | "unauthorized"
  | "stale_lease";

export interface AgentRef {
  agent_key: string;
  kind: AgentKind;
  instance_id: string;
  display_name?: string;
}

export interface TraceContext {
  /** Lowercase 16-byte W3C trace identifier, encoded as 32 hex characters. */
  trace_id: string;
  /** Lowercase 8-byte W3C span identifier, encoded as 16 hex characters. */
  span_id: string;
  trace_flags?: string;
  trace_state?: string;
}

export interface LeaseRef {
  lease_id: string;
  repository: string;
  path: string;
  fencing_token: number;
  expires_at: Timestamp;
}

export interface RealtimeEnvelope {
  schema_version: "agent-pontifex/realtime-envelope/v1";
  message_id: string;
  conversation_id: string;
  correlation_id?: string;
  causation_id?: string;
  sequence: number;
  sender: AgentRef;
  recipients: AgentRef[];
  kind: MessageKind;
  delivery: DeliveryMode;
  idempotency_key: string;
  trace?: TraceContext;
  lease?: LeaseRef;
  payload: Record<string, unknown>;
  created_at: Timestamp;
  expires_at?: Timestamp;
}

export interface Acknowledgement {
  schema_version: "agent-pontifex/acknowledgement/v1";
  acknowledgement_id: string;
  acknowledged_message_id: string;
  conversation_id: string;
  sender: AgentRef;
  status: AckStatus;
  reason_code?: string;
  observed_sequence: number;
  created_at: Timestamp;
}

export interface WorkHandoff {
  schema_version: "agent-pontifex/work-handoff/v1";
  handoff_id: string;
  conversation_id: string;
  from_agent: AgentRef;
  to_agent: AgentRef;
  objective: string;
  completed_work: string[];
  remaining_work: string[];
  evidence_uris: string[];
  lease?: LeaseRef;
  created_at: Timestamp;
}

export type JobStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";
export type CompletionOutcome = "succeeded" | "failed";

export interface Job {
  id: string;
  org: string;
  repo: string;
  task_type: string;
  payload: unknown;
  priority: number;
  status: JobStatus;
  created_at: Timestamp;
  updated_at: Timestamp;
  available_at: Timestamp;
  /** Required key, nullable value -- serialized even when absent. */
  claimed_by: string | null;
  lease_expires_at: Timestamp | null;
  attempts: number;
  max_attempts: number;
  result: unknown | null;
  last_error: string | null;
  budget_usd: number | null;
}

export interface CreateJobRequest {
  org: string;
  repo: string;
  task_type: string;
  payload?: unknown;
  priority?: number;
  max_attempts?: number;
  available_at?: Timestamp;
  budget_usd?: number;
}

export interface ClaimJobRequest {
  worker_id: string;
  orgs?: string[];
  repositories?: string[];
  task_types?: string[];
  lease_seconds?: number;
}

export interface CompleteJobRequest {
  worker_id: string;
  outcome: CompletionOutcome;
  result?: unknown;
  error?: string;
  retryable?: boolean;
  retry_delay_seconds?: number;
}

/** Highest mutually supported protocol major, or null when there is no overlap. */
export function highestCommonMajor(
  a: ProtocolVersionRange,
  b: ProtocolVersionRange,
): number | null {
  const lower = Math.max(a.min_major, b.min_major);
  const upper = Math.min(a.max_major, b.max_major);
  return lower <= upper ? upper : null;
}
